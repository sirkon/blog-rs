#![allow(unused)]

use crate::crc32custom;
use crate::log_parse::{log_parse_header, ErrorLogParse};
use crate::pointer_ext::PointerAppender;
use std::ops::Range;
use std::slice;

pub struct LogTransformerRepack {
    pub(crate) buf:          Vec<u8>,
    pub(crate) max_log_size: usize,
}

impl<'a> LogTransformerRepack {
    pub fn new() -> LogTransformerRepack {
        LogTransformerRepack {
            buf:          Vec::with_capacity(4096),
            max_log_size: 1 * 1024 * 1024,
        }
    }

    /// Packs record given in src into dst. Returns a range of the buffer
    /// referencing packed data and the rest of data after the record.
    pub unsafe fn transform(
        &mut self,
        dst: &mut Vec<u8>,
        src: &'a [u8],
    ) -> Result<(Range<usize>, &'a [u8]), ErrorLogParse> {
        // Repack won't make a record wider.
        dst.reserve(src.len());

        unsafe {
            let (record, rest) = match log_parse_header(src, self.max_log_size) {
                Ok((record, rest)) => (record, rest),
                Err(e) => return Err(e),
            };

            let ptr = record.as_ptr();
            let version = u16::from_le(ptr.cast::<u16>().read_unaligned());
            match version {
                1 => {
                    let rng = self.transform_repack_v1(dst, ptr as *mut u8, record.len())?;
                    return Ok((rng, rest));
                }
                _ => Err(ErrorLogParse::RecordVersionNotSupported(version)),
            }
        }
    }

    unsafe fn transform_repack_v1(
        &self,
        dst: &mut Vec<u8>,
        mut src: *mut u8,
        len: usize,
    ) -> Result<Range<usize>, ErrorLogParse> {
        unsafe {
            let pstart = dst.as_ptr();
            let srcorig = src;
            let mut pdst = dst.as_ptr().add(16) as *mut u8;

            // Copy || version(2) | time(8) | level(1)
            (pdst, src) = pdst.copy(src, 11);

            // Copy location.
            if *pdst == 0 {
                // There's no location.
                pdst = pdst.append_byte(0);
                src = src.add(1);
            } else {
                // Read file len (uvarint), file name data, line no (uvarint)
                (pdst, src) = pdst.copy_str(src);
                (pdst, src) = pdst.copy_uvarint(src);
            }

            // Copy message.
            (pdst, src) = pdst.copy_str(src);

            // Transform context.
            pdst =
                self.transform_repack_ctx_v1(pdst, src, len - (src.offset_from(srcorig) as usize))?;

            // Create || 0xFF | CRC32(new_record) | 0xFE | length(new_record) || header
            // before the new record.
            let new_len = pdst.offset_from(pstart) as usize - 16;
            let crc = crc32custom::fast_crc32c(
                0,
                slice::from_raw_parts(pstart.add(16), new_len),
            );
            let width = uvarint_bytes(new_len) + 6;
            let mut pheader = pstart.add(16 - width) as *mut u8;
            pheader = pheader.append_le(0xFF as u32 | crc << 8);
            pheader = pheader.append_le(crc >> 24 | 0xFE00);
            pheader = pheader.append_uvarint(new_len as u64);


            Ok(Range {
                start: pheader.offset_from(pstart) as usize,
                end:   pdst.offset_from(pstart) as usize,
            })
        }
    }

    #[allow(unused_variables)]
    fn transform_repack_ctx_v1(
        &self,
        dst: *mut u8,
        src: *mut u8,
        siz3: usize,
    ) -> Result<*mut u8, ErrorLogParse> {
        Ok(dst)
    }
}

fn uvarint_bytes(v: usize) -> usize {
    let bits = if v == 0 { 1 } else { usize::BITS - v.leading_zeros() };
    (bits as usize + 6) / 7  // деление с округлением вверх
}