#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parser_node::NodeKind;
use crate::log_render;
use crate::log_render::{LogRender};
use crate::pointer_ext::{PointerAppender};
use crate::{log_rend_json, slice_items};

impl<'a> LogRender<'a> {
    pub unsafe fn render_json(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        unsafe {
            if self.ctx.is_empty() {
                dst.extend_from_slice(b"{}\n");
                return;
            }

            dst.push(b'{');
            let node_ptr = self.ctx.as_ptr();
            let ptr = src.as_ptr();
            let mut pos = 0 as usize;
            let mut old = false;
            let mut is_embed_err = false;
            let mut embed_err_text = (0 as usize, 0 as usize);
            let mut in_err_depth: isize = 0;

            for node in self.ctx {
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
                    NodeKind::GroupEnd => {
                        dst.push(b'}');
                        if in_err_depth == 0 {
                            continue;
                        }
                        in_err_depth -= 1;
                        if in_err_depth != 0 {
                            continue;
                        }
                        // We just closed an error. Need to render an error @text.
                        dst.extend_from_slice(b", ");
                        dst.extend_from_slice(PRETTY_JSON_TEXT_PREFIX.as_bytes());
                        if self.err_stack.len() > 0 {
                            // It was an error with a computable text.
                            for (i, (len, off)) in self.err_stack.iter().rev().enumerate() {
                                if i != 0 {
                                    dst.extend_from_slice(b": ");
                                }
                                log_rend_json::render_safe_json_string_ptr(
                                    dst,
                                    ptr.add(*off),
                                    *len,
                                );
                            }
                            self.err_stack.clear();
                        } else {
                            // It was an error with precomputed text.
                            let (len, off) = embed_err_text;
                            log_rend_json::render_safe_json_string_ptr(dst, ptr.add(off), len);
                        }
                        dst.push(b'"');
                        dst.push(b'}');
                        old = true;
                        continue;
                    }
                    _ => {
                        if old {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        old = true;

                        match node.kind {
                            NodeKind::ErrorStageNew => {
                                dst.push(b'"');
                                dst.extend_from_slice(b"NEW: ");
                                log_rend_json::render_json_string_content(
                                    dst,
                                    node.key_as_slice(ptr),
                                );
                                dst.push(b'"');
                                in_err_depth += 1;
                            }
                            NodeKind::ErrorStageWrap => {
                                dst.push(b'"');
                                dst.extend_from_slice(b"WRAP: ");
                                log_rend_json::render_json_string_content(
                                    dst,
                                    node.key_as_slice(ptr),
                                );
                                dst.push(b'"');
                                in_err_depth += 1;
                            }
                            NodeKind::ErrorStageCtx => {
                                dst.push(b'"');
                                dst.extend_from_slice(b"CTX");
                                dst.push(b'"');
                                in_err_depth += 1;
                            }
                            _ => {
                                log_rend_json::render_json_string(dst, node.key_as_slice(ptr));
                            }
                        }

                        dst.push(b':');
                        dst.push(b' ');
                    }
                }

                match node.kind {
                    NodeKind::Bool => {
                        if node.val_off != 0 {
                            dst.extend_from_slice(b"true");
                        } else {
                            dst.extend_from_slice(b"false");
                        }
                    }
                    NodeKind::Time => {
                        dst.push(b'"');
                        self.render_time(dst, node.val_as_u64() as i64, true);
                        dst.push(b'"');
                    }
                    NodeKind::Dur => {
                        dst.push(b'"');
                        self.render_go_duration(dst, node.val_as_u64(), true);
                        dst.push(b'"');
                    }
                    NodeKind::Int => {
                        self.render_int(dst, node.val_as_u64() as i64);
                    }
                    NodeKind::IVar => {
                        self.render_int(dst, node.val_as_u64() as i64);
                    }
                    NodeKind::I8 => {
                        self.render_int(dst, node.val_off as i64);
                    }
                    NodeKind::I16 => {
                        self.render_int(dst, node.val_off as i64);
                    }
                    NodeKind::I32 => {
                        self.render_int(dst, node.val_off as i64);
                    }
                    NodeKind::I64 => {
                        self.render_int(dst, node.val_as_u64() as i64);
                    }
                    NodeKind::Uint => {
                        self.render_uint(dst, node.val_as_u64());
                    }
                    NodeKind::UVar => {
                        self.render_uint(dst, node.val_as_u64());
                    }
                    NodeKind::U8 => {
                        self.render_uint(dst, node.val_off as u64);
                    }
                    NodeKind::U16 => {
                        self.render_uint(dst, node.val_off as u64);
                    }
                    NodeKind::U32 => {
                        self.render_uint(dst, node.val_off as u64);
                    }
                    NodeKind::U64 => {
                        self.render_uint(dst, node.val_as_u64());
                    }
                    NodeKind::F32 => {
                        self.render_float(dst, f32::from_bits(node.val_off) as f64);
                    }
                    NodeKind::F64 => {
                        self.render_float(dst, f64::from_bits(node.val_as_u64()));
                    }
                    NodeKind::Str => {
                        log_rend_json::render_json_string(dst, node.val_as_slice(ptr));
                    }
                    NodeKind::Bytes => {
                        dst.push(b'"');
                        base64_simd::STANDARD.encode_append(node.val_as_slice(ptr), dst);
                        dst.push(b'"');
                    }
                    NodeKind::ErrTxt => {
                        log_rend_json::render_json_string(dst, node.val_as_slice(ptr));
                    }
                    NodeKind::ErrTxtFragment => {}
                    NodeKind::ErrLoc => {
                        dst.push(b'"');
                        dst.extend_from_slice(node.key_as_slice_direct(ptr));
                        dst.push(b':');
                        self.render_uint(dst, node.val_off as u64);
                        dst.push(b'"');
                    }
                    NodeKind::ErrEmbedText => {}
                    NodeKind::Bools => self.render_json_slice::<bool>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::Ints | NodeKind::I64s => self.render_json_slice::<i64>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::I8s => self.render_json_slice::<i8>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::I16s => self.render_json_slice::<i16>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::I32s => self.render_json_slice::<i32>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::Uints | NodeKind::U64s => self.render_json_slice::<u64>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::U8s => self.render_json_slice::<u8>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::U16s => self.render_json_slice::<i16>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::U32s => self.render_json_slice::<i32>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::F32s => self.render_json_slice::<f32>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::F64s => self.render_json_slice::<f64>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::Strs => self.render_json_slice::<slice_items::LiteralString>(
                        dst,
                        ptr.add(node.val_off as usize),
                        node.val_len as usize,
                    ),
                    NodeKind::Group => {
                        dst.push(b'{');
                        old = false;
                    }
                    NodeKind::Error => {
                        in_err_depth += 1;
                        is_embed_err = false;
                        self.err_stack.clear();
                        dst.extend_from_slice(PRETTY_JSON_CONTEXT_PREFIX.as_bytes());
                        old = false;
                        continue;
                    }
                    NodeKind::ErrorEmbed => {
                        in_err_depth += 1;
                        is_embed_err = true;
                        self.err_stack.clear();
                        dst.extend_from_slice(PRETTY_JSON_CONTEXT_PREFIX.as_bytes());
                        old = false;
                        continue;
                    }
                    NodeKind::ErrorStageNew
                    | NodeKind::ErrorStageWrap
                    | NodeKind::ErrorStageCtx => {
                        dst.push(b'{');
                        old = false;
                        if node.kind != NodeKind::ErrorStageCtx {
                            self.err_stack
                                .push((node.key_len as usize, node.key_off as usize));
                        }
                        continue;
                    }
                    NodeKind::GroupEnd => {}
                }
            }

            dst.push(b'}');
            dst.push(b'\n');
        }
    }

    #[inline(always)]
    unsafe fn render_json_slice<T>(&mut self, dst: &mut Vec<u8>, mut ptr: *const u8, len: usize)
    where
        T: slice_items::JSONLiteral,
    {
        unsafe {
            dst.push(b'[');
            for i in 0..len as usize {
                if i > 0 {
                    dst.push(b',');
                    dst.push(b' ');
                }
                ptr = T::render(self, dst, ptr);
            }
            dst.push(b']');
        }
    }
}

const PRETTY_JSON_CONTEXT_PREFIX: &'static str = r#"{"@context": {"#;
const PRETTY_JSON_TEXT_PREFIX: &'static str = r#""@text": ""#;

#[cfg(test)]
mod test {
    use crate::log_rend_json::render_json_string;

    #[test]
    fn test_render_json_string() {
        let mut dst = Vec::new();
        unsafe {
            render_json_string(&mut dst, b"abcd\"ef");
            assert_eq!(dst.as_slice(), b"\"abcd\\\"ef\"");
        }
    }
}
