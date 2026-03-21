use crate::log_parser_node::Node;
use crate::value_kind::{PREDEFINED_KEYS, ValueKind};
use crate::{level, log_render};
use memchr::Memchr;
use std::io::Read;
use std::slice;

pub struct LogRender<'a> {
    pub(crate) itoa:      itoa::Buffer,
    pub(crate) ryu:       ryu::Buffer,
    pub(crate) buf:       Vec<u8>,
    pub(crate) need_tree: bool,
    pub(crate) err_stack: Vec<(usize, usize)>,
    pub(crate) grp_stack: Vec<(usize, RenderGroupType)>,

    pub(crate) tree_always: bool,
    pub(crate) tree_stack:  Vec<(u64, isize)>,
    pub(crate) tree_prefix: u64,
    pub(crate) tree_depth:  isize,

    pub(crate) time:  u64,
    pub(crate) level: u8,
    pub(crate) loc:   Option<(usize, usize, usize)>,
    pub(crate) msg:   (usize, usize),
    pub(crate) ctx:   &'a [u8],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum RenderGroupType {
    Root,
    Group,
    Error,
    ErrorEmbed,
    ErrorStage,
}

impl<'a> LogRender<'a> {
    pub fn new() -> Self {
        Self {
            itoa:        itoa::Buffer::new(),
            ryu:         ryu::Buffer::new(),
            buf:         Vec::with_capacity(4096),
            need_tree:   false,
            err_stack:   Vec::with_capacity(16),
            grp_stack:   Vec::with_capacity(16),
            tree_always: false,
            tree_stack:  Vec::with_capacity(16),
            tree_prefix: 0,
            tree_depth:  0,
            time:        0,
            level:       0,
            loc:         None,
            msg:         (0, 0),
            ctx:         &[],
        }
    }

    pub fn tree_only(&mut self) -> &mut Self {
        self.tree_always = true;
        self
    }

    pub(crate) unsafe fn render(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        unsafe {
            let ptr: *const u8 = src.as_ptr();

            self.render_time(dst, self.time as i64);
            let is_panic = self.render_level(dst);
            self.render_location(dst, ptr);

            if !is_panic {
                let (length, off) = self.msg;
                dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), length));
                dst.push(b' ');

