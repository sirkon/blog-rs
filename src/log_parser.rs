#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parse;
use crate::log_parser_node::NodeKind;
use crate::log_parser_tree_builder::TreeBuilder;
use crate::log_render::LogRender;

pub struct LogParser {
    pub(crate) max_log_size:        usize,
    pub(crate) group_stack:         Vec<usize>,
    pub(crate) group_depth:         usize,
    pub(crate) ctx_size:            usize,
    pub(crate) has_errors:          bool,
    pub(crate) process_since_level: u8,

    // Parsing data.
    pub(crate) time:     u64,
    pub(crate) level:    u8,
    pub(crate) location: Option<(usize, usize, usize)>,
    pub(crate) msg:      (usize, usize),
    pub(crate) ctx:      TreeBuilder,
}

impl LogParser {
    pub fn new() -> Self {
        Self {
            max_log_size:        1 * 1024 * 1024,
            group_stack:         Vec::with_capacity(8),
            group_depth:         0,
            ctx_size:            0,
            has_errors:          false,
            process_since_level: 0,

            time:     0,
            level:    0,
            location: None,
            msg:      (0, 0),
            ctx:      TreeBuilder::new(),
        }
    }

    pub(crate) unsafe fn make_record<'a, 'b>(&'a self, dst: &'b mut LogRender<'a>)
    where
        'a: 'b,
    {
        dst.need_tree = (self.ctx_size >= dst.expand_context_since || self.has_errors)
            && self.group_depth <= 16;
        dst.time = self.time;
        dst.level = self.level;
        dst.loc = self.location;
        dst.msg = self.msg;
        dst.ctx = self.ctx.ctrl.as_slice();
        dst.err_stack.clear();
        dst.tree_stack.clear();
        dst.tree_prefix = 0;
        dst.tree_depth = 0;
    }

    #[allow(unused)]
    pub(crate) fn with_max_log_record_size(&mut self, size: usize) -> &mut Self {
        self.max_log_size = size;
        self
    }

    #[allow(unused)]
    pub(crate) fn with_show_since_level(&mut self, level: u8) -> &mut Self {
        self.process_since_level = level;
        self
    }

    #[allow(unused)]
    pub(crate) fn should_pass(&self) -> bool {
        self.level < self.process_since_level
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
            let (length, size) = log_parse::read_uvarint(src.add(off));
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
            let (length, size) = log_parse::read_uvarint(src.add(off));
            self.ctx
                .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

            off + size + (length as usize) * siz
        }
    }
}

