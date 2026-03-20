use super::*;
use crate::log_parser::{LogParser, read_uvarint, read_uvarint_safe};
use crate::log_parser_node::NodeKind;
use std::slice;

impl LogParser {
    // Parser input source as a log record and returns the rest of data.
    pub(crate) unsafe fn parse_log_data<'a>(
        &mut self,
        src: &'a [u8],
    ) -> Result<&'a [u8], log_parser::ErrorLogParse> {
        unsafe {
            if src.len() < 5 {
                return Err(log_parser::ErrorLogParse::NoHeader);
            }

            let ptr = src.as_ptr() as *mut u8;
            if *ptr != 0xFF {
                return Err(log_parser::ErrorLogParse::StartMarkerInvalid);
            }

            let record_crc = ptr.add(1).cast::<u32>().read_unaligned();
            let (length, size) = read_uvarint_safe(ptr.add(5), src.len() - 5);
            if size == usize::MAX {
                return Err(log_parser::ErrorLogParse::RecordLengthInvalid);
            }
            if length as usize > self.max_log_size {
                return Err(log_parser::ErrorLogParse::RecordLengthTooLarge);
            }
            if 5 + size + length as usize > src.len() {
                return Err(log_parser::ErrorLogParse::RecordNeedMore);
            }
            let off = 5 + size;
            self.source_off = off;
            let record = slice::from_raw_parts(ptr.add(off), length as usize);
            let check = crc32c::crc32c(record);
            if check != record_crc {
                return Err(log_parser::ErrorLogParse::RecordBroken);
            }
            self.parse_log_record(record)?;

            let rest = slice::from_raw_parts(
                ptr.add(off + length as usize),
                src.len() - off - length as usize,
            );
            Ok(rest)
        }
    }

    pub(crate) unsafe fn parse_log_record<'a>(
        &mut self,
        src: &'a [u8],
    ) -> Result<(), log_parser::ErrorLogParse> {
        unsafe {
            let ptr = src.as_ptr() as *mut u8;

            // Get and check version.
            let version = ptr.cast::<u16>().read_unaligned();
            // TODO
            if version != 1 {
                return Err(log_parser::ErrorLogParse::RecordVersionNotSupported(
                    version,
                ));
            }

            self.time = ptr.add(2).cast::<u64>().read_unaligned();

            self.level = *ptr.add(10);
            if self.level < self.process_since_level {
                return Ok(());
            }
            let mut off: usize = 11;

            self.location = if *ptr.add(off) == 0 {
                off += 1;
                None
            } else {
                let (length, size) = read_uvarint(ptr.add(off));
                let _file = slice::from_raw_parts(ptr.add(off + size), length as usize);
                let loc_file_off = off + size;
                off += size + length as usize;
                let (line, line_size) = read_uvarint(ptr.add(off));
                off += line_size;
                Some((length as usize, loc_file_off, line as usize))
            };

            let (msg_length, size) = read_uvarint(ptr.add(off));
            self.msg = (msg_length as usize, off + size);
            off += size + msg_length as usize;

            self.parse_ctx(slice::from_raw_parts_mut(ptr.add(off), src.len() - off));

            Ok(())
        }
    }

    pub(crate) unsafe fn parse_ctx<'a>(&mut self, src: &'a [u8]) {
        unsafe {
            self.ctx.reset();
            self.groups_lens.set_len(0);
            self.caps.set_len(0);
            self.err_frags.set_len(0);
            self.state_stack.set_len(0);
            self.ctx_size = 0;
            self.has_errors = false;

            let mut off: usize = 0;
            let _need_tree: bool = false;
            let ptr = src.as_ptr() as *mut u8;
            let mut had_stages = false;
            let mut cap = src.len();
            let mut parsing_state = log_parser::CtxParsingState::Normal;
            let mut group_cap: usize = 0;
            let mut group_off: usize = 0;
            loop {
                match parsing_state {
                    log_parser::CtxParsingState::Normal => {
                        if off >= cap {
                            if self.caps.is_empty() {
                                return;
                            }
                        }
                    }

                    log_parser::CtxParsingState::Group => {
                        if group_off >= group_cap {
                            self.ctx.leave_group();
                            if !self.groups_lens.is_empty() {
                                (group_off, group_cap) = self.groups_lens.pop().unwrap();
                                continue;
                            } else {
                                parsing_state = self.state_stack.pop().unwrap();
                            }
                        }
                    }

                    log_parser::CtxParsingState::Error => {
                        if off >= cap {
                            self.ctx.leave_group(); // Leave context group which is here.
                            self.ctx.leave_group(); // Leave error itself.
                            if self.caps.is_empty() {
                                return;
                            }
                            cap = self.caps.pop().unwrap();
                            parsing_state = self.state_stack.pop().unwrap();
                        }
                    }

                    log_parser::CtxParsingState::ErrorEmbed => {
                        if off >= cap {
                            self.ctx.leave_group();
                            self.ctx.leave_group();
                            if self.caps.is_empty() {
                                return;
                            }
                            cap = self.caps.pop().unwrap();
                            parsing_state = self.state_stack.pop().unwrap();
                        }
                    }
                }

                // Read code and continue the loop on some types that have no payload.
                let kind = *(ptr.add(off)) as value_kind::ValueKind;
                off += 1;
                match kind {
                    value_kind::JUST_CONTEXT_NODE | value_kind::JUST_CONTEXT_INHERITED_NODE => {
                        had_stages = self.leave_stage_group_if_needed(had_stages);
                        self.ctx.add(NodeKind::ErrorStageCtx, 0, 0, 0, 0);
                        self.ctx.enter_group();
                        continue;
                    }
                    value_kind::PHANTOM_CONTEXT_NODE => {
                        continue;
                    }
                    _ => {}
                }

                // Read the key. It can be either 0-lead uvarint of predefined key index, or
                // a literal key with uvarint(length) + body.
                let mut key_len: u32 = 0;
                let mut key_off: u32 = 0;
                let v = *(ptr.add(off));
                if v != 0 {
                    let (length, size) = read_uvarint(ptr.add(off));
                    key_len = length as u32;
                    key_off = (off + size) as u32;
                    off += size + length as usize;
                } else {
                    let (length, size) = read_uvarint(ptr.add(off + 1));
                    key_len = 0;
                    key_off = length as u32;
                    off += size + 1;
                }

                match kind {
                    value_kind::NEW_NODE => {
                        had_stages = self.leave_stage_group_if_needed(had_stages);
                        let (length, size) = read_uvarint(ptr.add(off));
                        self.ctx.add(
                            NodeKind::ErrorStageNew,
                            key_len,
                            key_off,
                            length as u32,
                            (off + size) as u32,
                        );
                        off += size + length as usize;
                        self.ctx.enter_group();
                    }
                    value_kind::WRAP_NODE | value_kind::WRAP_INHERITED_NODE => {
                        had_stages = self.leave_stage_group_if_needed(had_stages);
                        let (length, size) = read_uvarint(ptr.add(off));
                        self.ctx.add(
                            NodeKind::ErrorStageWrap,
                            key_len,
                            key_off,
                            length as u32,
                            (off + size) as u32,
                        );
                        off += size + length as usize;
                        self.ctx.enter_group();
                    }
                    value_kind::LOCATION_NODE => {
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.ctx
                            .add(NodeKind::ErrLoc, key_len, key_off, 0, length as u32);
                    }
                    value_kind::FOREIGN_ERROR_TEXT => {
                        self.ctx
                            .add(NodeKind::ErrTxtFragment, key_len, key_off, 0, 0);
                    }
                    value_kind::FOREIGN_ERROR_FORMAT => {
                        // Not supported as for now.
                    }

                    value_kind::BOOL => {
                        let v = *(ptr.add(off));
                        self.ctx.add(NodeKind::Bool, key_len, key_off, 0, v as u32);
                        off += 1;
                        self.ctx_size += 1;
                    }

                    value_kind::TIME => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::Time, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::DURATION => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::Time, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::I => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::Int, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::I8 => {
                        let v = *(ptr.add(off) as *const i8);
                        self.ctx.add(NodeKind::I8, key_len, key_off, 0, v as u32);
                        off += 1;
                        self.ctx_size += 1;
                    }

                    value_kind::I16 => {
                        let v = u16::from_le(ptr.add(off).cast::<u16>().read_unaligned());
                        self.ctx.add(NodeKind::I16, key_len, key_off, 0, v as u32);
                        off += 2;
                        self.ctx_size += 1;
                    }

                    value_kind::I32 => {
                        let v = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
                        self.ctx.add(NodeKind::I32, key_len, key_off, 0, v as u32);
                        off += 4;
                        self.ctx_size += 1;
                    }

                    value_kind::I64 => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::Int, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::U => {
                        let v = ptr.add(off).cast::<u64>().read_unaligned();
                        self.ctx
                            .add(NodeKind::Uint, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::U8 => {
                        let v = *(ptr.add(off) as *const u8);
                        self.ctx.add(NodeKind::U8, key_len, key_off, 0, v as u32);
                        off += 1;
                        self.ctx_size += 1;
                    }

                    value_kind::U16 => {
                        let v = u16::from_le(ptr.add(off).cast::<u16>().read_unaligned());
                        self.ctx.add(NodeKind::U16, key_len, key_off, 0, v as u32);
                        off += 2;
                        self.ctx_size += 1;
                    }

                    value_kind::U32 => {
                        let v = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
                        self.ctx.add(NodeKind::U32, key_len, key_off, 0, v);
                        off += 4;
                        self.ctx_size += 1;
                    }

                    value_kind::U64 => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::U64, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::FLOAT32 => {
                        let v = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
                        self.ctx.add(NodeKind::F32, key_len, key_off, 0, v);
                        off += 4;
                        self.ctx_size += 1;
                    }

                    value_kind::FLOAT64 => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::F64, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    value_kind::STRING => {
                        off = self.varthing(ptr, off, NodeKind::Str, key_len, key_off);
                        self.ctx_size += 1;
                    }

                    value_kind::BYTES => {
                        off = self.varthing(ptr, off, NodeKind::Bytes, key_len, key_off);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_BOOL => {
                        off = self.slice(ptr, off, NodeKind::Bool, key_len, key_off, 1);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_I => {
                        off = self.slice(ptr, off, NodeKind::Ints, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_I8 => {
                        off = self.slice(ptr, off, NodeKind::I8s, key_len, key_off, 1);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_I16 => {
                        off = self.slice(ptr, off, NodeKind::I16s, key_len, key_off, 2);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_I32 => {
                        off = self.slice(ptr, off, NodeKind::I32s, key_len, key_off, 4);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_I64 => {
                        off = self.slice(ptr, off, NodeKind::I64s, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_U => {
                        off = self.slice(ptr, off, NodeKind::Uints, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_U8 => {
                        off = self.slice(ptr, off, NodeKind::U8s, key_len, key_off, 1);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_U16 => {
                        off = self.slice(ptr, off, NodeKind::U16s, key_len, key_off, 2);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_U32 => {
                        off = self.slice(ptr, off, NodeKind::U32s, key_len, key_off, 4);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_U64 => {
                        off = self.slice(ptr, off, NodeKind::U64s, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_F32 => {
                        off = self.slice(ptr, off, NodeKind::F32s, key_len, key_off, 4);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_F64 => {
                        off = self.slice(ptr, off, NodeKind::F64s, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    value_kind::SLICE_STRING => {
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        off += size;
                        let start = off;
                        for _ in 0..lenght {
                            let (length, size) = read_uvarint(ptr.add(off));
                            off += size + length as usize;
                        }
                        self.ctx.add(
                            NodeKind::Strs,
                            key_len,
                            key_off,
                            lenght as u32,
                            start as u32,
                        );
                        self.ctx_size += 1;
                    }

                    value_kind::ERROR => {
                        self.ctx.add(NodeKind::Error, key_len, key_off, 0, 0);
                        self.ctx.enter_group();
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.caps.push(cap);
                        self.state_stack.push(parsing_state);
                        parsing_state = log_parser::CtxParsingState::Error;
                        cap = off + lenght as usize;
                        self.ctx_size += 1;
                        self.has_errors = true;
                    }

                    value_kind::ERROR_EMBED => {
                        self.ctx.add(NodeKind::ErrorEmbed, key_len, key_off, 0, 0);
                        self.ctx.enter_group();
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.ctx
                            .add(NodeKind::ErrEmbedText, 0, 0, lenght as u32, off as u32);
                        self.caps.push(cap);
                        self.state_stack.push(parsing_state);
                        off = off + lenght as usize;
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        cap = off + size + lenght as usize;
                        self.ctx_size += 1;
                        self.has_errors = true;
                    }

                    value_kind::GROUP => {
                        self.ctx.add(NodeKind::Group, key_len, key_off, 0, 0);
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.groups_lens.push((group_off, group_cap));
                        self.state_stack.push(parsing_state);
                        group_off = 0;
                        group_cap = lenght as usize;
                        parsing_state = log_parser::CtxParsingState::Group;
                        self.ctx_size += 1;
                    }

                    _ => {}
                }

                group_off += 1;
            }
        }
    }
}