                if self.need_tree || self.tree_always {
                    self.render_tree(dst, src);
                } else {
                    self.render_json(dst, src);
                }
                return;
            }

            self.render_json(dst, src);
            self.render_stacktrace(dst, src);
        }
    }

    pub(crate) unsafe fn render_stacktrace(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        unsafe {
            let ptr: *const u8 = src.as_ptr();
            let (length, off) = self.msg;
            let st = slice::from_raw_parts(ptr.add(off), length);

            let mut decoder = flate2::read::GzDecoder::new(st);
            self.buf.set_len(0);
            match decoder.read_to_end(&mut self.buf) {
                Ok(_) => {}
                Err(x) => {
                    self.buf.set_len(0);
                    self.buf.extend_from_slice(x.to_string().as_bytes());
                }
            };
            let mut start: usize = 0;
            let haystack = self.buf.as_slice();
            for pos in Memchr::new(b'\n', haystack) {
                dst.extend_from_slice(b".... ");
                dst.extend_from_slice(&haystack[start..pos]);
                dst.push(b'\n');
                start = pos + 1;
            }
            if start < haystack.len() {
                dst.extend_from_slice(b".... ");
                dst.extend_from_slice(&haystack[start..]);
                dst.push(b'\n');
            }
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn render_location(&mut self, dst: &mut Vec<u8>, ptr: *const u8) {
        unsafe {
            match self.loc {
                Some((lenght, off, line)) => {
                    dst.push(b'(');
                    dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), lenght));
                    dst.push(b':');
                    let lin = self.itoa.format(line);
                    dst.extend_from_slice(lin.as_bytes());
                    dst.push(b')');
                    dst.push(b' ');
                }
                _ => {}
            }
        }
    }

    #[inline(always)]
    pub(crate) fn render_level(&mut self, dst: &mut Vec<u8>) -> bool {
        let mut is_panic = false;

        match self.level {
            level::TRACE => {
                dst.extend_from_slice(b" TRACE");
            }

            level::DEBUG => {
                dst.extend_from_slice(b" DEBUG");
            }

            level::INFO => {
                dst.extend_from_slice(b"  INFO");
            }

            level::WARN => {
                dst.extend_from_slice(b"  WARN");
            }

            level::ERROR => {
                dst.extend_from_slice(b" ERROR");
            }

            level::PANIC => {
                is_panic = true;
                dst.extend_from_slice(b" PANIC");
            }
            _ => match self.level {
                0..10 => {
                    dst.extend_from_slice(b"   ");
                    dst.push(b'!');
                    dst.push('0' as u8 + self.level);
                }
                10..100 => {
                    dst.extend_from_slice(b"  ");
                    dst.push(b'!');
                    dst.push(b'0' as u8 + self.level / 10);
                    dst.push(b'0' as u8 + self.level % 10);
                }
                100..=255 => {
                    dst.extend_from_slice(b" ");
                    dst.push(b'!');
                    dst.push(b'0' as u8 + self.level / 100);
                    dst.push(b'0' as u8 + (self.level % 100) / 10);
                    dst.push(b'0' as u8 + self.level % 10);
                }
            },
        }

        dst.push(b' ');
        is_panic
    }

    #[inline(always)]
    pub(crate) fn render_int(&mut self, dst: &mut Vec<u8>, value: i64) {
        let res = self.itoa.format(value);
        dst.extend_from_slice(res.as_bytes());
    }

    #[inline(always)]
    pub(crate) fn render_uint(&mut self, dst: &mut Vec<u8>, value: u64) {
        let res = self.itoa.format(value);
        dst.extend_from_slice(res.as_bytes());
    }

    #[inline(always)]
    pub(crate) fn render_float(&mut self, dst: &mut Vec<u8>, value: f64) {
        let res = self.ryu.format(value);
        dst.extend_from_slice(res.as_bytes());
    }

    #[inline(always)]
    pub(crate) fn render_go_duration(&mut self, dst: &mut Vec<u8>, nanos: u64) {
        if nanos == 0 {
            dst.extend_from_slice(b"0s");
            return;
        }

        if nanos < 1_000 {
            // < 1µs
            let val = self.itoa.format(nanos);
            dst.extend_from_slice(val.as_bytes());
            dst.extend_from_slice(b"ns");
        } else if nanos < 1_000_000 {
            // < 1ms
            let val = self.itoa.format(nanos / 1_000);
            dst.extend_from_slice(val.as_bytes());
            dst.extend_from_slice(b"\xB5s");
        } else if nanos < 1_000_000_000 {
            // < 1s
            let val = self.itoa.format(nanos / 1_000_000);
            dst.extend_from_slice(val.as_bytes());
            dst.extend_from_slice(b"ms");
        } else {
            let mut seconds = nanos / 1_000_000_000;
            let n = nanos % 1_000_000_000;

            let hours = seconds / 3600;
            seconds %= 3600;
            let minutes = seconds / 60;
            seconds %= 60;

            if hours > 0 {
                let val = self.itoa.format(hours);
                dst.extend_from_slice(val.as_bytes());
                dst.extend_from_slice(b"h");
            }
            if minutes > 0 {
                let val = self.itoa.format(minutes);
                dst.extend_from_slice(val.as_bytes());
                dst.extend_from_slice(b"m");
            }
            if seconds > 0 || n > 0 {
                if n == 0 {
                    let val = self.itoa.format(seconds);
                    dst.extend_from_slice(val.as_bytes());
                    dst.extend_from_slice(b"s");
                } else {
                    // Формат секунд с дробной частью, как в Go (до 9 знаков)
                    let val = self.itoa.format(seconds);
                    dst.extend_from_slice(val.as_bytes());
                    let val = self.itoa.format(1_000_000_000 + n);
                    let fraction = &val.as_bytes()[1..];
                    let mut end = 9;
                    while end > 0 && fraction[end - 1] == b'0' {
                        end -= 1;
                    }
                    if end > 0 {
                        dst.push(b'.');
                        dst.extend_from_slice(&fraction[..end]);
                    }
                    dst.push(b's');
                }
            }
        }
    }

    /// TODO: replace on something functional, with timezones and so on.
    #[inline(always)]
    pub(crate) fn render_time(&mut self, dst: &mut Vec<u8>, unix_nanos: i64) {
        let secs = unix_nanos / 1_000_000_000;
        let nanos = unix_nanos % 1_000_000_000;

        // Математика эпохи (UTC)
        let z = (secs / 86400) + 719468;
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = (z - era * 146097) as u32;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = (yoe as i32) + (era as i32 * 400);
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = y + (if m <= 2 { 1 } else { 0 });

        let h = (secs / 3600) % 24;
        let min = (secs / 60) % 60;
        let s = secs % 60;

        // Пишем в буфер "вручную" (без парсинга формат-строки)
        // ГГГГ-ММ-ДД ЧЧ:ММ:СС.нннннн
        let start = dst.len();
        dst.extend_from_slice(b"0000-00-00 00:00:00");
        let b = &mut dst[start..];

        // Быстрая запись чисел (можно еще быстрее через lookup-таблицу на 100 байт)
        fn u2(b: &mut [u8], v: i64) {
            b[0] = b'0' + (v / 10 % 10) as u8;
            b[1] = b'0' + (v % 10) as u8;
        }

        // Год
        let y_u = year as i64;
        u2(&mut b[0..2], y_u / 100);
        u2(&mut b[2..4], y_u);
        // Остальное
        u2(&mut b[5..7], m as i64);
        u2(&mut b[8..10], d as i64);
        u2(&mut b[11..13], h);
        u2(&mut b[14..16], min);
        u2(&mut b[17..19], s);

        // Наносекунды (микро)
        let mic = nanos / 1000 + 1_000_000;
        let s = self.itoa.format(mic);
        let rs = &s.as_bytes()[1..];
        dst.push(b'.');
        dst.extend_from_slice(rs);
        // u2(&mut b[20..23], mic / 1000);
    }
}

pub(crate) fn predefined_key<'a>(key: ValueKind) -> &'a [u8] {
    if key < 255 as ValueKind {
        return "!invalid-predefined-key".as_bytes();
    }

    let index = (key >> 8) - 1;
    if index >= PREDEFINED_KEYS.len() as ValueKind {
        return "!unknown-predefined-key".as_bytes();
    }

    let value = PREDEFINED_KEYS[index as usize];
    return value.as_bytes();
}

#[cfg(test)]
mod test {
    use crate::log_render::LogRender;

    #[test]
    fn test_duration() {
        let v = 1_234_567_890_101_121u64;

        let mut r = LogRender::new();
        let mut out: Vec<u8> = Vec::new();
        r.render_go_duration(&mut out, v);

        assert_eq!(out, b"342h56m7.890101121s");
    }

    #[test]
    fn test_time() {
        let t = 1773974798041168000i64;

        let mut r = LogRender::new();
        let mut out: Vec<u8> = Vec::new();
        r.render_time(&mut out, t);

        assert_eq!(out, b"2026-03-20 02:46:38.041168");
    }
}
