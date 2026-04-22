#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::log_parser_node::{Node, NodeKind};

/// blog context structure builder type.
pub(crate) struct TreeBuilder {
    pub(crate) ctrl:  Vec<Node>,
    pub(crate) stack: Vec<usize>,
    pub(crate) last:  isize,
    pub(crate) off:   usize,
}

impl TreeBuilder {
    /// Constructs new [TreeBuilder] instance with preallocated data.
    pub(crate) fn new() -> Self {
        Self {
            ctrl:  Vec::with_capacity(128),
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
        self.ctrl.push(Node {
            kind,
            is_last: 0,
            key_len,
            key_off,
            val_len,
            val_off,
        })
    }
}

// Shows collected data as a dump.
#[allow(unused)]
pub(crate) unsafe fn show(ctrl: &Vec<Node>) {
    unsafe {
        for (i, node) in ctrl.iter().enumerate() {
            println!(
                "{:03X} {:10} last[{}] key.len[{:03}] key.off[{:03}] val[{:03}] val.off[{:03}]",
                i,
                node.kind.string(),
                node.is_last != 0,
                node.key_len,
                node.key_off,
                node.val_len,
                node.val_off,
            );
        }
    }
}
