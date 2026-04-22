#![allow(unused)]

use crate::itoa4::{append_itoa, append_utoa};

use std::ptr::{copy_nonoverlapping, write_unaligned};
pub(crate) trait PointerAppender {
    unsafe fn append<T: AsRef<[u8]>>(self, s: T) -> *mut u8;
    unsafe fn append_escaped<T: AsRef<[u8]>>(self, s: T) -> *mut u8;
    unsafe fn append_quoted<T: AsRef<[u8]>>(self, s: T) -> *mut u8;
    unsafe fn append_escaped_ptr(self, src: *const u8, len: usize) -> *mut u8;
    unsafe fn append_byte(self, c: u8) -> *mut u8;

    unsafe fn append_utoa(self, v: u64) -> *mut u8;
    unsafe fn append_itoa(self, v: i64) -> *mut u8;

    unsafe fn append_le<T: NumericLE>(self, v: T) -> *mut u8;
    unsafe fn append_uvarint(self, v: u64) -> *mut u8;
    unsafe fn copy_uvarint(self, src: *mut u8) -> (*mut u8, *mut u8);
    unsafe fn copy_str(self, src: *mut u8) -> (*mut u8, *mut u8);
    unsafe fn copy(self, src: *mut u8, count: usize) -> (*mut u8, *mut u8);
}

use crate::log_parse::read_uvarint;
use std::slice;

pub(crate) trait NumericLE {
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8;
}

impl PointerAppender for *mut u8 {
    #[inline(always)]
    unsafe fn append<T: AsRef<[u8]>>(self, s: T) -> *mut u8 {
        unsafe {
            let bytes = s.as_ref();
            let len = bytes.len();
            copy_nonoverlapping(bytes.as_ptr(), self, len);
            self.add(len)
        }
    }

    #[inline(always)]
    unsafe fn append_escaped<T: AsRef<[u8]>>(self, s: T) -> *mut u8 {
        unsafe {
            let bytes = s.as_ref();
            let len = bytes.len();

            let mut tmp_vec = std::mem::ManuallyDrop::new(Vec::from_raw_parts(self, 0, 8 * len));

            json_escape_simd::escape_into(str::from_utf8_unchecked(bytes), &mut tmp_vec);
            self.add(tmp_vec.len())
        }
    }

    #[inline(always)]
    unsafe fn append_quoted<T: AsRef<[u8]>>(self, s: T) -> *mut u8 {
        unsafe { self.append_byte(b'"').append(s).append_byte(b'"') }
    }

    #[inline(always)]
    unsafe fn append_escaped_ptr(self, src: *const u8, len: usize) -> *mut u8 {
        unsafe {
            let bytes = slice::from_raw_parts(src, len);

            let mut tmp_vec = std::mem::ManuallyDrop::new(Vec::from_raw_parts(self, 0, 8 * len));

            json_escape_simd::escape_into(str::from_utf8_unchecked(bytes), &mut tmp_vec);
            self.add(tmp_vec.len())
        }
    }

    #[inline(always)]
    unsafe fn append_byte(self, c: u8) -> *mut u8 {
        unsafe {
            *self = c;
            self.add(1)
        }
    }

    #[inline(always)]
    unsafe fn append_utoa(self, v: u64) -> *mut u8 {
        unsafe { append_utoa(self, v) }
    }

    #[inline(always)]
    unsafe fn append_itoa(self, v: i64) -> *mut u8 {
        unsafe { append_itoa(self, v) }
    }

    #[inline(always)]
    unsafe fn append_le<T: NumericLE>(self, v: T) -> *mut u8 {
        unsafe { v.to_bytes(self) }
    }

    #[inline]
    unsafe fn append_uvarint(self, mut v: u64) -> *mut u8 {
        unsafe {
            if v < 0x80 {
                *self = v as u8;
                return self.add(1);
            }
            if v < 0x4000 {
                let vv = ((v & 0x7F) as u16 | 0x80) | ((v >> 7) << 8) as u16;
                write_unaligned::<u16>(self as *mut u16, vv);
                return self.add(2);
            }
            let mut pdst = self;
            while v >= 0x80 {
                *pdst = (v as u8) | 0x80;
                pdst = pdst.add(1);
                v >>= 7;
            }
            *pdst = v as u8;
            pdst.add(1)
        }
    }

    #[inline(always)]
    unsafe fn copy_uvarint(self, src: *mut u8) -> (*mut u8, *mut u8) {
        unsafe {
            let pdst: *mut u8 = self;
            let (_, size) = read_uvarint(src);
            copy_nonoverlapping(src, pdst, size);
            (pdst.add(size), src.add(size))
        }
    }

    #[inline(always)]
    unsafe fn copy_str(self, mut src: *mut u8) -> (*mut u8, *mut u8) {
        unsafe {
            let mut pdst: *mut u8 = self;
            let (length, size) = read_uvarint(src);
            copy_nonoverlapping(src, pdst, size);
            (pdst, src) = (pdst.add(size), src.add(size));
            copy_nonoverlapping(src, pdst, length as usize);
            (pdst.add(length as usize), src.add(length as usize))
        }
    }

    unsafe fn copy(self, src: *mut u8, count: usize) -> (*mut u8, *mut u8) {
        unsafe {
            copy_nonoverlapping(src, self, count);
            (self.add(count), src.add(count))
        }
    }
}

impl NumericLE for u8 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            *pdst = self;
            pdst.add(1)
        }
    }
}

impl NumericLE for u16 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut u16, self.to_le());
            pdst.add(2)
        }
    }
}

impl NumericLE for u32 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut u32, self.to_le());
            pdst.add(4)
        }
    }
}

impl NumericLE for u64 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut u64, self.to_le());
            pdst.add(8)
        }
    }
}

impl NumericLE for i8 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut i8, self.to_le());
            pdst.add(1)
        }
    }
}

impl NumericLE for i16 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut i16, self.to_le());
            pdst.add(2)
        }
    }
}

impl NumericLE for i32 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut i32, self.to_le());
            pdst.add(4)
        }
    }
}

impl NumericLE for i64 {
    #[inline(always)]
    unsafe fn to_bytes(self, pdst: *mut u8) -> *mut u8 {
        unsafe {
            write_unaligned(pdst as *mut i64, self.to_le());
            pdst.add(8)
        }
    }
}
