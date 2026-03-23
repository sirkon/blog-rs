use crate::log_parse::read_uvarint;
use crate::log_rend_json::render_json_string;
use crate::log_render::LogRender;
use std::slice;

pub(crate) trait JSONLiteral {
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8;
}

pub(crate) trait TreeLiteral {
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8;
}

#[derive(Copy, Clone)]
pub(crate) enum LiteralString {}

impl JSONLiteral for bool {
    #[inline(always)]
    unsafe fn render(_r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            if *src != 0 {
                dst.extend_from_slice(b"true");
            } else {
                dst.extend_from_slice(b"false");
            }

            src.add(1)
        }
    }
}

impl JSONLiteral for LiteralString {
    #[inline(always)]
    unsafe fn render(_r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let (length, size) = read_uvarint(src);
            render_json_string(dst, slice::from_raw_parts(src.add(size), length as usize));
            src.add(size + length as usize)
        }
    }
}

impl JSONLiteral for i8 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = src.cast::<i8>().read_unaligned();
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(1)
        }
    }
}

impl JSONLiteral for i16 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = i16::from_le(src.cast::<i16>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(2)
        }
    }
}

impl JSONLiteral for i32 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = i32::from_le(src.cast::<i32>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(4)
        }
    }
}

impl JSONLiteral for i64 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = i64::from_le(src.cast::<i64>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(8)
        }
    }
}

impl JSONLiteral for u8 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = src.cast::<u8>().read_unaligned();
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(1)
        }
    }
}

impl JSONLiteral for u16 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u16::from_le(src.cast::<u16>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(2)
        }
    }
}

impl JSONLiteral for u32 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u32::from_le(src.cast::<u32>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(4)
        }
    }
}

impl JSONLiteral for u64 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u64::from_le(src.cast::<u64>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(8)
        }
    }
}

impl JSONLiteral for f32 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u32::from_le(src.cast::<u32>().read_unaligned());
            let s = r.ryu.format(f32::from_bits(v));
            dst.extend_from_slice(s.as_bytes());
            src.add(4)
        }
    }
}

impl JSONLiteral for f64 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u64::from_le(src.cast::<u64>().read_unaligned());
            let s = r.ryu.format(f64::from_bits(v));
            dst.extend_from_slice(s.as_bytes());
            src.add(8)
        }
    }
}

impl TreeLiteral for bool {
    #[inline(always)]
    unsafe fn render(_r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            if *src != 0 {
                dst.extend_from_slice(b"true");
            } else {
                dst.extend_from_slice(b"false");
            }

            src.add(1)
        }
    }
}

impl TreeLiteral for LiteralString {
    #[inline(always)]
    unsafe fn render(_r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let (length, size) = read_uvarint(src);
            dst.extend_from_slice(slice::from_raw_parts(src.add(size), length as usize));
            src.add(size + length as usize)
        }
    }
}

impl TreeLiteral for i8 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = src.cast::<i8>().read_unaligned();
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(1)
        }
    }
}

impl TreeLiteral for i16 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = i16::from_le(src.cast::<i16>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(2)
        }
    }
}

impl TreeLiteral for i32 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = i32::from_le(src.cast::<i32>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(4)
        }
    }
}

impl TreeLiteral for i64 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = i64::from_le(src.cast::<i64>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(8)
        }
    }
}

impl TreeLiteral for u8 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = src.cast::<u8>().read_unaligned();
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(1)
        }
    }
}

impl TreeLiteral for u16 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u16::from_le(src.cast::<u16>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(2)
        }
    }
}

impl TreeLiteral for u32 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u32::from_le(src.cast::<u32>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(4)
        }
    }
}

impl TreeLiteral for u64 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u64::from_le(src.cast::<u64>().read_unaligned());
            let s = r.itoa.format(v);
            dst.extend_from_slice(s.as_bytes());
            src.add(8)
        }
    }
}

impl TreeLiteral for f32 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u32::from_le(src.cast::<u32>().read_unaligned());
            let s = r.ryu.format(f32::from_bits(v));
            dst.extend_from_slice(s.as_bytes());
            src.add(4)
        }
    }
}

impl TreeLiteral for f64 {
    #[inline(always)]
    unsafe fn render(r: &mut LogRender, dst: &mut Vec<u8>, src: *const u8) -> *const u8 {
        unsafe {
            let v = u64::from_le(src.cast::<u64>().read_unaligned());
            let s = r.ryu.format(f64::from_bits(v));
            dst.extend_from_slice(s.as_bytes());
            src.add(8)
        }
    }
}
