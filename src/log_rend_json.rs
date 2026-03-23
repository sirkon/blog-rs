use std::slice;

#[inline(always)]
pub(crate) fn render_safe_json_string(dst: &mut Vec<u8>, src: &[u8]) {
    dst.push(b'"');
    dst.extend_from_slice(src);
    dst.push(b'"');
}

#[inline(always)]
pub(crate) unsafe fn render_safe_json_string_ptr(dst: &mut Vec<u8>, src: *const u8, len: usize) {
    dst.push(b'"');
    dst.extend_from_slice(slice::from_raw_parts(src, len));
    dst.push(b'"');
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
pub(crate) unsafe fn render_json_string_ptr(dst: &mut Vec<u8>, ptr: *const u8, len: usize) {
    unsafe {
        dst.push(b'"');
        render_json_string_content_ptr(dst, ptr, len);
        dst.push(b'"');
    }
}

#[inline(always)]
pub(crate) unsafe fn render_json_string_content(dst: &mut Vec<u8>, src: &[u8]) {
    render_json_string_content_ptr(dst, src.as_ptr(), src.len());
}

#[inline(always)]
pub(crate) unsafe fn render_json_string_content_ptr(dst: &mut Vec<u8>, ptr: *const u8, len: usize) {
    if len == 0 {
        return;
    }

    // Резервируем с запасом, чтобы push() не вызывал реаллокацию внутри цикла
    dst.reserve(len + 8);

    let mut start = 0;

    for i in 0..len {
        let b = *ptr.add(i);
        if *NEEDS_ESCAPE.get_unchecked(b as usize) == 0 {
            continue;
        }

        if start < i {
            // Быстрое копирование "чистого" куска без проверок
            let current_len = dst.len();
            if current_len + i - start > dst.capacity() {
                dst.reserve(((current_len + i) - start) * 3 / 2);
            }
            std::ptr::copy_nonoverlapping(
                ptr.add(start),
                dst.as_mut_ptr().add(current_len),
                i - start,
            );
            dst.set_len(current_len + (i - start));
        }

        // Вместо тяжелого match можно использовать вторую таблицу для простых замен
        match b {
            b'"' => dst.extend_from_slice(br#"\""#),
            b'\\' => dst.extend_from_slice(br#"\\"#),
            b'\n' => dst.extend_from_slice(br#"\n"#),
            b'\r' => dst.extend_from_slice(br#"\r"#),
            b'\t' => dst.extend_from_slice(br#"\t"#),
            _ if b < 0x20 => {
                dst.extend_from_slice(br#"\u00"#);
                dst.push(*HEX.get_unchecked((b >> 4) as usize));
                dst.push(*HEX.get_unchecked((b & 0x0F) as usize));
            }
            _ => dst.push(b), // На случай других символов из таблицы
        }
        start = i + 1;
    }

    if start < len {
        let remainder = len - start;
        let current_len = dst.len();
        std::ptr::copy_nonoverlapping(ptr.add(start), dst.as_mut_ptr().add(current_len), remainder);
        dst.set_len(current_len + remainder);
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
