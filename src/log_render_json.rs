use crate::log_render::{LogRender, RenderGroupType};
use crate::log_parser_node::NodeKind;
use crate::value_kind::{PREDEFINED_NAME_CONTEXT, PREDEFINED_NAME_TEXT};
use crate::{log_parser, log_render};
use base64::Engine;
use std::slice;
use crate::log_parser_node::Node;

impl<'a> LogRender<'a> {
    pub unsafe fn render_json(&mut self, dst: &mut Vec<u8>, src: &[u8]) { unsafe {
        if self.ctx.is_empty () {
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
            if old {
                dst.push(b',');
                dst.push(b' ');
            }
            old = true;

            let node = &*(node_ptr.add(pos) as *const Node);

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
                    dst.extend_from_slice("base64.".as_bytes());
                    base64::engine::general_purpose::STANDARD
                        .encode_slice(node.val_as_slice(ptr), dst);
                    dst.push(b'"');
                }
                NodeKind::ErrTxt => {
                    render_json_string(dst, node.val_as_slice(ptr));
                }
                NodeKind::ErrTxtFragment => {
                    if !is_embed_err {
                        self.err_stack
                            .push((node.val_len as usize, node.val_off as usize));
                    }
                }
                NodeKind::ErrLoc => {
                    dst.push(b'"');
                    dst.extend_from_slice(node.key_as_slice(ptr));
                    dst.push(b':');
                    self.render_uint(dst, node.val_off as u64);
                    dst.push(b'"');
                }
                NodeKind::ErrEmbedText => {
                    embed_err_text = (node.val_len as usize, node.val_off as usize);
                }
                NodeKind::Bools => {
                    dst.push(b'[');
                    for i in 0..node.val_len {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        if *ptr.add(node.val_off as usize + i as usize) != 0 {
                            dst.extend_from_slice(b"true");
                        } else {
                            dst.extend_from_slice(b"false");
                        }
                    }
                    dst.push(b']');
                }
                NodeKind::Ints => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<i64>().read_unaligned();
                        self.render_int(dst, i64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::I8s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = *xptr.add(i) as i64;
                        self.render_int(dst, val);
                    }
                    dst.push(b']');
                }
                NodeKind::I16s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 2).cast::<i16>().read_unaligned();
                        self.render_int(dst, i16::from_le(val) as i64);
                    }
                    dst.push(b']');
                }
                NodeKind::I32s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 4).cast::<i32>().read_unaligned();
                        self.render_int(dst, i32::from_le(val) as i64);
                    }
                    dst.push(b']');
                }
                NodeKind::I64s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<i64>().read_unaligned();
                        self.render_int(dst, i64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::Uints => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<u64>().read_unaligned();
                        self.render_uint(dst, u64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::U8s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = *xptr.add(i) as u64;
                        self.render_uint(dst, val);
                    }
                    dst.push(b']');
                }
                NodeKind::U16s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 2).cast::<u16>().read_unaligned();
                        self.render_uint(dst, u16::from_le(val) as u64);
                    }
                    dst.push(b']');
                }
                NodeKind::U32s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 4 ).cast::<u32>().read_unaligned();
                        self.render_uint(dst, u32::from_le(val) as u64);
                    }
                    dst.push(b']');
                }
                NodeKind::U64s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8 ).cast::<u64>().read_unaligned();
                        self.render_uint(dst, u64::from_le(val));
                    }
                    dst.push(b']');
                }
                NodeKind::F32s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 4).cast::<u32>().read_unaligned();
                        self.render_float(dst, f32::from_bits(u32::from_le(val)) as f64);
                    }
                    dst.push(b']');
                }
                NodeKind::F64s => {
                    dst.push(b'[');
                    let xptr = ptr.add(node.val_off as usize);
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let val = xptr.add(i * 8).cast::<u64>().read_unaligned();
                        self.render_float(dst, f64::from_bits(u64::from_le(val)));
                    }
                    dst.push(b']');
                }
                NodeKind::Strs => {
                    dst.push(b'[');
                    let mut off = node.val_off as usize;
                    for i in 0..node.val_len as usize {
                        if i > 0 {
                            dst.push(b',');
                            dst.push(b' ');
                        }
                        let (length, size) = log_parser::read_uvarint(ptr.add(off));
                        let val = slice::from_raw_parts(ptr.add(off + size), length as usize);
                        render_json_string(dst, val);
                        off += size + length as usize;
                    }
                }
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
                NodeKind::ErrorStageNew | NodeKind::ErrorStageWrap | NodeKind::ErrorStageCtx => {
                    if node.child != u32::MAX {
                        self.grp_stack.push((pos, render_state));
                        render_state = RenderGroupType::ErrorStage;
                        dst.push(b'{');
                        old = false;
                        pos = node.child as usize;
                        if node.kind != NodeKind::ErrorStageCtx {
                            self.err_stack
                                .push((node.val_len as usize, node.val_off as usize));
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
                        render_json_string(dst, log_render::predefined_key(PREDEFINED_NAME_TEXT));
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
                        render_json_string(dst, log_render::predefined_key(PREDEFINED_NAME_TEXT));
                        dst.push(b':');
                        dst.push(b' ');
                        let (length, off) = embed_err_text;
                        let txt = slice::from_raw_parts(ptr.add(off), length);
                        render_json_string(dst, txt);
                        dst.push(b',');
                        dst.push(b' ');
                        dst.push(b'}');
                    }
                    _ => {}
                }
                (pos, render_state) = self.grp_stack.pop().unwrap();
            }
        }

        dst.push(b'}');
        dst.push(b'\n');
    }}
}

#[inline(always)]
pub(crate) unsafe fn render_err_ctx_header(dst: &mut Vec<u8>) { unsafe {
    render_json_string(dst, log_render::predefined_key(PREDEFINED_NAME_CONTEXT));
    dst.push(b':');
    dst.push(b' ');
    dst.push(b'{');
}}

#[inline(always)]
pub(crate) unsafe fn render_json_string(dst: &mut Vec<u8>, src: &[u8]) { unsafe {
    dst.push(b'"');
    render_json_string_content(dst, src);
    dst.push(b'"');
}}

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
