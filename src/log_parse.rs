use std::slice;

/// Parses input source for logs and splits it into (log_data, rest of the data) on happy path.
#[inline(always)]
pub(crate) unsafe fn log_parse_header(src: &[u8], max_log_size: usize) -> Result<(&[u8], &[u8]), ErrorLogParse> {
    if src.len() < 5 {
        return Err(ErrorLogParse::NoHeader);
    }

    let ptr = src.as_ptr() as *mut u8;
    if *ptr != 0xFF {
        return Err(ErrorLogParse::StartMarkerInvalid);
    }

    let record_crc = ptr.add(1).cast::<u32>().read_unaligned();
    let (length, size) = read_uvarint_safe(ptr.add(5), src.len() - 5);
    if size == usize::MAX {
        return Err(ErrorLogParse::RecordLengthInvalid);
    }
    if length as usize > max_log_size {
        return Err(ErrorLogParse::RecordLengthTooLarge);
    }
    if 5 + size + length as usize > src.len() {
        return Err(ErrorLogParse::RecordNeedMore);
    }
    let off = 5 + size;
    let record = slice::from_raw_parts(ptr.add(off), length as usize);
    let check = crc32c::crc32c(record);
    if check != record_crc {
        return Err(ErrorLogParse::RecordCRCMismatch)
    }
    let record_size = 5 + size + length as usize;

    Ok((record, slice::from_raw_parts(ptr.add(record_size), src.len() - record_size)))
}

/// Log parsing error states.
#[derive(Copy, Clone, Debug)]
pub(crate) enum ErrorLogParse {
    /// Missing this
    ///
    /// | 0xFF | CRC32 |
    /// |------|-------|
    ///
    /// 5 bytes header.
    NoHeader,
    /// Log data must start with 0xFF, got something different.
    StartMarkerInvalid,
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
}

#[inline(always)]
pub(crate) unsafe fn read_uvarint(ptr: *const u8) -> (u64, usize) {
    unsafe {
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

#[inline(always)]
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

/// Denotes a context parsing state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CtxParsingState {
    Normal,
    Group,
    Error,
    ErrorEmbed,
}