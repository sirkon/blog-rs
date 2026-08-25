#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::crc32custom::fast_crc32c;
use core::arch::x86_64::{_pext_u64, _tzcnt_u64};
use std::ptr::read_unaligned;
use std::{ptr, slice};

/// Parses input source for logs and splits it into (log_data, rest of the data) on happy path.
/// The buffer must be long enough to have 8 bytes in it. Meaning, leave at last 8 bytes
/// unoccupied, although I would propose more: 64 bytes or even 128 or even more for widish
/// vectors or even integrated GPUs.
#[inline(always)]
pub(crate) unsafe fn log_parse_header(
    src: &[u8],
    max_log_size: usize,
) -> Result<(&[u8], &[u8]), ErrorLogParse> {
    unsafe {
        if src.len() < 7 {
            return Err(ErrorLogParse::NoHeader);
        }

        let ptr = src.as_ptr() as *mut u8;
        let header = read_unaligned::<u64>(ptr as *mut u64);
        if header as u8 != 0xFF {
            return Err(ErrorLogParse::StartMarkerInvalid(header as u8));
        }
        if (header >> 40) as u8 != 0xFE {
            return Err(ErrorLogParse::TailMarkerInvalid((header >> 40) as u8));
        }

        let record_crc = (header >> 8) as u32;
        let (length, size) = read_uvarint_safe(ptr.add(6), src.len() - 6);
        if size == usize::MAX {
            return Err(ErrorLogParse::RecordLengthInvalid);
        }
        if length as usize > max_log_size {
            return Err(ErrorLogParse::RecordLengthTooLarge);
        }
        if 6 + size + length as usize > src.len() {
            return Err(ErrorLogParse::RecordNeedMore);
        }
        let off = 6 + size;
        let record = slice::from_raw_parts(ptr.add(off), length as usize);

        let check = fast_crc32c(0u32, record);
        if check != record_crc {
            return Err(ErrorLogParse::RecordCRCMismatch);
        }
        let record_size = 6 + size + length as usize;

        Ok((
            record,
            slice::from_raw_parts(ptr.add(record_size), src.len() - record_size),
        ))
    }
}

/// Log parsing error states.
#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub(crate) enum ErrorLogParse {
    /// Missing this
    ///
    /// | 0xFF | CRC32 | 0xFE |
    /// |------|-------|------|
    ///
    /// 5 bytes header.
    NoHeader,
    /// Log data must start with 0xFF, got something different.
    StartMarkerInvalid(u8),
    /// Log data must have 0xFE byte on its sixth position, right after CRC32.
    TailMarkerInvalid(u8),
    /// Record length in uvarint encoding is either cut or something is off with it.
    RecordLengthInvalid,
    /// Record length is out of limit.
    RecordLengthTooLarge,
    /// The rest of data does not have the entire record. Need to read more.
    RecordNeedMore,
    /// Record data does not match the CRC.
    RecordCRCMismatch,
    /// Record data has unsupported version.
    RecordVersionNotSupported(u16),
    /// Record data has unsupported level.
    RecordLevelNotSupported(u8),
    /// Context node type is unknown.
    RecordContextNodeType(u8),
    /// Context predefined key is unkown
    RecordContextNodePredefinedKeyUnknown(u32),
}


use std::arch::x86_64::*;

#[inline(always)]
unsafe fn pair(v: u64) -> (u64, i32) {
    if v & 0x80 == 0 {
        return (v & 0x7F, 1);
    }

    if v & 0x8000 == 0 {
        return ((v & 0x7F) | ((v >> 8 & 0x7F) << 7), 2);
    }

    return ((v & 0x7F) | ((v >> 8 & 0x7F) << 7), 0);
}

#[inline(never)]
#[cold]
unsafe fn panic_overload() -> ! {
    panic!("overflow decoding uvarint");
}

#[inline(always)]
pub(crate) unsafe fn read_uvarint(src: *const u8) -> (u64, usize) {
    let (v, ptr) = read_uvarint_ddd(src);
    (v, ptr.offset_from(src) as usize)
}

