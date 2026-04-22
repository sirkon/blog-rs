#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parse::{ErrorLogParse, log_parse_header, read_uvarint};
use crate::log_rend::render_time;
use crate::log_render::predefined_keys_safe;
use crate::log_transfomer_into_json_consts::{
    JSON_ERROR_CTX, JSON_ERROR_LOC, JSON_ERROR_TXT, JSON_LEVEL, JSON_LEVEL_DEBUG, JSON_LEVEL_ERROR,
    JSON_LEVEL_INFO, JSON_LEVEL_PANIC, JSON_LEVEL_TRACE, JSON_LEVEL_UNKNOWN, JSON_LEVEL_WARN,
    JSON_LOCATION, JSON_MESSAGE, JSON_STACKTRACE, JSON_TIME,
};
use crate::pointer_ext::PointerExt;
use crate::transform_json_items::{
    TransformBytes, TransformDuration, TransformIntoJSONLiteral, TransformIvar, TransformString,
    TransformTime, TransformUvar,
};
use crate::value_kind::ValueKind;
use crate::{level, log_parse, value_kind};
use std::io::Read;
use std::slice;

/// Transforms log record into pure JSON.
pub struct LogTransfomer {
    pub(crate) itoa: itoa::Buffer,
    pub(crate) ryu: ryu::Buffer,
    pub(crate) buf: Vec<u8>,
    pub(crate) err_frags: Vec<(usize, usize)>,
    pub(crate) fmtbuf: Vec<u8>,

    pub(crate) max_log_size: usize,
    pub(crate) format_time: bool,
}

impl LogTransfomer {
    pub fn new() -> Self {
        Self {
            itoa: itoa::Buffer::new(),
            ryu: ryu::Buffer::new(),
            buf: Vec::with_capacity(4096),
            err_frags: Vec::with_capacity(16),
            fmtbuf: Vec::with_capacity(4096),

            max_log_size: 1 * 1024 * 1024,
            format_time: false,
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
                    self.transform_json_v1(dst, ptr.add(2), record.len() - 2)?;
                }
                _ => {
                    return Err(log_parse::ErrorLogParse::RecordVersionNotSupported(version));
                }
            }

