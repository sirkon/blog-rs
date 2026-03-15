use std::fmt;

#[repr(u8)]
pub enum Level {
    Invalid(u8),
    Trace = 10,
    Debug = 20,
    Info = 30,
    Warn = 40,
    Error = 50,
    Panic = 60,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Invalid(n) => write!(f, "invalid-level[{}]", n),
            Level::Trace => write!(f, "TRACE"),
            Level::Debug => write!(f, "DEBUG"),
            Level::Info => write!(f, " INFO"),
            Level::Warn => write!(f, " WARN"),
            Level::Error => write!(f, "ERROR"),
            Level::Panic => write!(f, "PANIC"),
        }
    }
}
