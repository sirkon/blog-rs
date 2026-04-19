#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parse::{read_uvarint, read_varint};
use crate::log_rend::render_go_duration;
use crate::log_transfomer_into_json::LogTransfomer;
use std::slice;
use crate::pointer_ext::PointerExt;

pub(crate) trait TransformLiteral {
    unsafe fn render(t: &mut LogTransfomer, dst: *mut u8, ptr: *const u8, off: usize)
    -> (*mut u8, usize);
}

pub(crate) struct TransformTime {}
pub(crate) struct TransformDuration {}
pub(crate) struct TransformString {}
pub(crate) struct TransformIvar {}
pub(crate) struct TransformUvar {}
pub(crate) struct TransformBytes {}

impl TransformLiteral for bool {
    #[inline(always)]
    unsafe fn render(
        _t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = ptr.add(off).cast::<u8>().read_unaligned();
            if val != 0 {
                dst = dst.append(b"true");
            } else {
                dst = dst.append(b"false");
            }

            (dst, off + 1)
        }
    }
}

impl TransformLiteral for TransformTime {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            dst = dst.append_utoa(val);

            (dst, off + 8)
        }
    }
}

impl TransformLiteral for TransformDuration {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            t.fmtbuf.clear();
            render_go_duration(&mut t.itoa, &mut t.fmtbuf, val);
            dst = dst.append_quoted(t.fmtbuf.as_slice());

            (dst, off + 8)
        }
    }
}

impl TransformLiteral for TransformIvar {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        let (value, size) = read_uvarint(ptr.add(off));
        dst = dst.append_itoa(value as i64);

        (dst, off + size)
    }
}

impl TransformLiteral for i64 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = i64::from_le(ptr.add(off).cast::<i64>().read_unaligned());
            dst = dst.append_itoa(val);

            (dst, off + 8)
        }
    }
}

impl TransformLiteral for i32 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = i32::from_le(ptr.add(off).cast::<i32>().read_unaligned());
            dst = dst.append_itoa(val as i64);

            (dst, off + 4)
        }
    }
}

impl TransformLiteral for i16 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = i16::from_le(ptr.add(off).cast::<i16>().read_unaligned());
            dst = dst.append_itoa(val as i64);

            (dst, off + 2)
        }
    }
}

impl TransformLiteral for i8 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = ptr.add(off).cast::<i8>().read_unaligned();
            dst = dst.append_itoa(val as i64);

            (dst, off + 1)
        }
    }
}

impl TransformLiteral for TransformUvar {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        let (value, size) = read_uvarint(ptr.add(off));
        dst = dst.append_utoa(value);

        (dst, off + size)
    }
}

impl TransformLiteral for u64 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            dst = dst.append_utoa(val);

            (dst, off + 8)
        }
    }
}

impl TransformLiteral for u32 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
            dst = dst.append_utoa(val as u64);

            (dst, off + 4)
        }
    }
}

impl TransformLiteral for u16 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u16::from_le(ptr.add(off).cast::<u16>().read_unaligned());
            dst = dst.append_utoa(val as u64);

            (dst, off + 2)
        }
    }
}

impl TransformLiteral for u8 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = ptr.add(off).cast::<u8>().read_unaligned();
            dst = dst.append_utoa(val as u64);

            (dst, off + 1)
        }
    }
}

impl TransformLiteral for f64 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            let fval = f64::from_bits(val);
            if !f64::is_nan(fval) {
                let s = t.ryu.format(fval);
                dst = dst.append(s);
            } else {
                dst = dst.append_quoted(b"NaN");
            }

            (dst, off + 8)
        }
    }
}

impl TransformLiteral for f32 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let val = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
            let fval = f32::from_bits(val);
            if !f32::is_nan(fval) {
                let s = t.ryu.format(fval);
                dst = dst.append(s);
            } else {
                dst = dst.append_quoted(b"NaN");
            }

            (dst, off + 4)
        }
    }
}

impl TransformLiteral for TransformString {
    #[inline(always)]
    unsafe fn render(
        _t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let (length, size) = read_uvarint(ptr.add(off));
            dst = dst.append_escaped_ptr(ptr.add(off+size), length as usize);

            (dst, off + size + length as usize)
        }
    }
}

impl TransformLiteral for TransformBytes {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        mut dst: *mut u8,
        ptr: *const u8,
        off: usize,
    ) -> (*mut u8, usize) {
        unsafe {
            let (length, size) = read_uvarint(ptr.add(off));
            t.fmtbuf.clear();
            base64_simd::STANDARD.encode_append(
                slice::from_raw_parts(ptr.add(off + size), length as usize),
                &mut t.fmtbuf,
            );
            dst = dst.append_quoted(t.fmtbuf.as_slice());

            (dst, off + size + length as usize)
        }
    }
}
