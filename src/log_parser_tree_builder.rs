use std::slice;
use crate::log_parser_node::{Node, NodeKind};

/// blog context structure builder type.
pub(crate) struct TreeBuilder {
    pub(crate) ctrl:  Vec<u8>,
    pub(crate) stack: Vec<usize>,
    pub(crate) last:  isize,
    pub(crate) off:   usize,
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
}

// Shows collected data as a dump.
pub(crate) unsafe fn show(ptr: *const u8) {
    unsafe {
        let mut pos = 0;
        let mut stack: Vec<usize> = Vec::with_capacity(16);

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

/// [Tree] represents a logical tree layout in a continuous memory area.
pub(crate) struct Tree<'a> {
    ctrl: &'a [u8],
    tree: bool,
}