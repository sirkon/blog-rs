#![allow(unused)]
use crate::pointer_ext::PointerAppender;
use std::ptr::write_unaligned;
use include_bytes_aligned::include_bytes_aligned;

static ITOA_TABLE: &[u8; 200] = include_bytes_aligned!(2, "itoa2.bin");
const N: u64 = 100;

#[inline(always)]
unsafe fn append_utoa_short(dst: *mut u8, v: u64) -> *mut u8 {
    unsafe {
        let src = ITOA_TABLE.as_ptr() as *const u16;
        let vv = *src.add(v as usize);

        if v < 10 {
            let x = vv >> 8;
            (dst as *mut u16).write_unaligned(x);
            return dst.add(1);
        }

        write_unaligned(dst as *mut u16, vv);
        dst.add(2)
    }
}

pub(crate) unsafe fn append_utoa(mut dst: *mut u8, v: u64) -> *mut u8 {
    unsafe {
        let src = ITOA_TABLE.as_ptr() as *const u16;

        if v < N {
            return append_utoa_short(dst, v);
        }

        let hi1 = v / N;
        let lo1 = v % N;

        let hi2 = hi1 / N;
        let lo2 = hi1 % N;
        if hi2 == 0 {
            dst = append_utoa_short(dst, lo2);
            let x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi3 = hi2 / N;
        let lo3 = hi2 % N;
        if hi3 == 0 {
            dst = append_utoa_short(dst, lo3);
            let mut x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi4 = hi3 / N;
        let lo4 = hi3 % N;
        if hi4 == 0 {
            dst = append_utoa_short(dst, lo4);
            let mut x = *src.add(lo3 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi5 = hi4 / N;
        let lo5 = hi4 % N;
        if hi5 == 0 {
            dst = append_utoa_short(dst, lo5);
            let mut x = *src.add(lo4 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo3 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi6 = hi5 / N;
        let lo6 = hi6 % N;
        if hi6 == 0 {
            dst = append_utoa_short(dst, lo6);
            let mut x = *src.add(lo5 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo4 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo3 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi7 = hi6 / N;
        let lo7 = hi7 % N;
        if hi7 == 0 {
            dst = append_utoa_short(dst, lo7);
            let mut x = *src.add(lo6 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo5 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo4 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo3 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi8 = hi7 / N;
        let lo8 = hi8 % N;
        if hi8 == 0 {
            dst = append_utoa_short(dst, lo8);
            let mut x = *src.add(lo7 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo6 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo5 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo4 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo3 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let hi9 = hi8 / N;
        let lo9 = hi8 % N;
        if hi9 == 0 {
            dst = append_utoa_short(dst, lo9);
            let mut x = *src.add(lo8 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo7 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo6 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo5 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo4 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo3 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo2 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            dst = dst.add(2);
            x = *src.add(lo1 as usize);
            write_unaligned::<u16>(dst as *mut u16, x);
            return dst.add(2);
        }

        let lo10 = hi9 / N;
        dst = append_utoa_short(dst, lo10);
        let mut x = *src.add(lo9 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo8 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo7 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo6 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo5 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo4 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo3 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo2 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        dst = dst.add(2);
        x = *src.add(lo1 as usize);
        write_unaligned::<u16>(dst as *mut u16, x);
        return dst.add(2);
    }
}

#[inline(always)]
pub(crate) unsafe fn append_itoa(mut dst: *mut u8, v: i64) -> *mut u8 {
    unsafe {
        if v < 0 {
            dst = dst.append_byte(b'-');
        }
        let vv = v.unsigned_abs();
        append_utoa(dst, vv)
    }
}

#[cfg(test)]
mod test {
    use crate::itoa4::{append_itoa, append_utoa};

    #[test]
    fn test_utoa() {
        #[derive(Copy, Clone, Debug)]
        struct TestSample {
            value: u64,
            want: &'static str,
        }

        let tests = [
            0,
            1,
            12,
            123,
            1234,
            1234_5,
            1234_5678_9,
            1234_5678_9012_3,
            1234_5678_9012_3456_7u64,
        ];
        let mut buf: [u8; 64] = [0; 64];
        for x in tests {
            let got = unsafe {
                let mut ptr = buf.as_mut_ptr();
                let orig = ptr;
                ptr = append_utoa(ptr, x);
                str::from_utf8_unchecked(&buf[0..ptr.offset_from(orig) as usize])
            };
            let expected = format!("{}", x);
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn test_itoa() {
        const X: i64 = -101;

        let mut buf: [u8; 64] = [0; 64];
        let mut sample_buf = itoa::Buffer::new();
        let got = unsafe {
            let mut ptr = buf.as_mut_ptr();
            let orig = ptr;
            ptr = append_itoa(ptr, X);
            str::from_utf8_unchecked(&buf[0..ptr.offset_from(orig) as usize])
        };
        let expected = sample_buf.format(X);
        assert_eq!(got, expected);
    }
}
