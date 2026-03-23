use super::*;
use crate::log_parse::{log_parse_header, read_uvarint, CtxParsingState, ErrorLogParse};
use crate::log_rend::render_time;
use crate::log_rend_json::{
    render_json_string, render_json_string_content_ptr, render_json_string_ptr,
    render_safe_json_string,
};
use crate::log_transfomer_into_json_consts::{
    JSON_ERROR_CTX, JSON_ERROR_LOC, JSON_ERROR_TXT, JSON_LEVEL, JSON_LEVEL_DEBUG, JSON_LEVEL_ERROR,
    JSON_LEVEL_INFO, JSON_LEVEL_PANIC, JSON_LEVEL_TRACE, JSON_LEVEL_UNKNOWN, JSON_LEVEL_WARN,
    JSON_LOCATION, JSON_MESSAGE, JSON_STACKTRACE, JSON_TIME,
};
use crate::transform_items::{
    TransformBytes, TransformDuration, TransformLiteral, TransformString, TransformTime,
};
use std::io::Read;
use std::slice;

/// Transforms log record into pure JSON.
pub struct LogTransfomer {
    pub(crate) itoa:       itoa::Buffer,
    pub(crate) ryu:        ryu::Buffer,
    pub(crate) buf:        Vec<u8>,
    pub(crate) err_frags:  Vec<(usize, usize)>,
    pub(crate) grp_caps:   Vec<(usize, usize)>,
    pub(crate) err_caps:   Vec<usize>,
    pub(crate) prs_states: Vec<CtxParsingState>,

    pub(crate) max_log_size: usize,
    pub(crate) format_time:  bool,
}

impl LogTransfomer {
    pub fn new() -> Self {
        Self {
            itoa:       itoa::Buffer::new(),
            ryu:        ryu::Buffer::new(),
            buf:        Vec::with_capacity(4096),
            err_frags:  Vec::with_capacity(16),
            grp_caps:   Vec::with_capacity(16),
            err_caps:   Vec::with_capacity(16),
            prs_states: Vec::with_capacity(16),

            max_log_size: 1 * 1024 * 1024,
            format_time:  false,
        }
    }

