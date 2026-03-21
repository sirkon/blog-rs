#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parser_node::NodeKind;
use crate::log_parser_tree_builder::TreeBuilder;
use crate::log_render::LogRender;

pub struct LogParser {
    pub(crate) max_log_size:        usize,
    pub(crate) groups_lens:         Vec<(usize, usize)>,
    pub(crate) caps:                Vec<usize>,
    pub(crate) err_frags:           Vec<(usize, usize)>,
    pub(crate) state_stack:         Vec<CtxParsingState>,
    pub(crate) ctx_size:            usize,
    pub(crate) has_errors:          bool,
    pub(crate) process_since_level: u8,
    pub(crate) group_depth:         usize,

    // Parsing data.
    pub(crate) time:     u64,
    pub(crate) level:    u8,
    pub(crate) location: Option<(usize, usize, usize)>,
    pub(crate) msg:      (usize, usize),
    pub(crate) ctx:      TreeBuilder,
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
    RecordBroken,
    /// Record data has unsupported version.
    RecordVersionNotSupported(u16),
    /// Record data has unsupported level.
    RecordLevelNotSupported(u8),
}

/// Denotes a context parsing state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CtxParsingState {
    Normal,
    Group,
    Error,
    ErrorEmbed,
}

impl LogParser {
    pub fn new() -> Self {
        Self {
            max_log_size:        1 * 1024 * 1024,
            groups_lens:         Vec::with_capacity(8),
            caps:                Vec::with_capacity(8),
            err_frags:           Vec::with_capacity(8),
            state_stack:         Vec::with_capacity(8),
            ctx_size:            0,
            has_errors:          false,
            process_since_level: 0,
            group_depth:         0,

            time:     0,
            level:    0,
            location: None,
            msg:      (0, 0),
            ctx:      TreeBuilder::new(),
        }
    }

    pub(crate) fn make_record<'a, 'b>(&'a self, dst: &'b mut LogRender<'a>)
    where
        'a: 'b,
    {
        dst.need_tree = (self.ctx_size >= dst.expand_context_since || self.has_errors)
            && self.group_depth <= 16;
        dst.time = self.time;
        dst.level = self.level;
        dst.loc = self.location;
        dst.msg = self.msg;
        dst.ctx = &self.ctx.ctrl.as_slice()[..self.ctx.off];
        dst.grp_stack.clear();
        dst.err_stack.clear();
        dst.tree_stack.clear();
        dst.tree_prefix = 0;
        dst.tree_depth = 0;
    }

    pub(crate) fn with_max_log_record_size(&mut self, size: usize) -> &mut Self {
        self.max_log_size = size;
        self
    }

    pub(crate) fn with_show_since_level(&mut self, level: u8) -> &mut Self {
        self.process_since_level = level;
        self
    }

    pub(crate) fn should_pass(&self) -> bool {
        self.level < self.process_since_level
    }

    #[inline(always)]
    pub(crate) unsafe fn leave_stage_group_if_needed(&mut self, had_stages: bool) -> bool {
        unsafe {
            if had_stages {
                self.ctx.leave_group();
            }

            true
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn varthing(
        &mut self,
        src: *const u8,
        off: usize,
        nkind: NodeKind,
        key_len: u32,
        key_off: u32,
    ) -> usize {
        unsafe {
            let (length, size) = read_uvarint(src.add(off));
            self.ctx
                .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

            off + size + length as usize
        }
    }
    #[inline(always)]
    pub(crate) unsafe fn slice(
        &mut self,
        src: *const u8,
        off: usize,
        nkind: NodeKind,
        key_len: u32,
        key_off: u32,
        siz: usize,
    ) -> usize {
        unsafe {
            let (length, size) = read_uvarint(src.add(off));
            self.ctx
                .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

            off + size + (length as usize) * siz
        }
    }
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

#[cfg(test)]
mod test {
    use crate::log_parser_node::NodeKind;
    use crate::log_parser_tree_builder;
    use crate::log_parser_tree_builder::TreeBuilder;

    #[test]
    fn test_tree_builder() {
        unsafe {
            let mut b = TreeBuilder::new();
            b.add(NodeKind::Bool, 2, 3, 4, 5);
            b.add(NodeKind::Int, 3, 4, 5, 6);
            b.add(NodeKind::Group, 31, 32, 33, 34);
            b.enter_group();
            b.add(NodeKind::Bool, 32, 33, 34, 35);
            b.add(NodeKind::Int, 33, 34, 35, 36);
            b.add(NodeKind::Group, 331, 332, 333, 333);
            b.enter_group();
            b.add(NodeKind::F32, 332, 333, 334, 335);
            b.leave_group();
            b.add(NodeKind::Str, 35, 36, 37, 38);
            b.leave_group();
            b.add(NodeKind::Bytes, 5, 6, 7, 8);

            println!();
            log_parser_tree_builder::show(b.ctrl.as_ptr());
        }
    }
}
