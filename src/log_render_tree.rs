use crate::log_parser_node::{Node, NodeKind};
use crate::log_render::{LogRender, RenderGroupType};
use crate::log_render_tree_prefixes::render_tree_prefix;
use crate::slice_items::LiteralString;
use crate::value_kind::{PREDEFINED_NAME_CONTEXT, PREDEFINED_NAME_TEXT};
use crate::{log_parser, log_render, slice_items};
use std::slice;

const TREE_ITEM_INTR: &'static [u8; 7] = b"\xE2\x94\x9C\xE2\x94\x80 ";
const TREE_ITEM_FIN: &'static [u8; 7] = b"\xE2\x94\x94\xE2\x94\x80 ";

impl<'a> LogRender<'a> {
    pub(crate) unsafe fn render_tree(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        unsafe {
            if self.ctx.is_empty() {
                dst.extend_from_slice(b"{}\n");
                return;
            }
            dst.push(b'\n');

            let node_ptr = self.ctx.as_ptr();
            let ptr = src.as_ptr();
            let mut pos = 0 as usize;
            let mut old = false;
            let mut is_embed_err = false;
            let mut embed_err_text = (0 as usize, 0 as usize);
            let mut render_state = RenderGroupType::Root;

            'outer: loop {
                let mut node = &*(node_ptr.add(pos) as *const Node);
                let val_off = node.val_off as usize;
                let val_len = node.val_len as usize;

                match node.kind {
                    NodeKind::ErrEmbedText => {
                        embed_err_text = (node.val_len as usize, node.val_off as usize);
                    }
                    NodeKind::ErrTxtFragment => {
                        if !is_embed_err {
                            self.err_stack
                                .push((node.key_len as usize, node.key_off as usize));
                        }
                    }
                    _ => {
                        self.render_tree_prefix(dst, node.next);

                        match node.kind {
                            NodeKind::ErrorStageNew => {
                                dst.extend_from_slice(b"NEW: ");
                                dst.extend_from_slice(node.key_as_slice(ptr));
                                self.tree_push_prefix(node.next);
                            }
                            NodeKind::ErrorStageWrap => {
                                dst.extend_from_slice(b"WRAP: ");
                                dst.extend_from_slice(node.key_as_slice(ptr));
                                self.tree_push_prefix(node.next);
                            }
                            NodeKind::ErrorStageCtx => {
                                dst.extend_from_slice(b"CTX");
                                self.tree_push_prefix(node.next);
                            }
                            NodeKind::Group => {
                                dst.extend_from_slice(node.key_as_slice(ptr));
                                self.tree_push_prefix(node.next);
                            }
                            _ => {
                                dst.extend_from_slice(node.key_as_slice(ptr));
                            }
                        }

                    }
                }

                match node.kind {
                    NodeKind::Bool => {
                        dst.push(b':');
                        dst.push(b' ');
                        if node.val_off != 0 {
                            dst.extend_from_slice(b"true");
                        } else {
                            dst.extend_from_slice(b"false");
                        }
                        dst.push(b'\n');
                    }
                    NodeKind::Time => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_time(dst, node.val_as_u64() as i64);
                        dst.push(b'\n');
                    }
                    NodeKind::Dur => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_go_duration(dst, node.val_as_u64());
                        dst.push(b'\n');
                    }
                    NodeKind::Int => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_int(dst, node.val_as_u64() as i64);
                        dst.push(b'\n');
                    }
                    NodeKind::I8 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_int(dst, node.val_off as i64);
                        dst.push(b'\n');
                    }
                    NodeKind::I16 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_int(dst, node.val_off as i64);
                        dst.push(b'\n');
                    }
                    NodeKind::I32 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_int(dst, node.val_off as i64);
                        dst.push(b'\n');
                    }
                    NodeKind::I64 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_int(dst, node.val_as_u64() as i64);
                        dst.push(b'\n');
                    }
                    NodeKind::Uint => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_uint(dst, node.val_as_u64());
                        dst.push(b'\n');
                    }
                    NodeKind::U8 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_uint(dst, node.val_off as u64);
                        dst.push(b'\n');
                    }
                    NodeKind::U16 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_uint(dst, node.val_off as u64);
                        dst.push(b'\n');
                    }
                    NodeKind::U32 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_uint(dst, node.val_off as u64);
                        dst.push(b'\n');
                    }
                    NodeKind::U64 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_uint(dst, node.val_as_u64());
                        dst.push(b'\n');
                    }
                    NodeKind::F32 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_float(dst, f32::from_bits(node.val_off) as f64);
                        dst.push(b'\n');
                    }
                    NodeKind::F64 => {
                        dst.push(b':');
                        dst.push(b' ');
                        self.render_float(dst, f64::from_bits(node.val_as_u64()));
                        dst.push(b'\n');
                    }
                    NodeKind::Str => {
                        dst.push(b':');
                        dst.push(b' ');
                        dst.extend_from_slice(node.val_as_slice(ptr));
                        dst.push(b'\n');
                    }
                    NodeKind::Bytes => {
                        dst.push(b':');
                        dst.push(b' ');
                        dst.extend_from_slice(b"base64.");
                        base64_simd::STANDARD.encode_append(node.val_as_slice(ptr), dst);
                        dst.push(b'\n');
                    }
                    NodeKind::ErrTxt => {
                        dst.push(b':');
                        dst.push(b' ');
                        dst.extend_from_slice(node.val_as_slice(ptr));
                        dst.push(b'\n');
                    }
                    NodeKind::ErrTxtFragment => {}
                    NodeKind::ErrLoc => {
                        dst.push(b':');
                        dst.push(b' ');
                        dst.extend_from_slice(node.key_as_slice_direct(ptr));
                        dst.push(b':');
                        self.render_uint(dst, node.val_off as u64);
                        dst.push(b'\n');
                    }
                    NodeKind::ErrEmbedText => {}
                    NodeKind::Bools => {
                        self.render_tree_slice::<bool>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::Ints | NodeKind::I64s => {
                        self.render_tree_slice::<i64>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::I8s => {
                        self.render_tree_slice::<i8>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::I16s => {
                        self.render_tree_slice::<i16>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::I32s => {
                        self.render_tree_slice::<i32>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::Uints | NodeKind::U64s => {
                        self.render_tree_slice::<u64>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::U8s => {
                        self.render_tree_slice::<u8>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::U16s => {
                        self.render_tree_slice::<u16>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::U32s => {
                        self.render_tree_slice::<u32>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::F32s => {
                        self.render_tree_slice::<f32>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::F64s => {
                        self.render_tree_slice::<f64>(dst, ptr.add(val_off), val_len, node.next);
                    }
                    NodeKind::Strs => {
                        self.render_tree_slice::<LiteralString>(
                            dst,
                            ptr.add(val_off),
                            val_len,
                            node.next,
                        );
                    }
                    NodeKind::Group => {
                        if node.child != u32::MAX {
                            self.grp_stack.push((pos, render_state));
                            render_state = RenderGroupType::Group;
                            dst.push(b'\n');
                            old = false;
                            pos = node.child as usize;
                            continue; // ???
                        } else {
                            dst.push(b':');
                            dst.push(b' ');
                            self.tree_pop_prefix();
                            dst.extend_from_slice(b"{}\n");
                        }
                    }
                    NodeKind::Error => {
                        if node.child != u32::MAX {
                            is_embed_err = false;
                            self.err_stack.clear();
                            self.grp_stack.push((pos, render_state));
                            self.tree_push_prefix(node.next);
                            render_state = RenderGroupType::Error;
                            dst.push(b'\n');
                            self.render_tree_prefix(dst, 1);
                            dst.extend_from_slice(log_render::predefined_key(
                                PREDEFINED_NAME_CONTEXT,
                            ));
                            dst.push(b'\n');
                            self.tree_push_prefix(1);
                            old = false;
                            pos = node.child as usize;
                            continue;
                        } else {
                            dst.push(b':');
                            dst.push(b' ');
                            self.tree_pop_prefix();
                            dst.extend_from_slice(b"{}\n");
                        }
                    }
                    NodeKind::ErrorEmbed => {
                        if node.child != u32::MAX {
                            is_embed_err = true;
                            self.err_stack.clear();
                            self.grp_stack.push((pos, render_state));
                            self.tree_push_prefix(node.next);
                            render_state = RenderGroupType::ErrorEmbed;
                            dst.push(b'\n');
                            self.render_tree_prefix(dst, 1);
                            dst.extend_from_slice(log_render::predefined_key(
                                PREDEFINED_NAME_CONTEXT,
                            ));
                            dst.push(b'\n');
                            self.tree_push_prefix(1);
                            old = false;
                            pos = node.child as usize;
                            continue;
                        } else {
                            dst.push(b':');
                            dst.push(b' ');
                            self.tree_pop_prefix();
                            dst.extend_from_slice(b"{}\n");
                        }
                    }
                    NodeKind::ErrorStageNew
                    | NodeKind::ErrorStageWrap
                    | NodeKind::ErrorStageCtx => {
                        if node.child != u32::MAX {
                            self.grp_stack.push((pos, render_state));
                            render_state = RenderGroupType::ErrorStage;
                            dst.push(b'\n');
                            old = false;
                            pos = node.child as usize;
                            if node.kind != NodeKind::ErrorStageCtx {
                                self.err_stack
                                    .push((node.key_len as usize, node.key_off as usize));
                            }
                            continue;
                        } else {
                            dst.extend_from_slice(b"{}\n");
                        }
                    }
                }

                loop {
                    pos = node.next as usize;
                    if pos != 0 {
                        continue 'outer;
                    }

                    if self.grp_stack.is_empty() {
                        break 'outer;
                    }
                    self.tree_pop_prefix();
                    match render_state {
                        RenderGroupType::Error => {
                            self.render_tree_prefix(dst, node.next);
                            dst.extend_from_slice(log_render::predefined_key(PREDEFINED_NAME_TEXT));
                            dst.push(b':');
                            dst.push(b' ');
                            for (i, (len, off)) in self.err_stack.iter().rev().enumerate() {
                                if i != 0 {
                                    dst.push(b':');
                                    dst.push(b' ');
                                }
                                dst.extend_from_slice(slice::from_raw_parts(ptr.add(*off), *len));
                            }
                            dst.push(b'\n');
                            self.tree_pop_prefix();
                        }
                        RenderGroupType::ErrorEmbed => {
                            self.render_tree_prefix(dst, node.next);
                            dst.extend_from_slice(log_render::predefined_key(PREDEFINED_NAME_TEXT));
                            dst.push(b':');
                            dst.push(b' ');
                            let (length, off) = embed_err_text;
                            let txt = slice::from_raw_parts(ptr.add(off), length);
                            dst.extend_from_slice(txt);
                            dst.push(b'\n');
                            self.tree_pop_prefix();
                        }
                        _ => {}
                    }
                    (pos, render_state) = self.grp_stack.pop().unwrap();
                    node = &*(node_ptr.add(pos) as *const Node);
                }
            }
        }
    }

    #[inline(always)]
    unsafe fn render_tree_slice<T>(
        &mut self,
        dst: &mut Vec<u8>,
        mut ptr: *const u8,
        len: usize,
        next: u32,
    ) where
        T: slice_items::TreeLiteral,
    {
        if len < 10 {
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

        self.tree_push_prefix_tmp(next);
        dst.push(b'\n');
        for i in 0..len {
            self.render_tree_prefix(dst, (len - i - 1) as u32);
            ptr = T::render(self, dst, ptr);
            dst.push(b'\n');
        }
        self.tree_depth -= 1;
    }

    #[inline(always)]
    unsafe fn render_tree_prefix(&self, dst: &mut Vec<u8>, next: u32) {
        render_tree_prefix(dst, self.tree_prefix, self.tree_depth);
        if next != 0 {
            dst.extend_from_slice(TREE_ITEM_INTR);
        } else {
            dst.extend_from_slice(TREE_ITEM_FIN);
        }
    }

    #[inline(always)]
    fn tree_push_prefix_tmp(&mut self, next: u32) {
        let op = (1u64) << self.tree_depth;
        if next == 0 {
            self.tree_prefix &= !op;
            self.tree_depth += 1;
            return;
        }

        self.tree_prefix |= op;
        self.tree_depth += 1;
    }

    #[inline(always)]
    fn tree_push_prefix(&mut self, next: u32) {
        self.tree_stack.push((self.tree_prefix, self.tree_depth));
        self.tree_push_prefix_tmp(next);
    }

    #[inline(always)]
    pub(crate) fn tree_pop_prefix(&mut self) {
        (self.tree_prefix, self.tree_depth) = self.tree_stack.pop().unwrap();
    }
}
