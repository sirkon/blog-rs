#![allow(unused_unsafe)]
#![allow(unsafe_code)]

use crate::value_kind;
use std::fmt::{Display, Formatter};
use std::slice;

pub struct CtxParser {
    need_tree: bool,
    builder:   TreeBuilder,
}

impl CtxParser {
    pub fn new() -> Self {
        Self {
            need_tree: false,
            builder:   TreeBuilder::new(),
        }
    }

    pub unsafe fn build_struct<'a>(&mut self, src: &'a [u8]) {
        self.builder.reset();

        let mut off: usize = 0;
        let mut need_tree: bool = false;
        let ptr = src.as_ptr() as *mut u8;
        let mut had_stages = false;
        loop {
            if off >= src.len() {
                return;
            }

            // Read code and continue the loop on some types that have no payload.
            let kind = *(ptr.add(off)) as value_kind::ValueKind;
            off += 1;
            match kind {
                value_kind::JUST_CONTEXT_NODE | value_kind::JUST_CONTEXT_INHERITED_NODE => {
                    had_stages = self.leave_stage_group_if_needed(had_stages);
                    self.builder.add(NodeKind::ErrorStageCtx, 0, 0, 0, 0);
                    self.builder.enter_group();
                    continue;
                }
                value_kind::PHANTOM_CONTEXT_NODE => {
                    continue;
                }
                _ => {}
            }

            // Read the key. It can be either 0-lead uvarint of predefined key index, or
            // a literal key with uvarint(length) + body.
            let mut key_len: u32 = 0;
            let mut key_off: u32 = 0;
            let v = *(ptr.add(off));
            if v != 0 {
                let (length, size) = read_uvarint(ptr.add(off));
                key_len = length as u32;
                key_off = (off + size) as u32;
                off += size + length as usize;
            } else {
                let (length, size) = read_uvarint(ptr.add(off + 1));
                key_len = 0;
                key_off = length as u32;
                off += size + 1;
            }

            match kind {
                value_kind::NEW_NODE => {
                    had_stages = self.leave_stage_group_if_needed(had_stages);
                    let (length, size) = read_uvarint(ptr.add(off));
                    self.builder.add(
                        NodeKind::ErrorStageNew,
                        key_len,
                        key_off,
                        length as u32,
                        (off + size) as u32,
                    );
                    off += size + length as usize;
                    self.builder.enter_group();
                }
                value_kind::WRAP_NODE | value_kind::WRAP_INHERITED_NODE => {
                    had_stages = self.leave_stage_group_if_needed(had_stages);
                    let (length, size) = read_uvarint(ptr.add(off));
                    self.builder.add(
                        NodeKind::ErrorStageWrap,
                        key_len,
                        key_off,
                        length as u32,
                        (off + size) as u32,
                    );
                    off += size + length as usize;
                    self.builder.enter_group();
                }
                value_kind::LOCATION_NODE => {
                    let (length, size) = read_uvarint(ptr.add(off));
                    off += size;
                    self.builder
                        .add(NodeKind::ErrLoc, key_len, key_off, 0, length as u32);
                }
                value_kind::FOREIGN_ERROR_TEXT => {
                    self.builder
                        .add(NodeKind::ErrTxtFragment, key_len, key_off, 0, 0);
                }
                value_kind::FOREIGN_ERROR_FORMAT => {
                    // Not supported as for now.
                }

                value_kind::BOOL => {
                    let v = *(ptr.add(off));
                    self.builder
                        .add(NodeKind::Bool, key_len, key_off, 0, v as u32);
                    off += 1
                }

                value_kind::TIME => {
                    let v = *(ptr.add(off) as *const u64);
                    self.builder.add(
                        NodeKind::Time,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8
                }

                value_kind::DURATION => {
                    let v = *(ptr.add(off) as *const u64);
                    self.builder.add(
                        NodeKind::Time,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8
                }

                value_kind::I => {
                    let v = *(ptr.add(off) as *const u64);
                    self.builder.add(
                        NodeKind::Int,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8
                }

                value_kind::I8 => {
                    let v = *(ptr.add(off) as *const i8);
                    self.builder
                        .add(NodeKind::I8, key_len, key_off, 0, v as u32);
                    off += 1
                }

                value_kind::I16 => {
                    let v = *(ptr.add(off) as *const i16);
                    self.builder
                        .add(NodeKind::I16, key_len, key_off, 0, v as u32);
                    off += 2
                }

                value_kind::I32 => {
                    let v = *(ptr.add(off) as *const i32);
                    self.builder
                        .add(NodeKind::I32, key_len, key_off, 0, v as u32);
                    off += 4
                }

                value_kind::I64 => {
                    let v = *(ptr.add(off) as *const i64);
                    self.builder.add(
                        NodeKind::Int,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8
                }

                value_kind::U => {
                    let v = *(ptr.add(off) as *const u64);
                    self.builder.add(
                        NodeKind::Uint,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8
                }

                value_kind::U8 => {
                    let v = *(ptr.add(off) as *const u8);
                    self.builder
                        .add(NodeKind::U8, key_len, key_off, 0, v as u32);
                    off += 1
                }

                value_kind::U16 => {
                    let v = *(ptr.add(off) as *const u16);
                    self.builder
                        .add(NodeKind::U16, key_len, key_off, 0, v as u32);
                    off += 2
                }

                value_kind::U32 => {
                    let v = *(ptr.add(off) as *const u32);
                    self.builder.add(NodeKind::U32, key_len, key_off, 0, v);
                    off += 4
                }

                value_kind::U64 => {
                    let v = *(ptr.add(off) as *const u64);
                    self.builder.add(
                        NodeKind::U64,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8;
                }

                value_kind::FLOAT32 => {
                    let v = *(ptr.add(off) as *const u32);
                    self.builder.add(NodeKind::F32, key_len, key_off, 0, v);
                    off += 4
                }

                value_kind::FLOAT64 => {
                    let v = *(ptr.add(off) as *const u64);
                    self.builder.add(
                        NodeKind::F64,
                        key_len,
                        key_off,
                        v as u32,
                        (v >> 32) as u32,
                    );
                    off += 8;
                }

                value_kind::STRING => {
                    off = self.varthing(ptr, off, NodeKind::Str, key_len, key_off);
                }

                value_kind::BYTES => {
                    off = self.varthing(ptr, off, NodeKind::Bytes, key_len, key_off);
                }

                value_kind::SLICE_BOOL => {
                    off = self.slice(ptr, off, NodeKind::Bool, key_len, key_off, 1);
                }

                value_kind::SLICE_I => {
                    off = self.slice(ptr, off, NodeKind::Ints, key_len, key_off, 8);
                }

                value_kind::SLICE_I8 => {
                    off = self.slice(ptr, off, NodeKind::I8s, key_len, key_off, 1);
                }

                value_kind::SLICE_I16 => {
                    off = self.slice(ptr, off, NodeKind::I16s, key_len, key_off, 2);
                }

                value_kind::SLICE_I32 => {
                    off = self.slice(ptr, off, NodeKind::I32s, key_len, key_off, 4);
                }

                value_kind::SLICE_I64 => {
                    off = self.slice(ptr, off, NodeKind::I64s, key_len, key_off, 8);
                }

                value_kind::SLICE_U => {
                    off = self.slice(ptr, off, NodeKind::Uints, key_len, key_off, 8);
                }

                value_kind::SLICE_U8 => {
                    off = self.slice(ptr, off, NodeKind::U8s, key_len, key_off, 1);
                }

                value_kind::SLICE_U16 => {
                    off = self.slice(ptr, off, NodeKind::U16s, key_len, key_off, 2);
                }

                value_kind::SLICE_U32 => {
                    off = self.slice(ptr, off, NodeKind::U32s, key_len, key_off, 4);
                }

                value_kind::SLICE_U64 => {
                    off = self.slice(ptr, off, NodeKind::U64s, key_len, key_off, 8);
                }

                value_kind::SLICE_F32 => {
                    off = self.slice(ptr, off, NodeKind::F32s, key_len, key_off, 4);
                }

                value_kind::SLICE_F64 => {
                    off = self.slice(ptr, off, NodeKind::F64s, key_len, key_off, 8);
                }

                _ => {}
            }
        }
    }

    #[inline(always)]
    pub unsafe fn leave_stage_group_if_needed(&mut self, had_stages:bool) -> bool {
        if had_stages {
            self.builder.leave_group();
        }

        true
    }

    #[inline(always)]
    pub unsafe fn varthing(
        &mut self,
        src: *const u8,
        off: usize,
        nkind: NodeKind,
        key_len: u32,
        key_off: u32,
    ) -> usize {
        let (length, size) = read_uvarint(src.add(off));
        self.builder
            .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

        off + size + length as usize
    }
    #[inline(always)]
    pub unsafe fn slice(
        &mut self,
        src: *const u8,
        off: usize,
        nkind: NodeKind,
        key_len: u32,
        key_off: u32,
        siz: usize,
    ) -> usize {
        let (length, size) = read_uvarint(src.add(off));
        self.builder
            .add(nkind, key_len, key_off, length as u32, (off + size) as u32);

        off + size + (length as usize) * siz
    }
}

/// Defines a node of blog context structure.
///
/// Here:
///  - `kind` denotes a kind of node.
///  - `next` is u32 offset the next sibling.
///  - `child` is u32 offset of the first child.
///  - `data` is u32 offset to the respective log fragment.
#[repr(C)]
pub struct Node {
    kind:    NodeKind,
    next:    u32,
    child:   u32,
    _pad:    u32,
    key_len: u32,
    key_off: u32,
    val_len: u32,
    val_off: u32,
}

const _: () = assert!(std::mem::size_of::<Node>() == 32);

/// Represents a part of kind value in [Node].
/// Splited into three regions:
/// - 0…63 values.
/// - 64…127 slices.
/// - 128…255 hierarchy roots.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    // Values
    Bool           = 0,
    Time           = 1,
    Dur            = 2,
    Int            = 3,
    I8             = 4,
    I16            = 5,
    I32            = 6,
    I64            = 7,
    Uint           = 8,
    U8             = 10,
    U16            = 11,
    U32            = 12,
    U64            = 13,
    F32            = 14,
    F64            = 15,
    Str            = 16,
    Bytes          = 17,
    ErrTxt         = 18,
    ErrTxtFragment = 19,
    ErrLoc         = 20,

    // Slices/arrays/lists or whatever you call them.
    Bools          = 64,
    Ints           = 65,
    I8s            = 66,
    I16s           = 67,
    I32s           = 68,
    I64s           = 69,
    Uints          = 70,
    U8s            = 71,
    U16s           = 72,
    U32s           = 73,
    U64s           = 74,
    F32s           = 75,
    F64s           = 76,
    Strs           = 77,

    // Roots.
    Group          = 128,
    Error          = 129,
    ErrorStageNew  = 130,
    ErrorStageWrap = 131,
    ErrorStageCtx  = 132,
}

impl NodeKind {
    fn string(&self) -> &'static str {
        match self {
            NodeKind::Bool => "bool",
            NodeKind::Time => "time",
            NodeKind::Dur => "dur",
            NodeKind::Int => "int",
            NodeKind::I8 => "i8",
            NodeKind::I16 => "i16",
            NodeKind::I32 => "i32",
            NodeKind::I64 => "i64",
            NodeKind::Uint => "uint",
            NodeKind::U8 => "u8",
            NodeKind::U16 => "u16",
            NodeKind::U32 => "u32",
            NodeKind::U64 => "u64",
            NodeKind::F32 => "f32",
            NodeKind::F64 => "f64",
            NodeKind::Str => "str",
            NodeKind::Bytes => "bytes",
            NodeKind::ErrTxt => "error:Text",
            NodeKind::ErrLoc => "error:Loc",
            NodeKind::Bools => "bools",
            NodeKind::Ints => "ints",
            NodeKind::I8s => "i8s",
            NodeKind::I16s => "i16s",
            NodeKind::I32s => "i32s",
            NodeKind::I64s => "i64s",
            NodeKind::Uints => "uints",
            NodeKind::U8s => "u8s",
            NodeKind::U16s => "u16s",
            NodeKind::U32s => "u32s",
            NodeKind::U64s => "u64s",
            NodeKind::F32s => "f32s",
            NodeKind::F64s => "f64s",
            NodeKind::Strs => "strs",
            NodeKind::Group => "group",
            NodeKind::Error => "error",
            NodeKind::ErrorStageNew => "error:New",
            NodeKind::ErrorStageWrap => "error:Wrap",
            NodeKind::ErrorStageCtx => "error:Ctx",
            &NodeKind::ErrTxtFragment => "error:TextFragment",
        }
    }
}

/// [Tree] represents a logical tree layout in a continuous memory area.
pub struct Tree<'a> {
    ctrl: &'a [u8],
    tree: bool,
}

/// blog context structure builder type.
pub struct TreeBuilder {
    pub ctrl:  Vec<u8>,
    pub stack: Vec<usize>,
    pub last:  isize,
    pub off:   usize,
}

impl TreeBuilder {
    /// Constructs new [TreeBuilder] instance with preallocated data.
    pub fn new() -> Self {
        Self {
            ctrl:  vec![0u8; 4096],
            stack: Vec::with_capacity(16),
            last:  -1,
            off:   0,
        }
    }

    /// Resets the state of builder for further reuse.
    pub unsafe fn reset(&mut self) {
        self.stack.set_len(0);
        self.last = -1;
        self.off = 0;
    }

    /// Adds a new node with given type and data.
    #[inline(always)]
    pub unsafe fn add(
        &mut self,
        kind: NodeKind,
        key_len: u32,
        key_off: u32,
        val_len: u32,
        val_off: u32,
    ) {
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

    /// Starts a new group AFTER root node (kind >=128). There's no check though,
    /// this is up to a user.
    /// This call must be paired with respective leave_group.
    #[inline(always)]
    pub fn enter_group(&mut self) {
        self.stack.push(self.last as usize);
    }

    /// Exits existing group. Must be called somewhere after enter_group.
    #[inline(always)]
    pub unsafe fn leave_group(&mut self) {
        let last = self.last;
        self.last = self.stack.pop().unwrap() as isize;

        if last == self.last {
            // We are closing a root node that has no content.
            let node = self.ctrl.as_mut_ptr().add(last as usize) as *mut Node;
            (*node).child = u32::MAX;
        }
    }

    /// Превращает билд в готовое дерево (Zero-copy ссылки)
    pub unsafe fn finish<'a>(&'a self) -> Tree<'a> {
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
    pub unsafe fn show(&mut self) {
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

#[inline(always)]
pub unsafe fn read_uvarint(ptr: *const u8) -> (u64, usize) {
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

#[cfg(test)]
mod test {
    use crate::ctxview::{NodeKind, TreeBuilder};

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
