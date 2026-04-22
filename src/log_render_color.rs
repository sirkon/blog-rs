#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::LogRender;

#[allow(unused)]
pub(crate) struct ColorProfile {
    pub(crate) reset:     &'static [u8],
    pub(crate) bold:      &'static [u8],
    pub(crate) time:      &'static [u8],
    pub(crate) trace:     &'static [u8],
    pub(crate) debug:     &'static [u8],
    pub(crate) info:      &'static [u8],
    pub(crate) warn:      &'static [u8],
    pub(crate) error:     &'static [u8],
    pub(crate) panic:     &'static [u8],
    pub(crate) levelu:    &'static [u8],
    pub(crate) loc:       &'static [u8],
    pub(crate) link:      &'static [u8],
    pub(crate) st_dots:   &'static [u8],
    pub(crate) st_text:   &'static [u8],
    pub(crate) key:       &'static [u8],
    pub(crate) err_key:   &'static [u8],
    pub(crate) err_meta:  &'static [u8],
    pub(crate) err_stage: &'static [u8],
    pub(crate) ctx:       &'static [u8],
}

#[allow(unused)]
impl ColorProfile {
    pub fn dark() -> Self {
        Self {
            reset:     b"\x1b[0m",
            bold:      b"\x1b[1m",
            time:      b"\x1b[35m",
            trace:     b"\x1b[90m",
            debug:     b"\x1b[36m",
            info:      b"\x1b[32m",
            warn:      b"\x1b[33m",
            error:     b"\x1b[31m",
            panic:     b"\x1b[1;41;97m",
            levelu:    b"\x1b[1;41;97m",
            loc:       b"\x1b[38;5;244m",
            link:      b"\x1b[38;5;236m",
            st_dots:   b"\x1b[38;5;236m",
            st_text:   b"\x1b[38;5;245m",
            key:       b"\x1b[38;5;109m",
            err_key:   b"\x1b[38;5;203m",
            err_meta:  b"\x1b[38;2;255;140;0m",
            err_stage: b"\x1b[38;2;255;165;0m",
            ctx:       b"\x1b[38;5;252m",
        }
    }

    pub fn light() -> Self {
        Self {
            reset:     b"\x1b[0m",
            bold:      b"\x1b[1m",
            time:      b"\x1b[95m",
            trace:     b"\x1b[90m",
            debug:     b"\x1b[36m",
            info:      b"\x1b[32m",
            warn:      b"\x1b[33m",
            error:     b"\x1b[31m",
            panic:     b"\x1b[1;41;97m",
            levelu:    b"\x1b[1;41;97m",
            loc:       b"\x1b[38;5;240m",
            link:      b"\x1b[38;5;248m",
            st_dots:   b"\x1b[38;5;252m",
            st_text:   b"\x1b[38;5;240m",
            key:       b"\x1b[38;5;31m",
            err_key:   b"\x1b[38;5;203m",
            err_meta:  b"\x1b[38;2;255;140;0m",
            err_stage: b"\x1b[38;2;255;165;0m",
            ctx:       b"\x1b[38;5;238m",
        }
    }

    pub fn noansi() -> Self {
        Self {
            reset:     b"",
            bold:      b"",
            time:      b"",
            trace:     b"",
            debug:     b"",
            info:      b"",
            warn:      b"",
            error:     b"",
            panic:     b"",
            levelu:    b"",
            loc:       b"",
            link:      b"",
            st_dots:   b"",
            st_text:   b"",
            key:       b"",
            err_key:   b"",
            err_meta:  b"",
            err_stage: b"",
            ctx:       b"",
        }
    }
}

impl<'a> LogRender<'a> {
    #[inline(always)]
    pub(crate) fn color_reset(&mut self, dst: &mut Vec<u8>) {
        match self.color_back {
            Some(x) => {
                dst.extend_from_slice(self.color_profile.reset);
                dst.extend_from_slice(x);
            }
            None => {
                dst.extend_from_slice(self.color_profile.reset);
            }
        }
    }

    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn color_set_back(&mut self, dst: &mut Vec<u8>, back: &'static [u8]) {
        self.color_back = Some(back);
        dst.extend_from_slice(back);
    }

    #[inline(always)]
    pub(crate) fn color_set_back_ctx(&mut self, dst: &mut Vec<u8>) {
        self.color_back = Some(self.color_profile.ctx);
        dst.extend_from_slice(self.color_profile.ctx);
    }

    #[inline(always)]
    pub(crate) fn color_reset_back(&mut self, dst: &mut Vec<u8>) {
        self.color_back = None;
        dst.extend_from_slice(self.color_profile.reset);
    }

    #[inline(always)]
    pub(crate) fn color_bold(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.bold);
    }

    #[inline(always)]
    pub(crate) fn color_level_trace(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.trace);
    }

    #[inline(always)]
    pub(crate) fn color_level_debug(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.debug)
    }

    #[inline(always)]
    pub(crate) fn color_level_info(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.info);
    }

    #[inline(always)]
    pub(crate) fn color_level_warn(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.warn);
    }

    #[inline(always)]
    pub(crate) fn color_level_error(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.error);
    }

    #[inline(always)]
    pub(crate) fn color_level_panic(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.panic);
    }

    #[inline(always)]
    pub(crate) fn color_level_unknown(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.levelu);
    }

    #[inline(always)]
    pub(crate) fn color_loc(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.loc);
    }

    #[inline(always)]
    pub(crate) fn color_time(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.time);
    }

    #[inline(always)]
    pub(crate) fn color_link(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.link);
    }

    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn color_st_dots(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.st_dots);
    }

    #[inline(always)]
    #[allow(unused)]
    pub(crate) fn color_st_text(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.st_text);
    }

    #[inline(always)]
    pub(crate) fn color_key(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.key);
    }

    #[inline(always)]
    pub(crate) fn color_err_key(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.err_key);
    }

    pub(crate) fn color_err_meta(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.key);
    }

    #[inline(always)]
    pub(crate) fn color_err_stage(&mut self, dst: &mut Vec<u8>) {
        dst.extend_from_slice(self.color_profile.bold);
        dst.extend_from_slice(self.color_profile.key);
    }
}
