#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use super::*;
use crate::log_parse;
use crate::log_parse::{log_parse_header, read_uvarint, read_varint};
use crate::log_parser::LogParser;
use crate::log_parser_node::NodeKind;
use crate::value_kind::ValueKind;
use std::fmt::{Display, Formatter};
use std::slice;

impl LogParser {
    /// Parse input source as a log record and returns:
    ///
    ///  1. Log record data of the first record in the data provided striped from the header.
    ///  2. The data after that first record. A tail.
    pub(crate) unsafe fn parse_log_data<'a>(
        &mut self,
        src: &'a [u8],
    ) -> Result<(&'a [u8], &'a [u8]), log_parse::ErrorLogParse> {
        unsafe {
            let (record, rest) = match log_parse_header(src, self.max_log_size) {
                Ok(x) => x,
                Err(e) => return Err(e),
            };
            self.parse_log_record(record)?;

            Ok((record, rest))
        }
    }

    pub(crate) unsafe fn parse_log_record<'a>(
        &mut self,
        src: &'a [u8],
    ) -> Result<(), log_parse::ErrorLogParse> {
        unsafe {
            let ptr = src.as_ptr() as *mut u8;

            // Get and check version.
            let version = ptr.cast::<u16>().read_unaligned();
            // TODO
            if version != 1 {
                return Err(log_parse::ErrorLogParse::RecordVersionNotSupported(version));
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

            self.parse_ctx(ptr, off, src.len());

            Ok(())
        }
    }

    pub(crate) unsafe fn parse_ctx<'a>(&mut self, ptr: *const u8, mut off: usize, cap: usize) {
        unsafe {
            self.ctx.reset();
            self.group_stack.clear();
            self.group_depth = 0;
            self.ctx_size = 0;
            self.has_errors = false;

            let _need_tree: bool = false;
            let mut prev = PrevElement::Whatever;
            while off < cap {
                self.group_depth = self.ctx.stack.len();

                // Read code and continue the loop on some types that have no payload.
                let kind = *(ptr.add(off) as *const ValueKind);
                off += 1;
                match kind {
                    ValueKind::JustContextNode => {
                        self.group_stack.push(self.ctx.ctrl.len());
                        self.ctx
                            .add(NodeKind::ErrorStageCtx, 0, 0, self.ctrl_len(), 0);
                        prev = PrevElement::Whatever;
                        continue;
                    }
                    ValueKind::PhantomContextNode => {
                        prev = PrevElement::Whatever;
                        continue;
                    }
                    ValueKind::GroupEnd => {
                        self.mark_as_last(prev);
                        let start = self.group_stack.pop().unwrap();
                        self.ctx.add(NodeKind::GroupEnd, 0, 0, 0, 0);
                        prev = PrevElement::End(start);
                        continue;
                    }
                    _ => {}
                }

                // Read the key. It can be either 0-lead uvarint of predefined key index, or
                // a literal key with uvarint(length) + body.
                #[allow(unused_assignments)]
                let mut key_len: u32 = 0;
                #[allow(unused_assignments)]
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

                prev = PrevElement::Whatever;
                match kind {
                    ValueKind::NewNode => {
                        self.group_stack.push(self.ctx.ctrl.len());
                        self.ctx.add(
                            NodeKind::ErrorStageNew,
                            key_len,
                            key_off,
                            self.ctrl_len(),
                            0,
                        );
                    }
                    ValueKind::WrapNode => {
                        self.group_stack.push(self.ctx.ctrl.len());
                        self.ctx.add(
                            NodeKind::ErrorStageWrap,
                            key_len,
                            key_off,
                            self.ctrl_len(),
                            0,
                        );
                    }
                    ValueKind::LocationNode => {
                        let (length, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.ctx
                            .add(NodeKind::ErrLoc, key_len, key_off, 0, length as u32);
                    }
                    ValueKind::ForeignErrorText => {
                        self.ctx
                            .add(NodeKind::ErrTxtFragment, key_len, key_off, 0, 0);
                    }

                    ValueKind::Bool => {
                        let v = *(ptr.add(off));
                        self.ctx.add(NodeKind::Bool, key_len, key_off, 0, v as u32);
                        off += 1;
                        self.ctx_size += 1;
                    }

                    ValueKind::Time => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::Time, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    ValueKind::Duration => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::Dur, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    ValueKind::Ivar => {
                        let (value, size) = read_varint(ptr.add(off));
                        self.ctx.add(
                            NodeKind::IVar,
                            key_len,
                            key_off,
                            value as u32,
                            (value as u64 >> 32) as u32,
                        );
                        off += size;
                        self.ctx_size += 1;
                    }

                    ValueKind::I8 => {
                        let v = *(ptr.add(off) as *const i8);
                        self.ctx.add(NodeKind::I8, key_len, key_off, 0, v as u32);
                        off += 1;
                        self.ctx_size += 1;
                    }

                    ValueKind::I16 => {
                        let v = u16::from_le(ptr.add(off).cast::<u16>().read_unaligned());
                        self.ctx.add(NodeKind::I16, key_len, key_off, 0, v as u32);
                        off += 2;
                        self.ctx_size += 1;
                    }

                    ValueKind::I32 => {
                        let v = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
                        self.ctx.add(NodeKind::I32, key_len, key_off, 0, v as u32);
                        off += 4;
                        self.ctx_size += 1;
                    }

                    ValueKind::I64 => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::I64, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    ValueKind::Uvar => {
                        let (value, size) = read_uvarint(ptr.add(off));
                        self.ctx.add(
                            NodeKind::UVar,
                            key_len,
                            key_off,
                            value as u32,
                            (value >> 32) as u32,
                        );
                        off += size;
                        self.ctx_size += 1;
                    }

                    ValueKind::U8 => {
                        let v = *(ptr.add(off) as *const u8);
                        self.ctx.add(NodeKind::U8, key_len, key_off, 0, v as u32);
                        off += 1;
                        self.ctx_size += 1;
                    }

                    ValueKind::U16 => {
                        let v = u16::from_le(ptr.add(off).cast::<u16>().read_unaligned());
                        self.ctx.add(NodeKind::U16, key_len, key_off, 0, v as u32);
                        off += 2;
                        self.ctx_size += 1;
                    }

                    ValueKind::U32 => {
                        let v = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
                        self.ctx.add(NodeKind::U32, key_len, key_off, 0, v);
                        off += 4;
                        self.ctx_size += 1;
                    }

                    ValueKind::U64 => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::U64, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    ValueKind::Float32 => {
                        let v = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
                        self.ctx.add(NodeKind::F32, key_len, key_off, 0, v);
                        off += 4;
                        self.ctx_size += 1;
                    }

                    ValueKind::Float64 => {
                        let v = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
                        self.ctx
                            .add(NodeKind::F64, key_len, key_off, v as u32, (v >> 32) as u32);
                        off += 8;
                        self.ctx_size += 1;
                    }

                    ValueKind::String => {
                        off = self.varthing(ptr, off, NodeKind::Str, key_len, key_off);
                        self.ctx_size += 1;
                    }

                    ValueKind::Bytes => {
                        off = self.varthing(ptr, off, NodeKind::Bytes, key_len, key_off);
                        self.ctx_size += 1;
                    }

                    ValueKind::ErrorRaw => {
                        off = self.varthing(ptr, off, NodeKind::ErrTxt, key_len, key_off);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceBool => {
                        off = self.slice(ptr, off, NodeKind::Bools, key_len, key_off, 1);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceI64 => {
                        off = self.slice(ptr, off, NodeKind::Ints, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceI8 => {
                        off = self.slice(ptr, off, NodeKind::I8s, key_len, key_off, 1);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceI16 => {
                        off = self.slice(ptr, off, NodeKind::I16s, key_len, key_off, 2);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceI32 => {
                        off = self.slice(ptr, off, NodeKind::I32s, key_len, key_off, 4);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceU64 => {
                        off = self.slice(ptr, off, NodeKind::Uints, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceU8 => {
                        off = self.slice(ptr, off, NodeKind::U8s, key_len, key_off, 1);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceU16 => {
                        off = self.slice(ptr, off, NodeKind::U16s, key_len, key_off, 2);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceU32 => {
                        off = self.slice(ptr, off, NodeKind::U32s, key_len, key_off, 4);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceF32 => {
                        off = self.slice(ptr, off, NodeKind::F32s, key_len, key_off, 4);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceF64 => {
                        off = self.slice(ptr, off, NodeKind::F64s, key_len, key_off, 8);
                        self.ctx_size += 1;
                    }

                    ValueKind::SliceString => {
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

                    ValueKind::Error => {
                        self.group_stack.push(self.ctx.ctrl.len());
                        self.ctx
                            .add(NodeKind::Error, key_len, key_off, self.ctrl_len(), 0);
                        self.ctx_size += 1;
                        self.has_errors = true;
                    }

                    ValueKind::ErrorEmbed => {
                        self.group_stack.push(self.ctx.ctrl.len());
                        self.ctx
                            .add(NodeKind::ErrorEmbed, key_len, key_off, self.ctrl_len(), 0);

                        // extract a frame of the embedded error text.
                        let (lenght, size) = read_uvarint(ptr.add(off));
                        off += size;
                        self.ctx
                            .add(NodeKind::ErrEmbedText, 0, 0, lenght as u32, off as u32);
                        off += lenght as usize;

                        // Now, to the payload!
                        self.ctx_size += 1;
                        self.has_errors = true;
                    }

                    ValueKind::Group => {
                        self.group_stack.push(self.ctx.ctrl.len());
                        self.ctx
                            .add(NodeKind::Group, key_len, key_off, self.ctrl_len(), 0);
                        self.ctx_size += 1;
                    }

                    _ => {
                        panic!(
                            "unknown value kind {} at offset {} out of {}",
                            value_kind::string(kind),
                            off,
                            cap
                        );
                    }
                }
            }
            self.mark_as_last(prev);
        }
    }

    fn ctrl_len(&self) -> u32 {
        self.ctx.ctrl.len() as u32
    }

    #[inline(always)]
    unsafe fn mark_as_last(&mut self, prev: PrevElement) {
        unsafe {
            match prev {
                PrevElement::End(idx) => {
                    let curlen = self.ctrl_len();
                    let x = self.ctx.ctrl.get_unchecked_mut(idx);
                    x.val_len = curlen - x.val_len;
                    x.is_last = 1
                }
                PrevElement::Whatever => {
                    let idx = self.ctx.ctrl.len() - 1;
                    if idx == usize::MAX {
                        return;
                    };
                    let curlen = self.ctrl_len();
                    let x = self.ctx.ctrl.get_unchecked_mut(idx);
                    if !x.kind.is_group() {
                        x.is_last = 1
                    } else {
                        x.val_len = curlen - x.val_len;
                    }
                }
            };
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrevElement {
    Whatever,
    End(usize),
}

impl Display for PrevElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PrevElement::Whatever => f.write_str("Whatever"),
            PrevElement::End(x) => {
                f.write_str("End(")?;
                f.write_str(format!("{}", x).as_str())?;
                f.write_str(")")
            }
        }
    }
}
