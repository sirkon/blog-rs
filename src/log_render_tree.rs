#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parser_node::{group_is_empty, Node, NodeKind};
use crate::log_render::{LogRender, RenderGroupType};
use crate::log_render_tree_prefixes::render_tree_prefix;
use crate::value_kind::{PREDEFINED_NAME_CONTEXT, PREDEFINED_NAME_LOCATION, PREDEFINED_NAME_TEXT};
use crate::{log_render, slice_items};
use std::panic::Location;
use std::slice;

const TREE_ITEM_INTR: &'static [u8; 7] = b"\xE2\x94\x9C\xE2\x94\x80 ";
const TREE_ITEM_FIN: &'static [u8; 7] = b"\xE2\x94\x94\xE2\x94\x80 ";

impl<'a> LogRender<'a> {
    pub(crate) unsafe fn render_tree(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        let mut error_text = (0 as usize, 0 as usize);
        let mut is_embed_error = false;
        let mut error_depth = 0 as u64;
        let ptr = src.as_ptr();

        let mut iter = self.ctx.iter().peekable();
        dst.push(b'\n');
        while let Some(node) = iter.next() {
            let is_last = node.is_last != 0;
            match node.kind {
                NodeKind::ErrTxtFragment => {
                    if !is_embed_error {
                        self.err_stack
                            .push((node.key_len as usize, node.key_off as usize))
                    }
                    continue;
                }
                NodeKind::ErrLoc => {
                    self.render_tree_prefix(dst, is_last);
                    self.color_err_meta(dst);
                    dst.extend_from_slice(log_render::predefined_key(PREDEFINED_NAME_LOCATION));
                    dst.extend_from_slice(b": ");
                    self.color_reset(dst);
                }
                NodeKind::ErrEmbedText => {
                    error_text = (node.val_len as usize, node.val_off as usize);
                    continue;
                }
                NodeKind::ErrorStageNew => {
                    self.render_tree_prefix(dst, is_last);
                    self.color_err_stage(dst);
                    dst.extend_from_slice(b"NEW: ");
                    dst.extend_from_slice(slice::from_raw_parts(
                        ptr.add(node.key_off as usize),
                        node.key_len as usize,
                    ));
                    dst.push(b'\n');
                    if !is_embed_error {
                        self.err_stack
                            .push((node.key_len as usize, node.key_off as usize));
                    }
                    error_depth += 1;
                    self.push_prefix(false);
                    continue;
                }
                NodeKind::ErrorStageWrap => {
                    self.render_tree_prefix(dst, is_last);
                    self.color_err_stage(dst);
                    dst.extend_from_slice(b"WRAP: ");
                    dst.extend_from_slice(slice::from_raw_parts(
                        ptr.add(node.key_off as usize),
                        node.key_len as usize,
                    ));
                    if !is_embed_error {
                        self.err_stack
                            .push((node.key_len as usize, node.key_off as usize));
                    }
                    dst.push(b'\n');
                    self.push_prefix(false);
                    error_depth += 1;
                    continue;
                }
                NodeKind::ErrorStageCtx => {
                    self.render_tree_prefix(dst, is_last);
                    self.color_err_stage(dst);
                    dst.extend_from_slice(b"CTX");
                    dst.push(b'\n');
                    self.push_prefix(false);
                    error_depth += 1;
                    continue;
                }
                NodeKind::GroupEnd => {
                    self.pop_prefix();
                    if error_depth == 0 {
                        continue;
                    }
                    error_depth -= 1;
                    if error_depth > 0 {
                        continue;
                    }
                    self.render_tree_prefix(dst, true);
                    self.pop_prefix();
                    self.color_err_key(dst);
                    dst.extend_from_slice(log_render::predefined_key(PREDEFINED_NAME_TEXT));
                    dst.extend_from_slice(b": ");
                    self.color_level_error(dst);
                    if is_embed_error {
                        let (len, off) = error_text;
                        dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), len));
                    } else {
                        for (i, x) in self.err_stack.iter().rev().enumerate() {
                            let (len, off) = x;
                            if i != 0 {
                                dst.extend_from_slice(b": ");
                            }
                            dst.extend_from_slice(slice::from_raw_parts(ptr.add(*off), *len));
                        }
                    }
                    self.color_reset(dst);
                    dst.push(b'\n');
                }
                NodeKind::Error | NodeKind::ErrorEmbed | NodeKind::ErrTxt => {
                    self.render_tree_prefix(dst, is_last);
                    self.color_err_key(dst);
                    dst.extend_from_slice(slice::from_raw_parts(
                        ptr.add(node.key_off as usize),
                        node.key_len as usize,
                    ));
                    dst.extend_from_slice(b": ");
                    self.color_reset(dst);
                }
                _ => {
                    self.render_tree_prefix(dst, is_last);
                    self.color_key(dst);
                    dst.extend_from_slice(slice::from_raw_parts(
                        ptr.add(node.key_off as usize),
                        node.key_len as usize,
                    ));
                    dst.extend_from_slice(b": ");
                    self.color_reset(dst);
                }
            }

            let (val_len, val_off) = (node.val_len as usize, node.val_off as usize);
            match node.kind {
                NodeKind::Bool => {
                    if node.val_off != 0 {
                        dst.extend_from_slice(b"true");
                    } else {
                        dst.extend_from_slice(b"false");
                    }
                    dst.push(b'\n');
                }
                NodeKind::Time => {
                    self.render_time(dst, node.val_as_u64() as i64, true);
                    dst.push(b'\n');
                }
                NodeKind::Dur => {
                    self.render_go_duration(dst, node.val_as_u64(), true);
                }
                NodeKind::Int | NodeKind::I64 | NodeKind::IVar => {
                    self.render_int(dst, node.val_as_u64() as i64);
                    dst.push(b'\n');
                }
                NodeKind::I8 | NodeKind::I16 | NodeKind::I32 => {
                    self.render_int(dst, node.val_off as i64);
                    dst.push(b'\n');
                }
                NodeKind::Uint | NodeKind::UVar | NodeKind::U64 => {
                    self.render_uint(dst, node.val_as_u64());
                    dst.push(b'\n');
                }
                NodeKind::U8 | NodeKind::U16 | NodeKind::U32 => {
                    self.render_uint(dst, node.val_off as u64);
                    dst.push(b'\n');
                }
                NodeKind::F32 => {
                    self.render_float(dst, f32::from_bits(node.val_off) as f64);
                    dst.push(b'\n');
                }
                NodeKind::F64 => {
                    self.render_float(dst, f64::from_bits(node.val_as_u64()));
                    dst.push(b'\n');
                }
                NodeKind::Str => {
                    dst.extend_from_slice(node.val_as_slice(ptr));
                    dst.push(b'\n');
                }
                NodeKind::Bytes => {
                    dst.extend_from_slice(b"base64.");
                    base64_simd::STANDARD.encode_append(node.val_as_slice(ptr), dst);
                    dst.push(b'\n');
                }
                NodeKind::ErrTxt => {
                    self.color_level_error(dst);
                    dst.extend_from_slice(node.val_as_slice(ptr));
                    self.color_reset(dst);
                    dst.push(b'\n');
                }
                NodeKind::ErrTxtFragment => {}
                NodeKind::ErrLoc => {
                    self.color_loc(dst);
                    dst.extend_from_slice(node.key_as_slice_direct(ptr));
                    dst.push(b':');
                    self.render_uint(dst, node.val_off as u64);
                    self.color_reset(dst);
                    dst.push(b'\n');
                }
                NodeKind::ErrEmbedText => {}
                NodeKind::Bools => {
                    self.render_tree_slice::<bool>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::Ints | NodeKind::I64s => {
                    self.render_tree_slice::<i64>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::I8s => {
                    self.render_tree_slice::<i8>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::I16s => {
                    self.render_tree_slice::<i16>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::I32s => {
                    self.render_tree_slice::<i32>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::Uints | NodeKind::U64s => {
                    self.render_tree_slice::<u64>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::U8s => {
                    self.render_tree_slice::<u8>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::U16s => {
                    self.render_tree_slice::<u16>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::U32s => {
                    self.render_tree_slice::<u32>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::F32s => {
                    self.render_tree_slice::<f32>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::F64s => {
                    self.render_tree_slice::<f64>(dst, ptr.add(val_off), val_len, is_last);
                }
                NodeKind::Strs => {
                    self.render_tree_slice::<slice_items::LiteralString>(
                        dst,
                        ptr.add(val_off),
                        val_len,
                        is_last,
                    );
                }
                NodeKind::Group => {
                    if group_is_empty(node) {
                        dst.extend_from_slice(b"{}\n");
                    } else {
                        dst.push(b'\n');
                    }
                    self.push_prefix(is_last);
                }
                NodeKind::Error => {
                    is_embed_error = false;
                    error_depth += 1;
                    dst.push(b'\n');
                    self.push_prefix(is_last);
                    self.render_tree_prefix(dst, false);
                    self.color_err_meta(dst);
                    dst.extend_from_slice(log_render::predefined_key(PREDEFINED_NAME_CONTEXT));
                    self.color_reset(dst);
                    dst.push(b'\n');
                    self.push_prefix(false);
                }
                NodeKind::ErrorEmbed => {
                    is_embed_error = true;
                    error_depth += 1;
                    dst.push(b'\n');
                    self.push_prefix(is_last);
                    self.render_tree_prefix(dst, false);
                    self.color_err_meta(dst);
                    dst.extend_from_slice(log_render::predefined_key(PREDEFINED_NAME_CONTEXT));
                    self.color_reset(dst);
                    dst.push(b'\n');
                    self.push_prefix(false);
                }
                NodeKind::ErrorStageNew | NodeKind::ErrorStageWrap | NodeKind::ErrorStageCtx => {}
                NodeKind::GroupEnd => {}
            }
        }
    }

    #[inline(always)]
    fn push_prefix(&mut self, is_last: bool) {
        let op = (1u64) << self.tree_depth;
        if is_last {
            self.tree_prefix &= !op;
            self.tree_depth += 1;
            return;
        }

        self.tree_prefix |= op;
        self.tree_depth += 1;
    }

    #[inline(always)]
    fn pop_prefix(&mut self) {
        self.tree_depth -= 1;
        let prefix = self.tree_prefix;
        let mask = !((1u64) << self.tree_depth);
        self.tree_prefix &= mask;
    }

    #[inline(always)]
    unsafe fn render_tree_slice<T>(
        &mut self,
        dst: &mut Vec<u8>,
        mut ptr: *const u8,
        len: usize,
        last: bool,
    ) where
        T: slice_items::TreeLiteral,
    {
        unsafe {
            if len < self.expand_array_since {
                dst.push(b':');
                dst.push(b' ');
                for i in 0..len as usize {
                    if i > 0 {
                        dst.push(b',');
                        dst.push(b' ');
                    }
                    ptr = T::render(self, dst, ptr);
                }
                dst.push(b'\n');
                return;
            }

            self.tree_push_prefix_tmp(last);
            dst.push(b'\n');
            for i in 0..len {
                self.render_tree_prefix(dst, i == len - 1);
                let s = self.itoa.format(i);
                dst.extend_from_slice(s.as_bytes());
                dst.push(b':');
                dst.push(b' ');
                ptr = T::render(self, dst, ptr);
                dst.push(b'\n');
            }
            self.tree_depth -= 1;
        }
    }

    #[inline(always)]
    unsafe fn render_tree_prefix(&mut self, dst: &mut Vec<u8>, last: bool) {
        self.color_link(dst);
        render_tree_prefix(dst, self.tree_prefix, self.tree_depth);
        if !last {
            dst.extend_from_slice(TREE_ITEM_INTR);
        } else {
            dst.extend_from_slice(TREE_ITEM_FIN);
        }
        self.color_reset(dst);
    }

    #[inline(always)]
    fn tree_push_prefix_tmp(&mut self, last: bool) {
        let op = (1u64) << self.tree_depth;
        if last {
            self.tree_prefix &= !op;
            self.tree_depth += 1;
            return;
        }

        self.tree_prefix |= op;
        self.tree_depth += 1;
    }
}
