pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Eof,
    UnsupportedType,
    InvalidUtf8(std::str::Utf8Error),
    Serialize(String),
    Deserialize(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eof => write!(f, "Unexpected end of input"),
            Self::UnsupportedType => write!(f, "Unsupported type"),
            Self::InvalidUtf8(e) => write!(f, "Invalid utf8 in input: {e}"),
            Self::Serialize(e) => write!(f, "Serialize error: {e}"),
            Self::Deserialize(e) => write!(f, "Deserialize error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Serialize(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Deserialize(msg.to_string())
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::InvalidUtf8(value)
    }
}

macro_rules! impl_de_error {
    ($($err:path)*) => {
        $(impl From<$err> for Error {
            fn from(value: $err) -> Self {
                Self::Deserialize(value.to_string())
            }
        })*
    };
}

impl_de_error! {
    std::num::ParseIntError std::num::ParseFloatError std::char::ParseCharError
    std::str::ParseBoolError
}