#[inline(always)]
unsafe fn read_uvarint_ddd(src: *const u8) -> (u64, *const u8) {
    let v = *(src as *const u64);

    // 1 байт
    if v & 0x80 == 0 {
        return (v & 127, src.add(1));
    }

    // 2 байта
    if v & 0x8000 == 0 {
        return (v & 127 | ((v >> 8) & 127) << 7, src.add(2));
    }

    let mut res = v & 127 | ((v >> 8) & 127) << 7;

    // 3-4 байта
    let (vv, off) = pair(v >> 16);
    res |= vv << 14;
    if off > 0 {
        return (res, src.add(2 + off as usize));
    }

    // 5-6 байт
    let (vv, off) = pair(v >> 32);
    res |= vv << 28;
    if off > 0 {
        return (res, src.add(4 + off as usize));
    }

    // 7-8 байт
    let (vv, off) = pair(v >> 48);
    res |= vv << 42;
    if off > 0 {
        return (res, src.add(6 + off as usize));
    }

    // 9-10 байт (читаем из памяти)
    let v = *(src.add(8) as *const u64);
    let (vv, off) = pair(v & 0xFFFF);
    res |= vv << 56;
    if off > 0 {
        return (res, src.add(8 + off as usize));
    }

    panic_overload();
    (0, src)
}

// #[inline(always)]
// pub(crate) unsafe fn read_uvarint(ptr: *const u8) -> (u64, usize) {
//     unsafe { read_uvarint_pext(ptr) }
// }

/// Decodes a single LEB128 varint using the SFVInt technique (PEXT):
/// - PEXT over 0x80.. gathers byte MSBs; trailing run of set bits equals the
///   number of continuation bytes, i.e. len-1.
/// - PEXT over 0x7f.. gathers the 7 payload bits of each byte into a packed
///   little-endian stream.
/// - ((1 << (7*len)) - 1) trims padding beyond the actual payload.
///
/// Data is assumed valid: at most 10 bytes, no overflow checking. The first 8
/// bytes of the word are read unconditionally, so the caller must guarantee at
/// least 8 readable bytes past `ptr`. Requires BMI2 (target-cpu=native).
#[inline(always)]
unsafe fn read_uvarint_pext(ptr: *const u8) -> (u64, usize) {
    let word = (ptr as *const u64).read_unaligned();

    // Находим длину через PEXT
    let cont = _pext_u64(word, 0x8080_8080_8080_8080u64);
    let len_minus_1 = _tzcnt_u64(!cont) as usize;
    let len = len_minus_1 + 1;

    // Парсим payload
    let packed = _pext_u64(word, 0x7f7f_7f7f_7f7f_7f7fu64);
    let mask = (1u64 << (7 * len)) - 1;
    let mut result = packed & mask;
    let mut total_len = len;

    // Если len == 10 (т.е. len_minus_1 == 9) - читаем еще байты
    // Используем беззнаковое сравнение для branchless
    let need_more = (len_minus_1 >= 8) as usize;
    if need_more != 0 {
        let b9 = *ptr.add(8) as u64;
        result |= (b9 & 0x7f) << 56;
        total_len = 9;

        if b9 & 0x80 != 0 {
            let b10 = *ptr.add(9) as u64;
            result |= (b10 & 0x7f) << 63;
            total_len = 10;
        }
    }

    (result, total_len)
}


#[inline]
#[allow(unused)]
pub(crate) fn write_uvarint(mut value: u64, buf: &mut Vec<u8>) {
    while value >= 0x80 {
        buf.push((value as u8) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
}

#[inline(always)]
#[allow(unused)]
pub(crate) unsafe fn read_varint(ptr: *const u8) -> (i64, usize) {
    unsafe {
        let (uval, len) = read_uvarint(ptr);
        // zigzag decode: (uval >> 1) ^ -(uval & 1)
        let val = ((uval >> 1) as i64) ^ (-((uval & 1) as i64));
        (val, len)
    }
}

#[inline]
#[allow(unused)]
pub(crate) fn write_varint(value: i64, buf: &mut Vec<u8>) {
    // zigzag encode: (value << 1) ^ (value >> 63)
    let uval = ((value << 1) ^ (value >> 63)) as u64;
    write_uvarint(uval, buf);
}

#[inline(always)]
#[allow(unused)]
pub(crate) unsafe fn read_uvarint_safe(ptr: *const u8, mut lim: usize) -> (u64, usize) {
    unsafe {
        let mut res = 0u64;
        let mut i = 0;
        if lim > 10 {
            lim = 10;
        }
        loop {
            if i >= lim {
                return (res, usize::MAX);
            }
            let b = *ptr.add(i);
            res |= ((b & 0x7F) as u64) << (i * 7);
            i += 1;
            if b & 0x80 == 0 {
                break;
            }
        }

        (res, i)
    }
}