    // Transforms a record in the given src into JSON
    pub(crate) unsafe fn transform_json<'a>(
        &mut self,
        dst: &mut Vec<u8>,
        src: &'a [u8],
    ) -> Result<&'a [u8], ErrorLogParse> {
        unsafe {
            let (record, rest) = match log_parse_header(src, self.max_log_size) {
                Ok((record, rest)) => (record, rest),
                Err(e) => return Err(e),
            };

            let ptr = record.as_ptr();
            let version = u16::from_le(ptr.cast::<u16>().read_unaligned());
            match version {
                1 => {
                    self.transform_json_v1(dst, ptr.add(2), record.len() - 2);
                }
                _ => {
                    return Err(log_parse::ErrorLogParse::RecordVersionNotSupported(version));
                }
            }

            Ok(rest)
        }
    }

    #[inline(always)]
    unsafe fn transform_json_v1(&mut self, dst: &mut Vec<u8>, ptr: *const u8, cap: usize) {
        unsafe {
            self.err_frags.clear();
            self.grp_caps.clear();
            self.err_caps.clear();

            dst.push(b'{');

            // Time.
            render_safe_json_string(dst, JSON_TIME);
            dst.push(b':');
            let time = ptr.cast::<u64>().read_unaligned();
            if self.format_time {
                dst.push(b'"');
                render_time(&mut self.itoa, dst, time as i64);
                dst.push(b'"');
            } else {
                let s = self.itoa.format(time);
                dst.extend_from_slice(s.as_bytes());
            }
            dst.push(b',');

            // Level.
            render_safe_json_string(dst, JSON_LEVEL);
            dst.push(b':');
            let lvl = *ptr.add(8);
            let mut is_panic = false;
            match lvl {
                level::TRACE => {
                    render_safe_json_string(dst, JSON_LEVEL_TRACE);
                }
                level::DEBUG => {
                    render_safe_json_string(dst, JSON_LEVEL_DEBUG);
                }
                level::INFO => {
                    render_safe_json_string(dst, JSON_LEVEL_INFO);
                }
                level::WARN => {
                    render_safe_json_string(dst, JSON_LEVEL_WARN);
                }
                level::ERROR => {
                    render_safe_json_string(dst, JSON_LEVEL_ERROR);
                }
                level::PANIC => {
                    is_panic = true;
                    render_safe_json_string(dst, JSON_LEVEL_PANIC);
                }
                _ => {
                    dst.push(b'"');
                    dst.extend_from_slice(JSON_LEVEL_UNKNOWN);
                    dst.push(b'(');
                    let s = self.itoa.format(lvl);
                    dst.extend_from_slice(s.as_bytes());
                    dst.push(b')');
                    dst.push(b'"');
                }
            }
            dst.push(b',');

            // May be location.
            let off: usize = if *ptr.add(9) != 0 {
                render_safe_json_string(dst, JSON_LOCATION);
                dst.push(b':');
                dst.push(b'"');
                let (length, size) = read_uvarint(ptr.add(9));
                render_json_string_content_ptr(dst, ptr.add(9 + size), length as usize);
                let off = 9 + size + length as usize;
                let (line, size) = read_uvarint(ptr.add(off));
                let s = self.itoa.format(line);
                dst.extend_from_slice(s.as_bytes());
                dst.push(b'"');
                dst.push(b',');

                off + size
            } else {
                10
            };

            // Message
            let (length, size) = read_uvarint(ptr.add(off));
            if !is_panic {
                render_safe_json_string(dst, JSON_MESSAGE);
                dst.push(b':');
                render_json_string_ptr(dst, ptr.add(off + size), length as usize);
            } else {
                render_safe_json_string(dst, JSON_STACKTRACE);
                dst.push(b':');

                // Gunzip stacktrace
                self.buf.clear();
                let st = slice::from_raw_parts(ptr.add(off + size), length as usize);
                let mut decoder = flate2::read::GzDecoder::new(st);
                match decoder.read_to_end(&mut self.buf) {
                    Ok(_) => {}
                    Err(x) => {
                        self.buf.extend_from_slice(x.to_string().as_bytes());
                    }
                };

                render_json_string(dst, self.buf.as_slice());
            }

            // Context.
            self.transform_json_ctx_v1(dst, ptr, off + size + length as usize, cap);
            dst.push(b'}');
        }
    }

    unsafe fn transform_json_ctx_v1(
        &mut self,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        mut off: usize,
        mut cap: usize,
    ) {
        unsafe {
            let mut old = true;
            let mut on_error_stage = false;
            let mut on_embed_error = false;
            let mut group_off: usize = 0;
            let mut parsing_state = CtxParsingState::Normal;
            let mut group_cap: usize = 0;

            loop {
                group_off += 1;

                match parsing_state {
                    CtxParsingState::Normal => {
                        if off >= cap {
                            return;
                        }
                    }

                    CtxParsingState::Group => {
                        if group_off > group_cap {
                            old = true;
                            dst.push(b'}');
                            if self.prs_states.is_empty() {
                                return;
                            }

                            parsing_state = self.prs_states.pop().unwrap();
                            if parsing_state == CtxParsingState::Group {
                                (group_off, group_cap) = self.grp_caps.pop().unwrap();
                            }
                            continue;
                        }
                    }

                    CtxParsingState::Error => {
                        if off >= cap {
                            dst.push(b'}');
                            dst.push(b'}');
                            dst.push(b',');
                            render_safe_json_string(dst, JSON_ERROR_TXT);
                            dst.push(b':');
                            dst.push(b'"');
                            for (i, (length, off)) in self.err_frags.iter().rev().enumerate() {
                                if i > 0 {
                                    dst.push(b':');
                                    dst.push(b' ');
                                }
                                render_json_string_content_ptr(dst, ptr.add(*off), *length);
                            }
                            dst.push(b'"');
                            dst.push(b'}');
                            old = true;

                            cap = self.err_caps.pop().unwrap();
                            parsing_state = self.prs_states.pop().unwrap();
                            continue;
                        }
                    }

                    CtxParsingState::ErrorEmbed => {
                        if off >= cap {
                            dst.push(b'}');
                            dst.push(b'}');
                            dst.push(b'}');
                            old = true;
                            cap = self.err_caps.pop().unwrap();
                            parsing_state = self.prs_states.pop().unwrap();
                            continue;
                        }
                    }
                }

                // Get and check kind.
                let kind = ptr.add(off).cast::<u8>().read_unaligned() as value_kind::ValueKind;
                off += 1;
                match kind {
                    value_kind::NEW_NODE => {
                        if on_error_stage {
                            dst.push(b'}');
                            dst.push(b',');
                        } else if old {
                            dst.push(b',');
                        }
                        on_error_stage = true;
                        dst.extend_from_slice(b"\"NEW: ");
                        let (length, size) = read_uvarint(ptr.add(off));
                        render_json_string_content_ptr(dst, ptr.add(off + size), length as usize);
                        if !on_embed_error {
                            self.err_frags.push((length as usize, off + size));
                        }
                        off += size + length as usize;
                        dst.extend_from_slice(b"\":{");
                        old = false;
                        continue;
                    }
                    value_kind::WRAP_NODE | value_kind::WRAP_INHERITED_NODE => {
                        if on_error_stage {
                            dst.push(b'}');
                            dst.push(b',');
                        } else if old {
                            dst.push(b',');
                        }
                        on_error_stage = true;
                        dst.extend_from_slice(b"\"WRAP: ");
                        let (length, size) = read_uvarint(ptr.add(off));
                        render_json_string_content_ptr(dst, ptr.add(off + size), length as usize);
                        if !on_embed_error {
                            self.err_frags.push((length as usize, off + size));
                        }
                        off += size + length as usize;
                        dst.extend_from_slice(b"\":{");
                        old = false;
                        continue;
                    }
                    value_kind::JUST_CONTEXT_NODE | value_kind::JUST_CONTEXT_INHERITED_NODE => {
                        old = true;
                        if on_error_stage {
                            dst.push(b'}');
                            dst.push(b',');
                        } else if old {
                            dst.push(b',');
                        }
                        dst.extend_from_slice(b"\"CTX\":{");
                        old = false;
                        continue;
                    }
                    value_kind::LOCATION_NODE => {
                        if old {
                            dst.push(b',');
                        }
                        old = true;
                        render_safe_json_string(dst, JSON_ERROR_LOC);
                        dst.push(b':');
                        let (length, size) = read_uvarint(ptr.add(off));
                        dst.push(b'"');
                        render_json_string_content_ptr(dst, ptr.add(off + size), length as usize);
                        dst.push(b':');
                        off += size + length as usize;
                        let (line, line_size) = read_uvarint(ptr.add(off));
                        off += line_size;
                        let s = self.itoa.format(line);
                        dst.extend_from_slice(s.as_bytes());
                        dst.push(b'"');
                        continue;
                    }
                    value_kind::FOREIGN_ERROR_TEXT => {
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        self.err_frags.push((lenght as usize, off + size));
                        off += size + lenght as usize;
                        continue;
                    }
                    value_kind::PHANTOM_CONTEXT_NODE => {
                        if on_error_stage {
                            dst.push(b'}');
                            old = true;
                        }
                        continue;
                    }
                    _ => {}
                }

                // Now, the key. Whatever it actually is. Only those above doesn't have it in their ways.
                let (length, size) = read_uvarint(ptr.add(off));
                if old {
                    dst.push(b',');
                }
                old = true;
                render_json_string_ptr(dst, ptr.add(off + size), length as usize);
                off += size + length as usize;
                dst.push(b':');
                match kind {
                    value_kind::NEW_NODE
                    | value_kind::WRAP_NODE
                    | value_kind::WRAP_INHERITED_NODE
                    | value_kind::JUST_CONTEXT_NODE
                    | value_kind::JUST_CONTEXT_INHERITED_NODE
                    | value_kind::LOCATION_NODE
                    | value_kind::FOREIGN_ERROR_TEXT
                    | value_kind::FOREIGN_ERROR_FORMAT => {
                        // Already handled. We list all possible nodes to have a proper debug further.
                    }

                    value_kind::BOOL => {
                        off = self.render_json::<bool>(dst, ptr, off);
                    }

                    value_kind::TIME => {
                        off = self.render_json::<TransformTime>(dst, ptr, off);
                    }

                    value_kind::DURATION => {
                        off = self.render_json::<TransformDuration>(dst, ptr, off);
                    }

                    value_kind::I | value_kind::I64 => {
                        off = self.render_json::<i64>(dst, ptr, off);
                    }

                    value_kind::I8 => {
                        off = self.render_json::<i8>(dst, ptr, off);
                    }

                    value_kind::I16 => {
                        off = self.render_json::<i16>(dst, ptr, off);
                    }

                    value_kind::I32 => {
                        off = self.render_json::<i32>(dst, ptr, off);
                    }

                    value_kind::U | value_kind::U64 => {
                        off = self.render_json::<u64>(dst, ptr, off);
                    }

                    value_kind::U8 => {
                        off = self.render_json::<u8>(dst, ptr, off);
                    }

                    value_kind::U16 => {
                        off = self.render_json::<u16>(dst, ptr, off);
                    }

                    value_kind::U32 => {
                        off = self.render_json::<u32>(dst, ptr, off);
                    }

                    value_kind::FLOAT32 => {
                        off = self.render_json::<f32>(dst, ptr, off);
                    }

                    value_kind::FLOAT64 => {
                        off = self.render_json::<f64>(dst, ptr, off);
                    }

                    value_kind::STRING | value_kind::ERROR_RAW => {
                        off = self.render_json::<TransformString>(dst, ptr, off);
                    }

                    value_kind::BYTES => {
                        off = self.render_json::<TransformBytes>(dst, ptr, off);
                    }

                    value_kind::SLICE_BOOL => {
                        off = self.render_slice_json::<bool>(dst, ptr, off);
                    }

                    value_kind::SLICE_I | value_kind::SLICE_I64 => {
                        off = self.render_slice_json::<i64>(dst, ptr, off);
                    }

                    value_kind::SLICE_I8 => {
                        off = self.render_slice_json::<i8>(dst, ptr, off);
                    }

                    value_kind::SLICE_I16 => {
                        off = self.render_slice_json::<i16>(dst, ptr, off);
                    }

                    value_kind::SLICE_I32 => {
                        off = self.render_slice_json::<i32>(dst, ptr, off);
                    }

                    value_kind::SLICE_U | value_kind::SLICE_U64 => {
                        off = self.render_slice_json::<u64>(dst, ptr, off);
                    }

                    value_kind::SLICE_U8 => {
                        off = self.render_slice_json::<u8>(dst, ptr, off);
                    }

                    value_kind::SLICE_U16 => {
                        off = self.render_slice_json::<u16>(dst, ptr, off);
                    }

                    value_kind::SLICE_U32 => {
                        off = self.render_slice_json::<u32>(dst, ptr, off);
                    }

                    value_kind::SLICE_F32 => {
                        off = self.render_slice_json::<f32>(dst, ptr, off);
                    }

                    value_kind::SLICE_F64 => {
                        off = self.render_slice_json::<f64>(dst, ptr, off);
                    }

                    value_kind::SLICE_STRING => {
                        off = self.render_slice_json::<TransformString>(dst, ptr, off);
                    }

                    value_kind::ERROR => {
                        on_embed_error = false;
                        on_error_stage = false;
                        self.prs_states.push(parsing_state);
                        self.err_caps.push(cap);
                        let (length, size) = read_uvarint(ptr.add(off));
                        dst.push(b'{');
                        render_safe_json_string(dst, JSON_ERROR_CTX);
                        dst.push(b':');
                        dst.push(b'{');
                        cap = off + size + length as usize;
                        off += size;
                        parsing_state = CtxParsingState::Error;
                        old = false;
                    }

                    value_kind::ERROR_EMBED => {
                        on_embed_error = true;
                        on_error_stage = false;
                        self.prs_states.push(parsing_state);
                        self.err_caps.push(cap);
                        let (length, size) = read_uvarint(ptr.add(off));
                        dst.push(b'{');
                        render_safe_json_string(dst, JSON_ERROR_TXT);
                        dst.push(b':');
                        render_json_string_ptr(dst, ptr.add(off + size), length as usize);
                        off += size + length as usize;
                        dst.push(b',');
                        render_safe_json_string(dst, JSON_ERROR_CTX);
                        dst.push(b':');
                        dst.push(b'{');

                        let (length, size) = read_uvarint(ptr.add(off));
                        cap = off + size + length as usize;
                        off += size;
                        parsing_state = CtxParsingState::ErrorEmbed;
                        old = false
                    }

                    value_kind::GROUP => {
                        group_off += 1;
                        dst.push(b'{');
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size;
                        if length == 0 {
                            dst.push(b'}');
                            old = true;
                            continue;
                        }
                        self.prs_states.push(parsing_state);
                        if parsing_state == CtxParsingState::Group {
                            self.grp_caps.push((group_off, group_cap));
                        }
                        group_off = 0;
                        group_cap = length as usize;
                        parsing_state = CtxParsingState::Group;
                        old = false;
                        continue;
                    }

                    _ => {
                        return;
                        panic!("unsupported value kind {}", value_kind::string(kind));
                    }
                }
            }
        }
    }

    #[inline(always)]
    unsafe fn render_json<T>(&mut self, dst: &mut Vec<u8>, ptr: *const u8, off: usize) -> usize
    where
        T: TransformLiteral,
    {
        unsafe { T::render(self, dst, ptr, off) }
    }

    #[inline(always)]
    unsafe fn render_slice_json<T>(
        &mut self,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        mut off: usize,
    ) -> usize
    where
        T: TransformLiteral,
    {
        unsafe {
            let (length, size) = read_uvarint(ptr.add(off));
            off += size;
            dst.push(b'[');
            if length == 0 {
                dst.push(b']');
                return off;
            }

            for i in 0..length {
                if i != 0 {
                    dst.push(b',');
                }

                off = T::render(self, dst, ptr, off)
            }
            dst.push(b']');

            off
        }
    }
}
