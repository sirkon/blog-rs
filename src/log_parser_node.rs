use crate::value_kind::{PREDEFINED_KEYS, PREDEFINED_NAME_LOCATION};
use std::slice;

/// Defines a node of blog context structure.
///
/// Here:
///  - `kind` denotes a kind of node.
///  - `next` is u32 offset the next sibling.
///  - `child` is u32 offset of the first child.
///  - `data` is u32 offset to the respective log fragment.
#[repr(C)]
pub(crate) struct Node {
    pub(crate) kind:    NodeKind,
    pub(crate) next:    u32,
    pub(crate) child:   u32,
    pub(crate) _pad:    u32,
    pub(crate) key_len: u32,
    pub(crate) key_off: u32,
    pub(crate) val_len: u32,
    pub(crate) val_off: u32,
}

const _: () = assert!(std::mem::size_of::<Node>() == 32);

impl Node {
    #[inline(always)]
    pub(crate) fn val_as_u64(&self) -> u64 {
        self.val_len as u64 | (self.val_off as u64) << 32
    }

    #[inline(always)]
    pub(crate) unsafe fn val_as_slice(&self, ptr: *const u8) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr.add(self.val_off as usize), self.val_len as usize) }
    }

    #[inline(always)]
    pub(crate) unsafe fn key_as_slice(&self, ptr: *const u8) -> &[u8] {
        unsafe {
            match self.kind {
                NodeKind::ErrLoc => {
                    let lock_key_index = (PREDEFINED_NAME_LOCATION >> 8) - 1;
                    let key = PREDEFINED_KEYS[lock_key_index as usize];
                    return key.as_bytes();
                }
                _ => {}
            }

            if self.key_len != 0 {
                return slice::from_raw_parts(
                    ptr.add(self.key_off as usize),
                    self.key_len as usize,
                );
            }

            if self.key_off >= PREDEFINED_KEYS.len() as u32 {
                return "!unknown-key".as_bytes();
            }

            let key = PREDEFINED_KEYS[self.key_len as usize];
            key.as_bytes()
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn key_as_slice_direct(&self, ptr: *const u8) -> &[u8] {
        unsafe {
            if self.key_len != 0 {
                return slice::from_raw_parts(
                    ptr.add(self.key_off as usize),
                    self.key_len as usize,
                );
            }

            if self.key_off >= PREDEFINED_KEYS.len() as u32 {
                return "!unknown-key".as_bytes();
            }

            let key = PREDEFINED_KEYS[self.key_len as usize];
            key.as_bytes()
        }
    }
}

/// Represents a part of kind value in [Node].
/// Splited into three regions:
/// - 0…63 values.
/// - 64…127 slices.
/// - 128…255 hierarchy roots.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
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
    ErrEmbedText   = 21,

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
    ErrorEmbed     = 130,
    ErrorStageNew  = 131,
    ErrorStageWrap = 132,
    ErrorStageCtx  = 133,
}

impl NodeKind {
    pub(crate) fn string(&self) -> &'static str {
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
            NodeKind::ErrEmbedText => "error:EmbedText",
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
            NodeKind::ErrorEmbed => "error:embed",
            NodeKind::ErrorStageNew => "error:New",
            NodeKind::ErrorStageWrap => "error:Wrap",
            NodeKind::ErrorStageCtx => "error:Ctx",
            &NodeKind::ErrTxtFragment => "error:TextFragment",
        }
    }
}
