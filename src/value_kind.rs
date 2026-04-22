use crate::log_parse::ErrorLogParse;
use crate::log_parse::ErrorLogParse::RecordContextNodePredefinedKeyUnknown;
use num_enum::TryFromPrimitive;
use std::fmt;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    // --- GROUP 1: tree nodes and metadata ---
    NewNode            = 1,
    WrapNode           = 2,
    JustContextNode    = 3,
    LocationNode       = 4,
    ForeignErrorText   = 5,
    PhantomContextNode = 6,
    Group              = 7,
    Error              = 8,
    ErrorEmbed         = 9,
    GroupEnd           = 10,

    // --- GROUP 2: Payload / base types ---
    Bool               = 11,
    Time               = 12,
    Duration           = 13,
    Ivar               = 14,
    I8                 = 15,
    I16                = 16,
    I32                = 17,
    I64                = 18,
    Uvar               = 19,
    U8                 = 20,
    U16                = 21,
    U32                = 22,
    U64                = 23,
    Float32            = 24,
    Float64            = 25,
    String             = 26,
    Bytes              = 27,
    ErrorRaw           = 28,

    // --- GROUP 3: Slices ---
    SliceBool          = 29,
    SliceI8            = 30,
    SliceI16           = 31,
    SliceI32           = 32,
    SliceI64           = 33,
    SliceU8            = 34,
    SliceU16           = 35,
    SliceU32           = 36,
    SliceU64           = 37,
    SliceF32           = 38,
    SliceF64           = 39,
    SliceString        = 40,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum PredefinedKeyCode {
    INVALID = 0,
}

pub(crate) fn predefined_key(code: PredefinedKeyCode) -> &'static str {
    match code {
        PredefinedKeyCode::INVALID => "INVALID",
    }
}

pub(crate) unsafe fn predefined_key_safe(code: u32) -> Result<&'static str, ErrorLogParse> {
    unsafe {
        match PredefinedKeyCode::try_from(code) {
            Ok(x) => Ok(predefined_key(x)),
            Err(_) => Err(RecordContextNodePredefinedKeyUnknown(code)),
        }
    }
}

// There're ValueKind values at 256 and further to represent [Attr] with predefined keys, where their
// lowest byte represents a kind and the upper 7 bytes refer a key index.

#[allow(unused)]
pub(crate) fn is_group_start(k: ValueKind) -> bool {
    match k {
        ValueKind::NewNode
        | ValueKind::WrapNode
        | ValueKind::JustContextNode
        | ValueKind::Group
        | ValueKind::Error
        | ValueKind::ErrorEmbed => true,
        _ => false,
    }
}

pub fn string(k: ValueKind) -> String {
    match k {
        ValueKind::NewNode => "error.New".to_string(),
        ValueKind::WrapNode => "error.Wrap".to_string(),
        ValueKind::JustContextNode => "error.Ctx".to_string(),
        ValueKind::LocationNode => "location".to_string(),
        ValueKind::ForeignErrorText => "error.(foreign text)".to_string(),
        ValueKind::PhantomContextNode => "errors.Ctx(phantom)".to_string(),
        ValueKind::Group => "blog.Group".to_string(),
        ValueKind::Error => "beer.Error".to_string(),
        ValueKind::ErrorEmbed => "error.Intermixed".to_string(),
        ValueKind::GroupEnd => "group.end".to_string(),
        ValueKind::Bool => "bool".to_string(),
        ValueKind::Time => "time.UnixNano".to_string(),
        ValueKind::Duration => "time.Duration".to_string(),
        ValueKind::Ivar => "vaint".to_string(),
        ValueKind::I8 => "int8".to_string(),
        ValueKind::I16 => "int16".to_string(),
        ValueKind::I32 => "int32".to_string(),
        ValueKind::I64 => "int64".to_string(),
        ValueKind::Uvar => "uvarint".to_string(),
        ValueKind::U8 => "uint8".to_string(),
        ValueKind::U16 => "uint16".to_string(),
        ValueKind::U32 => "uint32".to_string(),
        ValueKind::U64 => "uint64".to_string(),
        ValueKind::Float32 => "float32".to_string(),
        ValueKind::Float64 => "float64".to_string(),
        ValueKind::String => "string".to_string(),
        ValueKind::Bytes => "[]byte".to_string(),
        ValueKind::ErrorRaw => "error".to_string(),
        ValueKind::SliceBool => "[]bool".to_string(),
        ValueKind::SliceI8 => "[]int8".to_string(),
        ValueKind::SliceI16 => "[]int16".to_string(),
        ValueKind::SliceI32 => "[]int32".to_string(),
        ValueKind::SliceI64 => "[]int64".to_string(),
        ValueKind::SliceU8 => "[]uint8".to_string(),
        ValueKind::SliceU16 => "[]uint16".to_string(),
        ValueKind::SliceU32 => "[]uint32".to_string(),
        ValueKind::SliceU64 => "[]uint64".to_string(),
        ValueKind::SliceF32 => "[]float32".to_string(),
        ValueKind::SliceF64 => "[]float64".to_string(),
        ValueKind::SliceString => "[]string".to_string(),
    }
}

// Нужно использовать новтип для реализации Display
pub struct Kind(pub ValueKind);

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", string(self.0))
    }
}
