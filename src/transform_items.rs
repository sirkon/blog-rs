use crate::log_parse::read_uvarint;
use crate::log_rend::{render_go_duration, render_time};
use crate::log_rend_json::{render_json_string, render_json_string_ptr, render_safe_json_string};
use crate::log_transfomer_into_json::LogTransfomer;
use std::slice;

pub(crate) trait TransformLiteral {
    unsafe fn render(t: &mut LogTransfomer, dst: &mut Vec<u8>, ptr: *const u8, off: usize)
        -> usize;
}

pub(crate) struct TransformTime {}
pub(crate) struct TransformDuration {}
pub(crate) struct TransformString {}

pub(crate) struct TransformBytes {}

impl TransformLiteral for bool {
    #[inline(always)]
    unsafe fn render(
        _t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = ptr.add(off).cast::<u8>().read_unaligned();
            if val != 0 {
                render_safe_json_string(dst, b"true");
            } else {
                render_json_string(dst, b"false");
            }

            off + 1
        }
    }
}

impl TransformLiteral for TransformTime {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            dst.push(b'"');
            render_time(&mut t.itoa, dst, val as i64);
            dst.push(b'"');

            off + 8
        }
    }
}

impl TransformLiteral for TransformDuration {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            dst.push(b'"');
            render_go_duration(&mut t.itoa, dst, val);
            dst.push(b'"');

            off + 8
        }
    }
}

impl TransformLiteral for i64 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = i64::from_le(ptr.add(off).cast::<i64>().read_unaligned());
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 8
        }
    }
}

impl TransformLiteral for i32 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = i32::from_le(ptr.add(off).cast::<i32>().read_unaligned());
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 4
        }
    }
}

impl TransformLiteral for i16 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = i16::from_le(ptr.add(off).cast::<i16>().read_unaligned());
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 2
        }
    }
}

impl TransformLiteral for i8 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = ptr.add(off).cast::<i8>().read_unaligned();
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 1
        }
    }
}

impl TransformLiteral for u64 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 8
        }
    }
}

impl TransformLiteral for u32 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 4
        }
    }
}

impl TransformLiteral for u16 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u16::from_le(ptr.add(off).cast::<u16>().read_unaligned());
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 2
        }
    }
}

impl TransformLiteral for u8 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = ptr.add(off).cast::<u8>().read_unaligned();
            let s = t.itoa.format(val);
            dst.extend_from_slice(s.as_bytes());

            off + 1
        }
    }
}

impl TransformLiteral for f64 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u64::from_le(ptr.add(off).cast::<u64>().read_unaligned());
            let fval = f64::from_bits(val);
            if !f64::is_nan(fval) {
                let s = t.ryu.format(fval);
                dst.extend_from_slice(s.as_bytes());
            } else {
                render_safe_json_string(dst, b"NaN");
            }

            off + 8
        }
    }
}

impl TransformLiteral for f32 {
    #[inline(always)]
    unsafe fn render(
        t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let val = u32::from_le(ptr.add(off).cast::<u32>().read_unaligned());
            let fval = f32::from_bits(val);
            if !f32::is_nan(fval) {
                let s = t.ryu.format(fval);
                dst.extend_from_slice(s.as_bytes());
            } else {
                render_safe_json_string(dst, b"NaN");
            }

            off + 4
        }
    }
}

impl TransformLiteral for TransformString {
    #[inline(always)]
    unsafe fn render(
        _t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let (length, size) = read_uvarint(ptr.add(off));
            render_json_string_ptr(dst, ptr.add(off + size), length as usize);

            off + size + length as usize
        }
    }
}

impl TransformLiteral for TransformBytes {
    #[inline(always)]
    unsafe fn render(
        _t: &mut LogTransfomer,
        dst: &mut Vec<u8>,
        ptr: *const u8,
        off: usize,
    ) -> usize {
        unsafe {
            let (length, size) = read_uvarint(ptr.add(off));
            dst.push(b'"');
            base64_simd::STANDARD.encode_append(
                slice::from_raw_parts(ptr.add(off + size), length as usize),
                dst,
            );
            dst.push(b'"');

            off + size + length as usize
        }
    }
}
