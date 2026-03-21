use std::fmt;

pub type ValueKind = u64;

// --- GROUP 1: tree nodes and metadata  ---
pub const NEW_NODE: ValueKind = 1;
pub const WRAP_NODE: ValueKind = 2;
pub const WRAP_INHERITED_NODE: ValueKind = 3;
pub const JUST_CONTEXT_NODE: ValueKind = 4;
pub const JUST_CONTEXT_INHERITED_NODE: ValueKind = 5;
pub const LOCATION_NODE: ValueKind = 6;
pub const FOREIGN_ERROR_TEXT: ValueKind = 7;
pub const FOREIGN_ERROR_FORMAT: ValueKind = 8;
pub const PHANTOM_CONTEXT_NODE: ValueKind = 10;

// --- GROUP 2: Payload / base types (32+) ---
pub const BOOL: ValueKind = 32;
pub const TIME: ValueKind = 33;
pub const DURATION: ValueKind = 34;
pub const I: ValueKind = 35;
pub const I8: ValueKind = 36;
pub const I16: ValueKind = 37;
pub const I32: ValueKind = 38;
pub const I64: ValueKind = 39;
pub const U: ValueKind = 40;
pub const U8: ValueKind = 41;
pub const U16: ValueKind = 42;
pub const U32: ValueKind = 43;
pub const U64: ValueKind = 44;
pub const FLOAT32: ValueKind = 45;
pub const FLOAT64: ValueKind = 46;
pub const STRING: ValueKind = 47;
pub const BYTES: ValueKind = 48;
pub const ERROR_RAW: ValueKind = 49;

// --- GROUP 3: Complex structs and slices (64+) ---
pub const ERROR: ValueKind = 64;
pub const ERROR_EMBED: ValueKind = 65;
pub const GROUP: ValueKind = 66;

pub const SLICE_BOOL: ValueKind = 70;
pub const SLICE_I: ValueKind = 71;
pub const SLICE_I8: ValueKind = 72;
pub const SLICE_I16: ValueKind = 73;
pub const SLICE_I32: ValueKind = 74;
pub const SLICE_I64: ValueKind = 75;
pub const SLICE_U: ValueKind = 76;
pub const SLICE_U8: ValueKind = 77;
pub const SLICE_U16: ValueKind = 78;
pub const SLICE_U32: ValueKind = 79;
pub const SLICE_U64: ValueKind = 80;
pub const SLICE_F32: ValueKind = 81;
pub const SLICE_F64: ValueKind = 82;
pub const SLICE_STRING: ValueKind = 83;

pub const MAX: ValueKind = 255;

pub const PREDEFINED_NAME_CONTEXT: ValueKind = 1 << 8;
pub const PREDEFINED_NAME_TEXT: ValueKind = 2 << 8;
pub const PREDEFINED_NAME_LOCATION: ValueKind = 3 << 8;

// There're ValueKind values at 256 and further to represent [Attr] with predefined keys, where their
// lowest byte represents a kind and the upper 7 bytes refer a key index.

pub fn string(k: ValueKind) -> String {
    match k & 0xFF {
        NEW_NODE => "error.New".to_string(),
        WRAP_NODE => "error.Wrap".to_string(),
        WRAP_INHERITED_NODE => "error.Wrap(over foreign)".to_string(),
        JUST_CONTEXT_NODE => "error.Ctx".to_string(),
        JUST_CONTEXT_INHERITED_NODE => "error.Ctx(over foreign)".to_string(),
        LOCATION_NODE => "location".to_string(),
        FOREIGN_ERROR_TEXT => "error.(foreign text)".to_string(),
        FOREIGN_ERROR_FORMAT => "error.(foreign format)".to_string(),
        PHANTOM_CONTEXT_NODE => "errors.Ctx(phantom)".to_string(),
        BOOL => "bool".to_string(),
        TIME => "time.UnixNano".to_string(),
        DURATION => "time.Duration".to_string(),
        I => "int".to_string(),
        I8 => "int8".to_string(),
        I16 => "int16".to_string(),
        I32 => "int32".to_string(),
        I64 => "int64".to_string(),
        U => "uint".to_string(),
        U8 => "uint8".to_string(),
        U16 => "uint16".to_string(),
        U32 => "uint32".to_string(),
        U64 => "uint64".to_string(),
        FLOAT32 => "float32".to_string(),
        FLOAT64 => "float64".to_string(),
        STRING => "string".to_string(),
        BYTES => "[]byte".to_string(),
        ERROR_RAW => "error".to_string(),
        ERROR => "beer.Error".to_string(),
        ERROR_EMBED => "error.Intermixed".to_string(),
        GROUP => "blog.Group".to_string(),
        SLICE_BOOL => "[]bool".to_string(),
        SLICE_I => "[]int".to_string(),
        SLICE_I8 => "[]int8".to_string(),
        SLICE_I16 => "[]int16".to_string(),
        SLICE_I32 => "[]int32".to_string(),
        SLICE_I64 => "[]int64".to_string(),
        SLICE_U => "[]uint".to_string(),
        SLICE_U8 => "[]uint8".to_string(),
        SLICE_U16 => "[]uint16".to_string(),
        SLICE_U32 => "[]uint32".to_string(),
        SLICE_U64 => "[]uint64".to_string(),
        SLICE_F32 => "[]float32".to_string(),
        SLICE_F64 => "[]float64".to_string(),
        SLICE_STRING => "[]string".to_string(),
        _ => {
            // Probably a predefined thing?
            if k >> 8 > 0 {
                let index = k >> 8;
                if index <= PREDEFINED_KEYS.len() as ValueKind {
                    let res = PREDEFINED_KEYS[(index - 1) as usize].to_string();
                    let rem = k << 56 >> 56;
                    if k << 56 >> 56 != 0 {
                        return res + ":" + string(rem).as_str();
                    }
                    return res;
                }
            }
            format!("value-kind-unknown[{}]", k)
        }
    }
}

// PredefinedKeys keys can be set via the extension of kind in the
// higher 7 bytes of uint64. That extended bytes keep an index of
// the key spec in this slice.
pub const PREDEFINED_KEYS: &[&str] = &["@context", "@text", "@location"];

// Нужно использовать новтип для реализации Display
pub struct Kind(pub ValueKind);

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", string(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(NEW_NODE, 1);
        assert_eq!(BOOL, 32);
        assert_eq!(MAX, 255);
        assert_eq!(PREDEFINED_NAME_CONTEXT, 256);
    }

    #[test]
    fn test_display() {
        assert_eq!(string(BOOL), "bool");
        assert_eq!(string(ERROR), "beer.Error");
        assert_eq!(string(SLICE_STRING), "[]string");
        assert_eq!(string(SLICE_U8), "[]uint8");

        // Test unknown
        assert_eq!(string(254), "value-kind-unknown[254]");
        assert_eq!(string(999), "@location:value-kind-unknown[231]");
    }

    #[test]
    fn test_predefined() {
        let location_kind = PREDEFINED_NAME_LOCATION; // index 0
        assert_eq!(string(location_kind), "@location");

        let location_kind = PREDEFINED_NAME_CONTEXT;
        assert_eq!(string(location_kind), "@context");

        let location_kind = PREDEFINED_NAME_TEXT;
        assert_eq!(string(location_kind), "@text");
    }

    #[test]
    fn test_kind_newtype() {
        let kind = Kind(BOOL);
        assert_eq!(kind.to_string(), "bool");
    }
}
