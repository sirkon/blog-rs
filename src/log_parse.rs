#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::crc32custom::fast_crc32c;
use std::ptr::read_unaligned;
use std::slice;

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

#[inline(always)]
pub(crate) unsafe fn read_uvarint(ptr: *const u8) -> (u64, usize) {
    unsafe {
        let b = *ptr;
        if b < 0x80 {
            return (u64::from(b), 1);
        }

        let c = *ptr.add(1);
        if b < 0x80 {
            return (u64::from(c) << 7 + u64::from(b), 2);
        }

        let mut res = 0u64;
        let mut i = 0;
        loop {
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
