use crate::log_parser_node::Node;
use crate::log_parser_node::NodeKind;
use crate::log_render::{LogRender, RenderGroupType};
use crate::slice_items;
use crate::value_kind::{PREDEFINED_NAME_CONTEXT, PREDEFINED_NAME_TEXT};
use crate::log_render;
use std::slice;

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
            let mut render_state = RenderGroupType::Root;

            'outer: loop {
                let mut node = &*(node_ptr.add(pos) as *const Node);

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
                        if old {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        old = true;

                        match node.kind {
                            NodeKind::ErrorStageNew => {
                                dst.push(b'"');
                                dst.extend_from_slice(b"NEW: ");
                                render_json_string_content(dst, node.key_as_slice(ptr));
                                dst.push(b'"');
                            }
                            NodeKind::ErrorStageWrap => {
                                dst.push(b'"');
                                dst.extend_from_slice(b"WRAP: ");
                                render_json_string_content(dst, node.key_as_slice(ptr));
                                dst.push(b'"');
                            }
                            NodeKind::ErrorStageCtx => {
                                dst.push(b'"');
                                dst.extend_from_slice(b"CTX");
                                dst.push(b'"');
                            }
                            _ => {
                                render_json_string(dst, node.key_as_slice(ptr));
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
                        self.render_time(dst, node.val_as_u64() as i64);
                        dst.push(b'"');
                    }
                    NodeKind::Dur => {
                        dst.push(b'"');
                        self.render_go_duration(dst, node.val_as_u64());
                        dst.push(b'"');
                    }
                    NodeKind::Int => {
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
                        render_json_string(dst, node.val_as_slice(ptr));
                    }
                    NodeKind::Bytes => {
                        dst.push(b'"');
                        base64_simd::STANDARD.encode_append(node.val_as_slice(ptr), dst);
                        dst.push(b'"');
                    }
                    NodeKind::ErrTxt => {
                        render_json_string(dst, node.val_as_slice(ptr));
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
                        if node.child != u32::MAX {
                            self.grp_stack.push((pos, render_state));
                            render_state = RenderGroupType::Group;
                            dst.push(b'{');
                            old = false;
                            pos = node.child as usize;
                            continue;
                        } else {
                            dst.extend_from_slice(b"{}");
                        }
                    }
                    NodeKind::Error => {
                        if node.child != u32::MAX {
                            is_embed_err = false;
                            self.err_stack.clear();
                            self.grp_stack.push((pos, render_state));
                            render_state = RenderGroupType::Error;
                            dst.push(b'{');
                            render_json_string(
                                dst,
                                log_render::predefined_key(PREDEFINED_NAME_CONTEXT),
                            );
                            dst.push(b':');
                            dst.push(b' ');
                            dst.push(b'{');
                            old = false;
                            pos = node.child as usize;
                            continue;
                        } else {
                            dst.extend_from_slice(b"{}");
                        }
                    }
                    NodeKind::ErrorEmbed => {
                        if node.child != u32::MAX {
                            is_embed_err = true;
                            self.err_stack.clear();
                            self.grp_stack.push((pos, render_state));
                            render_state = RenderGroupType::ErrorEmbed;
                            dst.push(b'{');
                            render_json_string(
                                dst,
                                log_render::predefined_key(PREDEFINED_NAME_CONTEXT),
                            );
                            dst.push(b':');
                            dst.push(b' ');
                            dst.push(b'{');
                            old = false;
                            pos = node.child as usize;
                            continue;
                        } else {
                            dst.extend_from_slice(b"{}");
                        }
                    }
                    NodeKind::ErrorStageNew
                    | NodeKind::ErrorStageWrap
                    | NodeKind::ErrorStageCtx => {
                        if node.child != u32::MAX {
                            self.grp_stack.push((pos, render_state));
                            render_state = RenderGroupType::ErrorStage;
                            dst.push(b'{');
                            old = false;
                            pos = node.child as usize;
                            if node.kind != NodeKind::ErrorStageCtx {
                                self.err_stack
                                    .push((node.key_len as usize, node.key_off as usize));
                            }
                            continue;
                        } else {
                            dst.extend_from_slice(b"{}");
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
                    dst.push(b'}');
                    match render_state {
                        RenderGroupType::Error => {
                            dst.push(b',');
                            dst.push(b' ');
                            render_json_string(
                                dst,
                                log_render::predefined_key(PREDEFINED_NAME_TEXT),
                            );
                            dst.push(b':');
                            dst.push(b' ');
                            dst.push(b'"');
                            for (i, (len, off)) in self.err_stack.iter().rev().enumerate() {
                                if i != 0 {
                                    dst.push(b':');
                                    dst.push(b' ');
                                }
                                dst.extend_from_slice(slice::from_raw_parts(ptr.add(*off), *len));
                            }
                            dst.push(b'"');
                            dst.push(b'}');
                        }
                        RenderGroupType::ErrorEmbed => {
                            dst.push(b',');
                            dst.push(b' ');
                            render_json_string(
                                dst,
                                log_render::predefined_key(PREDEFINED_NAME_TEXT),
                            );
                            dst.push(b':');
                            dst.push(b' ');
                            let (length, off) = embed_err_text;
                            let txt = slice::from_raw_parts(ptr.add(off), length);
                            render_json_string(dst, txt);
                            dst.push(b'}');
                        }
                        _ => {}
                    }
                    (pos, render_state) = self.grp_stack.pop().unwrap();
                    node = &*(node_ptr.add(pos) as *const Node);
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
    { unsafe {
        dst.push(b'[');
        for i in 0..len as usize {
            if i > 0 {
                dst.push(b',');
                dst.push(b' ');
            }
            ptr = T::render(self, dst, ptr);
        }
        dst.push(b']');
    }}
}

#[inline(always)]
pub(crate) unsafe fn render_json_string(dst: &mut Vec<u8>, src: &[u8]) {
    unsafe {
        dst.push(b'"');
        render_json_string_content(dst, src);
        dst.push(b'"');
    }
}

#[inline(always)]
pub(crate) unsafe fn render_json_string_content(dst: &mut Vec<u8>, src: &[u8]) {
    let Some(first_escape) = src.iter().position(|&b| NEEDS_ESCAPE[b as usize] != 0) else {
        dst.extend_from_slice(src);
        return;
    };

    let mut start = 0;
    let mut i = first_escape;

    while i < src.len() {
        let b = src[i];

        if NEEDS_ESCAPE[b as usize] == 0 {
            i += 1;
            continue;
        }

        if start < i {
            dst.extend_from_slice(&src[start..i]);
        }

        match b {
            b'"' => dst.extend_from_slice(br#"\""#),
            b'\\' => dst.extend_from_slice(br#"\\"#),
            b'\n' => dst.extend_from_slice(br#"\n"#),
            b'\r' => dst.extend_from_slice(br#"\r"#),
            b'\t' => dst.extend_from_slice(br#"\t"#),
            0x08 => dst.extend_from_slice(br#"\b"#),
            0x0C => dst.extend_from_slice(br#"\f"#),
            _ => {
                // control chars: \u00XX
                dst.extend_from_slice(br#"\u00"#);
                dst.push(HEX[(b >> 4) as usize]);
                dst.push(HEX[(b & 0x0F) as usize]);
            }
        }

        i += 1;
        start = i;
    }

    if start < src.len() {
        dst.extend_from_slice(&src[start..]);
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";
const NEEDS_ESCAPE: [u8; 256] = build_needs_escape();

const fn build_needs_escape() -> [u8; 256] {
    let mut t = [0u8; 256];

    let mut i = 0;
    while i < 0x20 {
        t[i] = 1;
        i += 1;
    }

    t[b'"' as usize] = 1;
    t[b'\\' as usize] = 1;

    t
}

#[cfg(test)]
mod test {
    use crate::log_render_json::render_json_string;

    #[test]
    fn test_render_json_string() {
        let mut dst = Vec::new();
        unsafe {
            render_json_string(&mut dst, b"abcd\"ef");
            assert_eq!(dst.as_slice(), b"\"abcd\\\"ef\"");
        }
    }
}
