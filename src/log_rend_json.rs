#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use std::slice;

#[inline(always)]
pub(crate) fn render_safe_json_string(dst: &mut Vec<u8>, src: &[u8]) {
    dst.push(b'"');
    dst.extend_from_slice(src);
    dst.push(b'"');
}

#[inline(always)]
pub(crate) unsafe fn render_safe_json_string_ptr(dst: &mut Vec<u8>, src: *const u8, len: usize) {
    unsafe {
        dst.push(b'"');
        dst.extend_from_slice(slice::from_raw_parts(src, len));
        dst.push(b'"');
    }
}

#[inline(always)]
pub(crate) unsafe fn render_json_string(dst: &mut Vec<u8>, src: &[u8]) {
    unsafe {
        json_escape_simd::escape_into(str::from_utf8_unchecked(src), dst);
    }
}

#[inline(always)]
pub(crate) unsafe fn render_json_string_ptr(dst: &mut Vec<u8>, ptr: *const u8, len: usize) {
    unsafe {
        json_escape_simd::escape_into(str::from_utf8_unchecked(slice::from_raw_parts(ptr, len)), dst);
    }
}

#[inline(always)]
pub(crate) unsafe fn render_json_string_content(dst: &mut Vec<u8>, src: &[u8]) {
    unsafe {
        json_escape_simd::escape_into(str::from_utf8_unchecked(src), dst);
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
