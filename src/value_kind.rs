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

pub const PREDEFINED_USER_ID: ValueKind = 1 << 8;

// There're ValueKind values at 257 and further to represent [Attr] with predefined keys, where their
// lowest byte represents a kind and the upper 7 bytes refer a key index.

pub fn string(k: ValueKind) -> String {
    match k & 0xFF {
        1 => "error.New".to_string(),
        2 => "error.Wrap".to_string(),
        3 => "error.Wrap(over foreign)".to_string(),
        4 => "error.Ctx".to_string(),
        5 => "errors.Ctx(over foreign)".to_string(),
        6 => "location".to_string(),
        7 => "foreign error text".to_string(),
        8 => "foreign error format".to_string(),
        10 => "errors.Ctx(phantom)".to_string(),
        32 => "bool".to_string(),
        33 => "time.UnixNano".to_string(),
        34 => "time.Duration".to_string(),
        35 => "int".to_string(),
        36 => "int8".to_string(),
        37 => "int16".to_string(),
        38 => "int32".to_string(),
        39 => "int64".to_string(),
        40 => "uint".to_string(),
        41 => "uint8".to_string(),
        42 => "uint16".to_string(),
        43 => "uint32".to_string(),
        44 => "uint64".to_string(),
        45 => "float32".to_string(),
        46 => "float64".to_string(),
        47 => "string".to_string(),
        48 => "[]byte".to_string(),
        49 => "error".to_string(),
        64 => "beer.Error".to_string(),
        65 => "ForeignWrap(beef.Error)".to_string(),
        66 => "blog.Group".to_string(),
        70 => "[]bool".to_string(),
        71 => "[]int".to_string(),
        72 => "[]int8".to_string(),
        73 => "[]int16".to_string(),
        74 => "[]int32".to_string(),
        75 => "[]int64".to_string(),
        76 => "[]uint".to_string(),
        77 => "[]uint8".to_string(),
        78 => "[]uint16".to_string(),
        79 => "[]uint32".to_string(),
        80 => "[]uint64".to_string(),
        81 => "[]float32".to_string(),
        82 => "[]float64".to_string(),
        83 => "[]string".to_string(),
        _ => {
            // Probably a predefined thing?
            match k {
                PREDEFINED_USER_ID => (),
                _ => return format!("spec-kind-unknown[{}]", k),
            }
            PREDEFINED_KEYS[(k >> 8) as usize - 1].to_string()
        }
    }
}

// PredefinedKeys keys can be set via the extension of kind in the
// higher 7 bytes of uint64. That extended bytes keep an index of
// the key spec in this slice.
pub const PREDEFINED_KEYS: &[&str] = &[
    "user-id",
];

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
        assert_eq!(PREDEFINED_USER_ID, 256);
    }

    #[test]
    fn test_display() {
        assert_eq!(string(BOOL), "bool");
        assert_eq!(string(ERROR), "beer.Error");
        assert_eq!(string(SLICE_STRING), "[]string");
        assert_eq!(string(SLICE_U8), "[]uint8");

        // Test unknown
        assert_eq!(string(999), "spec-kind-unknown[999]");
    }

    #[test]
    fn test_predefined() {
        let user_id_kind = PREDEFINED_USER_ID; // index 0
        assert_eq!(string(user_id_kind), "user-id");
    }

    #[test]
    fn test_kind_newtype() {
        let kind = Kind(BOOL);
        assert_eq!(kind.to_string(), "bool");
    }
}