            Ok(rest)
        }
    }

    #[inline(always)]
    unsafe fn transform_json_v1(
        &mut self,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        cap: usize,
    ) -> Result<(), ErrorLogParse> {
        dst.reserve(cap * 8);
        let porig = dst.as_mut_ptr();
        let mut pdst = dst.as_mut_ptr();
        unsafe {
            self.err_frags.clear();

            pdst = pdst.append_byte(b'{');

            // Time.
            pdst = pdst.append(JSON_TIME);
            pdst = pdst.append_byte(b':');
            let time = ptr.cast::<u64>().read_unaligned();
            if self.format_time {
                pdst = pdst.append_byte(b'"');
                self.fmtbuf.clear();
                render_time(&mut self.itoa, &mut self.fmtbuf, time as i64);
                pdst = pdst.append(self.fmtbuf.as_slice());
                pdst = pdst.append_byte(b'"');
            } else {
                pdst = pdst.append_utoa(time);
            }
            pdst = pdst.append_byte(b',');

            // Level.
            pdst = pdst.append(JSON_LEVEL);
            pdst = pdst.append_byte(b':');
            let lvl = *ptr.add(8);
            let mut is_panic = false;
            match lvl {
                level::TRACE => {
                    pdst = pdst.append(JSON_LEVEL_TRACE);
                }
                level::DEBUG => {
                    pdst = pdst.append(JSON_LEVEL_DEBUG);
                }
                level::INFO => {
                    pdst = pdst.append(JSON_LEVEL_INFO);
                }
                level::WARN => {
                    pdst = pdst.append(JSON_LEVEL_WARN);
                }
                level::ERROR => {
                    pdst = pdst.append(JSON_LEVEL_ERROR);
                }
                level::PANIC => {
                    is_panic = true;
                    pdst = pdst.append(JSON_LEVEL_PANIC);
                }
                _ => {
                    pdst = pdst.append_byte(b'"');
                    pdst = pdst.append(JSON_LEVEL_UNKNOWN);
                    pdst = pdst.append_byte(b'(');
                    let s = self.itoa.format(lvl);
                    pdst = pdst.append(s);
                    pdst = pdst.append_byte(b')');
                    pdst = pdst.append_byte(b'"');
                }
            }
            pdst = pdst.append_byte(b',');

            // May be location.
            let off: usize = if *ptr.add(9) != 0 {
                pdst = pdst.append(JSON_LOCATION);
                pdst = pdst.append_byte(b':');
                pdst = pdst.append_byte(b'"');
                let (length, size) = read_uvarint(ptr.add(9));
                pdst = pdst.append_escaped_ptr(ptr.add(9 + size), length as usize);
                let off = 9 + size + length as usize;
                let (line, size) = read_uvarint(ptr.add(off));
                pdst = pdst.append_utoa(line);
                pdst = pdst.append_byte(b',');

                off + size
            } else {
                10
            };

            // Message
            let (length, size) = read_uvarint(ptr.add(off));
            if !is_panic {
                pdst = pdst.append(JSON_MESSAGE);
                pdst = pdst.append_byte(b':');
                pdst = pdst.append_escaped_ptr(ptr.add(off + size), length as usize);
            } else {
                pdst = pdst.append(JSON_STACKTRACE).append_byte(b':');

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

                pdst = pdst.append(self.buf.as_slice());
            }

            // Context.
            pdst = self.transform_json_ctx_v1(pdst, ptr, off + size + length as usize, cap)?;
            pdst = pdst.append_byte(b'}');
            dst.set_len(dst.len() + pdst.offset_from(porig) as usize);
            Ok(())
        }
    }

    unsafe fn transform_json_ctx_v1(
        &mut self,
        mut dst: *mut u8,
        ptr: *const u8,
        mut off: usize,
        cap: usize,
    ) -> Result<*mut u8, ErrorLogParse> {
        unsafe {
            let mut error_depth = 0;
            let mut err_text = (0usize, 0usize);
            let mut old = true;
            let mut is_embed_error = false;

            while off < cap {
                let kind = *ptr.add(off) as ValueKind;
                off += 1;

                // First match to filter out things that don't follow common key->value layout.
                match kind {
                    value_kind::JUST_CONTEXT_NODE | value_kind::JUST_CONTEXT_INHERITED_NODE => {
                        if old {
                            dst = dst.append_byte(b',');
                        }
                        dst = dst.append(b"\"CTX\":{");
                        old = false;
                        error_depth += 1;
                        continue;
                    }
                    value_kind::PHANTOM_CONTEXT_NODE => {
                        continue;
                    }
                    value_kind::NEW_NODE => {
                        if old {
                            dst = dst.append_byte(b',');
                        }
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.fmtbuf.clear();
                        self.fmtbuf.extend_from_slice(b"NEW: ");
                        self.fmtbuf.extend_from_slice(slice::from_raw_parts(
                            ptr.add(off),
                            length as usize,
                        ));
                        if !is_embed_error {
                            self.err_frags.push((length as usize, off));
                        }
                        off += length as usize;
                        dst = dst.append_escaped(self.fmtbuf.as_slice());
                        dst = dst.append(b":{");
                        error_depth += 1;
                        old = false;
                        continue;
                    }
                    value_kind::WRAP_NODE | value_kind::WRAP_INHERITED_NODE => {
                        if old {
                            dst = dst.append_byte(b',');
                        }
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.fmtbuf.clear();
                        self.fmtbuf.extend_from_slice(b"NEW: ");
                        self.fmtbuf.extend_from_slice(slice::from_raw_parts(
                            ptr.add(off),
                            length as usize,
                        ));
                        if !is_embed_error {
                            self.err_frags.push((length as usize, off));
                        }
                        off += length as usize;
                        dst = dst.append_escaped(self.fmtbuf.as_slice());
                        dst = dst.append(b":{");
                        error_depth += 1;
                        old = false;
                        continue;
                    }
                    value_kind::FOREIGN_ERROR_TEXT => {
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size;
                        if !is_embed_error {
                            self.err_frags.push((length as usize, off));
                        }
                        off += length as usize;
                        continue;
                    }
                    value_kind::LOCATION_NODE => {
                        if old {
                            dst = dst.append_byte(b',');
                        }
                        old = true;
                        dst = dst.append(JSON_ERROR_LOC);
                        let (mut length, mut size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.fmtbuf.clear();
                        self.fmtbuf.reserve(length as usize * 8);
                        dst = dst.append_escaped_ptr(ptr.add(off), length as usize);
                        dst = dst.sub(1);
                        dst = dst.append_byte(b':');
                        off += length as usize;
                        (length, size) = read_uvarint(ptr.add(off));
                        dst = dst.append_utoa(length);
                        dst = dst.append_byte(b'"');
                        off += size;
                        continue;
                    }
                    value_kind::GROUP_END => {
                        old = true;
                        if error_depth == 0 {
                            dst = dst.append_byte(b'}');
                            continue;
                        }
                        error_depth -= 1;
                        if error_depth > 0 {
                            dst = dst.append_byte(b'}');
                            continue;
                        }
                        dst = dst.append_byte(b'}').append_byte(b',');
                        dst = dst.append(JSON_ERROR_TXT);
                        if is_embed_error {
                            let (len, off) = err_text;
                            dst = dst.append_escaped_ptr(ptr.add(off), len);
                        } else {
                            self.fmtbuf.clear();
                            for (i, (len, off)) in self.err_frags.iter().rev().enumerate() {
                                if i > 0 {
                                    self.fmtbuf.extend_from_slice(b": ")
                                }
                                self.fmtbuf
                                    .extend_from_slice(slice::from_raw_parts(ptr.add(*off), *len));
                            }
                            dst = dst.append_escaped(self.fmtbuf.as_slice());
                        }
                        dst = dst.append(b"}");
                        continue;
                    }
                    _ => {
                        if old {
                            dst = dst.append_byte(b',');
                        }
                        old = true;
                    }
                }

                // Common layout it is.

                // Write key.
                #[allow(unused_assignments)]
                let mut key_len: usize = 0;
                #[allow(unused_assignments)]
                let mut key_off: usize = 0;
                let v = *(ptr.add(off));
                if v != 0 {
                    let (length, size) = read_uvarint(ptr.add(off));
                    key_len = length as usize;
                    key_off = off + size;
                    off += size + length as usize;
                    dst = dst.append_escaped_ptr(ptr.add(key_off), key_len);
                } else {
                    let (length, size) = read_uvarint(ptr.add(off + 1));
                    key_off = length as usize;
                    match predefined_keys_safe(key_off as ValueKind) {
                        Ok(v) => {
                            dst = dst.append_quoted(v);
                        }
                        Err(_) => {
                            return Err(ErrorLogParse::RecordContextNodePredefinedKeyUnknown(
                                key_off as u64,
                            ));
                        }
                    }
                    off += size + 1;
                }
                dst = dst.append_byte(b':');

                // Write value.
                match kind {
                    // These values will not just be shown here.
                    //  value_kind::JUST_CONTEXT_NODE
                    //  | value_kind::JUST_CONTEXT_INHERITED_NODE
                    //  | value_kind::NEW_NODE
                    //  | value_kind::WRAP_NODE
                    //  | value_kind::WRAP_INHERITED_NODE
                    //  | value_kind::FOREIGN_ERROR_TEXT
                    //  | value_kind::LOCATION_NODE
                    //  | value_kind::GROUP_END
                    //  | value_kind::PHANTOM_CONTEXT_NODE => {}
                    value_kind::BOOL => {
                        (dst, off) = self.render_json::<bool>(dst, ptr, off);
                    }
                    value_kind::TIME => {
                        (dst, off) = self.render_json::<TransformTime>(dst, ptr, off);
                    }
                    value_kind::DURATION => {
                        (dst, off) = self.render_json::<TransformDuration>(dst, ptr, off);
                    }
                    value_kind::I | value_kind::I64 => {
                        (dst, off) = self.render_json::<i64>(dst, ptr, off);
                    }
                    value_kind::IVAR => {
                        (dst, off) = self.render_json::<TransformIvar>(dst, ptr, off);
                    }
                    value_kind::I8 => {
                        (dst, off) = self.render_json::<i8>(dst, ptr, off);
                    }
                    value_kind::I16 => {
                        (dst, off) = self.render_json::<i16>(dst, ptr, off);
                    }
                    value_kind::I32 => {
                        (dst, off) = self.render_json::<i32>(dst, ptr, off);
                    }
                    value_kind::U | value_kind::U64 => {
                        (dst, off) = self.render_json::<u64>(dst, ptr, off);
                    }
                    value_kind::UVAR => {
                        (dst, off) = self.render_json::<TransformUvar>(dst, ptr, off);
                    }
                    value_kind::U8 => {
                        (dst, off) = self.render_json::<u8>(dst, ptr, off);
                    }
                    value_kind::U16 => {
                        (dst, off) = self.render_json::<u16>(dst, ptr, off);
                    }
                    value_kind::U32 => {
                        (dst, off) = self.render_json::<u32>(dst, ptr, off);
                    }
                    value_kind::FLOAT32 => {
                        (dst, off) = self.render_json::<f32>(dst, ptr, off);
                    }
                    value_kind::FLOAT64 => {
                        (dst, off) = self.render_json::<f64>(dst, ptr, off);
                    }
                    value_kind::STRING => {
                        (dst, off) = self.render_json::<TransformString>(dst, ptr, off);
                    }
                    value_kind::BYTES => {
                        (dst, off) = self.render_json::<TransformBytes>(dst, ptr, off);
                    }
                    value_kind::ERROR_RAW => {
                        (dst, off) = self.render_json::<TransformString>(dst, ptr, off);
                    }
                    value_kind::SLICE_BOOL => {
                        (dst, off) = self.render_slice_json::<bool>(dst, ptr, off);
                    }
                    value_kind::SLICE_I | value_kind::SLICE_I64 => {
                        (dst, off) = self.render_slice_json::<i64>(dst, ptr, off);
                    }
                    value_kind::SLICE_I8 => {
                        (dst, off) = self.render_slice_json::<i8>(dst, ptr, off);
                    }
                    value_kind::SLICE_I16 => {
                        (dst, off) = self.render_slice_json::<i16>(dst, ptr, off);
                    }
                    value_kind::SLICE_I32 => {
                        (dst, off) = self.render_slice_json::<i32>(dst, ptr, off);
                    }
                    value_kind::SLICE_U | value_kind::SLICE_U64 => {
                        (dst, off) = self.render_slice_json::<u64>(dst, ptr, off);
                    }
                    value_kind::SLICE_U8 => {
                        (dst, off) = self.render_slice_json::<u8>(dst, ptr, off);
                    }
                    value_kind::SLICE_U16 => {
                        (dst, off) = self.render_slice_json::<u16>(dst, ptr, off);
                    }
                    value_kind::SLICE_U32 => {
                        (dst, off) = self.render_slice_json::<u32>(dst, ptr, off);
                    }
                    value_kind::SLICE_F32 => {
                        (dst, off) = self.render_slice_json::<f32>(dst, ptr, off);
                    }
                    value_kind::SLICE_F64 => {
                        (dst, off) = self.render_slice_json::<f64>(dst, ptr, off);
                    }
                    value_kind::SLICE_STRING => {
                        (dst, off) = self.render_slice_json::<TransformString>(dst, ptr, off);
                    }

                    value_kind::GROUP => {
                        old = false;
                        dst = dst.append_byte(b'{');
                    }
                    value_kind::ERROR => {
                        is_embed_error = false;
                        error_depth += 1;
                        dst = dst.append(JSON_ERROR_CTX);
                        old = false;
                        self.err_frags.clear();
                    }
                    value_kind::ERROR_EMBED => {
                        is_embed_error = true;
                        error_depth += 1;
                        dst = dst.append(JSON_ERROR_CTX);
                        old = false;
                        let (len, size) = read_uvarint(ptr.add(off));
                        off += size;
                        err_text = (len as usize, off);
                        off += len as usize;
                    }

                    _ => {
                        return Err(ErrorLogParse::RecordContextNodePredefinedKeyUnknown(kind));
                    }
                }
            }

            Ok(dst)
        }
    }

    #[inline(always)]
    unsafe fn render_json<T>(
        &mut self,
        dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize)
    where
        T: TransformIntoJSONLiteral,
    {
        unsafe { T::render(self, dst, ptr, off) }
    }

    #[inline(always)]
    unsafe fn render_slice_json<T>(
        &mut self,
        mut dst: *mut u8,
        ptr: *const u8,
        mut off: usize,
    ) -> (*mut u8, usize)
    where
        T: TransformIntoJSONLiteral,
    {
        unsafe {
            let (length, size) = read_uvarint(ptr.add(off));
            off += size;
            dst = dst.append_byte(b'[');
            if length == 0 {
                dst = dst.append_byte(b']');
                return (dst, off);
            }

            for i in 0..length {
                if i != 0 {
                    dst = dst.append_byte(b',');
                }

                (dst, off) = T::render(self, dst, ptr, off)
            }
            dst = dst.append_byte(b']');

            (dst, off)
        }
    }
}
