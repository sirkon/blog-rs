use std::ptr::copy_nonoverlapping;
use std::slice;
use crate::itoa2::{append_itoa, append_utoa};

pub(crate) trait PointerExt {
    unsafe fn append<T: AsRef<[u8]>>(self, s: T) -> *mut u8;
    unsafe fn append_escaped<T: AsRef<[u8]>>(self, s: T) -> *mut u8;
    unsafe fn append_quoted<T: AsRef<[u8]>>(self, s: T) -> *mut u8;
    unsafe fn append_escaped_ptr(self, src: *const u8, len: usize) -> *mut u8;
    unsafe fn append_byte(self, c: u8) -> *mut u8;
    unsafe fn append_ptr(self, src: *const u8, len: usize) -> *mut u8;
    unsafe fn append_utoa(self, v :u64) -> *mut u8;
    unsafe fn append_itoa(self, v :i64) -> *mut u8;
}

impl PointerExt for *mut u8 {
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
        self.append_byte(b'"').append(s).append_byte(b'"')
    }

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

    unsafe fn append_ptr(self, src: *const u8, len: usize) -> *mut u8 {
        unsafe {
            copy_nonoverlapping(src, self, len);
            self.add(len)
        }
    }

    unsafe fn append_utoa(self, v: u64) -> *mut u8 {
        unsafe {
            append_utoa(self,v)
        }
    }

    unsafe fn append_itoa(self, v: i64) -> *mut u8 {
        unsafe {
            append_itoa(self, v)
        }
    }
}