use crate::level;
use crate::log_rend::{render_go_duration, render_time};
use crate::log_render_color::ColorProfile;
use crate::value_kind::{PREDEFINED_KEYS, ValueKind};
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

    pub(crate) expand_array_since:   usize,
    pub(crate) expand_context_since: usize,

    pub(crate) color_profile: ColorProfile,
    pub(crate) color_back:    Option<&'static [u8]>,

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
    pub fn new(color_profile: ColorProfile) -> Self {
        Self {
            itoa:                 itoa::Buffer::new(),
            ryu:                  ryu::Buffer::new(),
            buf:                  Vec::with_capacity(4096),
            need_tree:            false,
            err_stack:            Vec::with_capacity(16),
            grp_stack:            Vec::with_capacity(16),
            tree_always:          false,
            tree_stack:           Vec::with_capacity(16),
            tree_prefix:          0,
            tree_depth:           0,
            expand_array_since:   8,
            expand_context_since: 6,
            color_profile:        color_profile,
            color_back:           None,
            time:                 0,
            level:                0,
            loc:                  None,
            msg:                  (0, 0),
            ctx:                  &[],
        }
    }

    /// Forces tree view.
    pub fn tree_only(&mut self) -> &mut Self {
        self.tree_always = true;
        self
    }

    // Sets an amount of context elements to show as tree after reaching this value.
    pub fn show_as_tree_since(&mut self, size: usize) -> &mut Self {
        self.expand_context_since = size;
        self
    }

    // Sets an array length to show in expanded form since it reaches this value.
    pub fn show_arrays_as_tree_since_size(&mut self, size: usize) -> &mut Self {
        self.expand_array_since = size;
        self
    }

    pub(crate) unsafe fn render(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        unsafe {
            let ptr: *const u8 = src.as_ptr();

            self.render_time(dst, self.time as i64, false);
            let is_panic = self.render_level(dst);
            self.render_location(dst, ptr);

            if !is_panic {
                let (length, off) = self.msg;
                self.color_bold(dst);
                dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), length));
                dst.push(b' ');
                self.color_reset(dst);
                self.color_set_back_ctx(dst);

                if self.need_tree || self.tree_always {
                    self.render_tree(dst, src);
                } else {
                    self.render_json(dst, src);
                }
                self.color_reset_back(dst);
                return;
            }

            self.color_set_back_ctx(dst);
            self.render_json(dst, src);
            self.color_reset_back(dst);
            self.render_stacktrace(dst, src);
        }
    }

    pub(crate) unsafe fn render_stacktrace(&mut self, dst: &mut Vec<u8>, src: &[u8]) {
        unsafe {
            let ptr: *const u8 = src.as_ptr();
            let (length, off) = self.msg;
            let st = slice::from_raw_parts(ptr.add(off), length);

            let mut decoder = flate2::read::GzDecoder::new(st);
            self.buf.clear();
            match decoder.read_to_end(&mut self.buf) {
                Ok(_) => {}
                Err(x) => {
                    self.color_level_error(dst);
                    self.buf.clear();
                    self.buf.extend_from_slice(x.to_string().as_bytes());
                    self.color_reset(dst);
                }
            };
            let mut start: usize = 0;
            {
                let haystack = self.buf.as_slice();
                for pos in Memchr::new(b'\n', haystack) {
                    dst.extend_from_slice(self.color_profile.st_dots);
                    dst.extend_from_slice(b".... ");
                    dst.extend_from_slice(self.color_profile.st_text);
                    dst.extend_from_slice(&haystack[start..pos]);
                    dst.push(b'\n');
                    start = pos + 1;
                }
                if start < haystack.len() {
                    dst.extend_from_slice(self.color_profile.st_dots);
                    dst.extend_from_slice(b".... ");
                    dst.extend_from_slice(self.color_profile.st_text);
                    dst.extend_from_slice(&haystack[start..]);
                    dst.push(b'\n');
                }
            }
            self.color_reset(dst);
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn render_location(&mut self, dst: &mut Vec<u8>, ptr: *const u8) {
        unsafe {
            match self.loc {
                Some((lenght, off, line)) => {
                    self.color_loc(dst);
                    dst.push(b'(');
                    dst.extend_from_slice(slice::from_raw_parts(ptr.add(off), lenght));
                    dst.push(b':');
                    let lin = self.itoa.format(line);
                    dst.extend_from_slice(lin.as_bytes());
                    dst.push(b')');
                    dst.push(b' ');
                    self.color_reset(dst);
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
                dst.push(b' ');
                self.color_level_trace(dst);
                dst.extend_from_slice(b"TRACE");
                self.color_reset(dst);
            }

            level::DEBUG => {
                dst.push(b' ');
                self.color_level_debug(dst);
                dst.extend_from_slice(b"DEBUG");
                self.color_reset(dst);
            }

            level::INFO => {
                dst.push(b' ');
                dst.push(b' ');
                self.color_level_info(dst);
                dst.extend_from_slice(b"INFO");
                self.color_reset(dst);
            }

            level::WARN => {
                dst.push(b' ');
                dst.push(b' ');
                self.color_level_warn(dst);
                dst.extend_from_slice(b"WARN");
                self.color_reset(dst);
            }

            level::ERROR => {
                dst.push(b' ');
                self.color_level_error(dst);
                dst.extend_from_slice(b"ERROR");
                self.color_reset(dst);
            }

            level::PANIC => {
                is_panic = true;
                dst.push(b' ');
                self.color_level_panic(dst);
                dst.extend_from_slice(b"PANIC");
                self.color_reset(dst);
            }
            _ => {
                self.color_level_unknown(dst);
                match self.level {
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
                };
                self.color_reset(dst);
            }
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
    pub(crate) fn render_go_duration(&mut self, dst: &mut Vec<u8>, nanos: u64, in_ctx: bool) {
        render_go_duration(&mut self.itoa, dst, nanos);
    }

    #[inline(always)]
    pub(crate) fn render_time(&mut self, dst: &mut Vec<u8>, nanos: i64, in_ctx: bool) {
        self.color_time(dst);
        render_time(&mut self.itoa, dst, nanos);
        self.color_reset(dst);
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
    use crate::log_render_color::ColorProfile;

    #[test]
    fn test_duration() {
        let v = 1_234_567_890_101_121u64;

        let mut r = LogRender::new(ColorProfile::noansi());
        let mut out: Vec<u8> = Vec::new();
        r.render_go_duration(&mut out, v, false);

        assert_eq!(out, b"342h56m7.890101121s");
    }

    #[test]
    fn test_time() {
        let t = 1773974798041168000i64;

        let mut r = LogRender::new(ColorProfile::noansi());
        let mut out: Vec<u8> = Vec::new();
        r.render_time(&mut out, t, false);

        assert_eq!(out, b"2026-03-20 05:46:38.041");
    }
}
