#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parser_node::{Node, NodeKind};
use crate::log_render::LogRender;
use std::slice;

pub struct LogParser {
    pub(crate) max_log_size:        usize,
    pub(crate) groups_lens:         Vec<(usize, usize)>,
    pub(crate) caps:                Vec<usize>,
    pub(crate) err_frags:           Vec<(usize, usize)>,
    pub(crate) state_stack:         Vec<CtxParsingState>,
    pub(crate) ctx_size:            usize,
    pub(crate) has_errors:          bool,
    pub(crate) use_tree_since:      usize,
    pub(crate) process_since_level: u8,
    pub(crate) source_off:          usize,

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
#[derive(Copy, Clone, Debug)]
pub(crate) enum CtxParsingState {
    Normal,
    Group,
    Error,
    ErrorEmbed,
}

/// [Tree] represents a logical tree layout in a continuous memory area.
pub(crate) struct Tree<'a> {
    ctrl: &'a [u8],
    tree: bool,
}

/// blog context structure builder type.
pub(crate) struct TreeBuilder {
    pub(crate) ctrl:  Vec<u8>,
    pub(crate) stack: Vec<usize>,
    pub(crate) last:  isize,
    pub(crate) off:   usize,
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
            use_tree_since:      10,
            process_since_level: 0,
            source_off:          0,

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
        dst.need_tree = self.ctx_size >= self.use_tree_since || self.has_errors;
        dst.time = self.time;
        dst.level = self.level;
        dst.loc = self.location;
        dst.msg = self.msg;
        dst.ctx = &self.ctx.ctrl.as_slice()[..self.ctx.off];
        dst.grp_stack.clear();
        dst.err_stack.clear();
    }

    pub(crate) fn with_max_log_record_size(&mut self, size: usize) -> &mut Self {
        self.max_log_size = size;
        self
    }

    pub(crate) fn with_show_tree_after(&mut self, size: usize) -> &mut Self {
        self.use_tree_since = size;
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

impl TreeBuilder {
    /// Constructs new [TreeBuilder] instance with preallocated data.
    pub(crate) fn new() -> Self {
        Self {
            ctrl:  vec![0u8; 4096],
            stack: Vec::with_capacity(16),
            last:  -1,
            off:   0,
        }
    }

    /// Resets the state of builder for further reuse.
    pub(crate) unsafe fn reset(&mut self) {
        unsafe {
            self.stack.set_len(0);
            self.last = -1;
            self.off = 0;
        }
    }

    /// Adds a new node with given type and data.
    #[inline(always)]
    pub(crate) unsafe fn add(
        &mut self,
        kind: NodeKind,
        key_len: u32,
        key_off: u32,
        val_len: u32,
        val_off: u32,
    ) {
        unsafe {
            let node_size = 32;
            let pos = self.off;

            // 1. Check capacity and reallocate if needed.
            if self.ctrl.capacity() < pos + 32 {
                // Get some more space to avoid non-stop reallocates.
                self.ctrl.reserve(1024);
                self.ctrl.set_len(self.ctrl.capacity());
            }

            let ptr = self.ctrl.as_mut_ptr();

            // 3. Get direct pointer to a Node in ctrl.
            let node = ptr.add(pos) as *mut Node;
            (*node).kind = kind;
            (*node).next = 0;
            (*node).child = 0;
            (*node).key_len = key_len;
            (*node).key_off = key_off;
            (*node).val_len = val_len;
            (*node).val_off = val_off;

            if self.last < 0 {
                (self.last, self.off) = (self.off as isize, self.off + node_size);
                return;
            }

            let prev = ptr.add(self.last as usize) as *mut Node;
            let kind: u32 = (*prev).kind as u32;
            if (kind >> 7) > 0 && (*prev).child == 0 {
                (*prev).child = self.off as u32;
            } else {
                (*prev).next = self.off as u32;
            }
            (self.last, self.off) = (self.off as isize, self.off + node_size);
        }
    }

    /// Starts a new group AFTER root node (kind >=128). There's no check though,
    /// this is up to a user.
    /// This call must be paired with respective leave_group.
    #[inline(always)]
    pub(crate) fn enter_group(&mut self) {
        self.stack.push(self.last as usize);
    }

    /// Exits existing group. Must be called somewhere after enter_group.
    #[inline(always)]
    pub(crate) unsafe fn leave_group(&mut self) {
        unsafe {
            let last = self.last;
            self.last = self.stack.pop().unwrap() as isize;

            if last == self.last {
                // We are closing a root node that has no content.
                let node = self.ctrl.as_mut_ptr().add(last as usize) as *mut Node;
                (*node).child = u32::MAX;
            }
        }
    }

    /// Превращает билд в готовое дерево (Zero-copy ссылки)
    pub(crate) unsafe fn finish<'a>(&'a self) -> Tree<'a> {
        let res: &[u8] = unsafe {
            let ptr = self.ctrl.as_ptr();
            slice::from_raw_parts(ptr, self.off)
        };
        Tree {
            ctrl: res,
            tree: true,
        }
    }

    // Shows collected data as a dump.
    pub(crate) unsafe fn show(&mut self) {
        unsafe {
            if self.off == 0 {
                println!("empty");
                return;
            }

            let mut pos = 0;
            let mut stack: Vec<usize> = Vec::with_capacity(16);
            let ptr = self.ctrl.as_mut_ptr();

            loop {
                let node = &mut *(ptr.add(pos) as *mut Node);

                println!(
                    "{:03X} {:10} next[{:03X}] child[{:03X}] key.len[{:03}] key.off[{:03}] val[{:03}] val.off[{:03}]",
                    pos,
                    node.kind.string(),
                    node.next,
                    node.child,
                    node.key_len,
                    node.key_off,
                    node.val_len,
                    node.val_off,
                );

                if (node.kind as u32 >> 7) != 0 && node.child != u32::MAX {
                    stack.push(pos);
                    pos = node.child as usize;
                    continue;
                }

                let mut curr_node = node;
                loop {
                    if curr_node.next != 0 {
                        pos = curr_node.next as usize;
                        break;
                    }

                    if stack.is_empty() {
                        return;
                    }

                    pos = stack.pop().unwrap();
                    curr_node = &mut *(ptr.add(pos) as *mut Node);
                }
            }
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
    use crate::log_parser::TreeBuilder;
    use crate::log_parser_node::NodeKind;

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
            b.show();
        }
    }
}
