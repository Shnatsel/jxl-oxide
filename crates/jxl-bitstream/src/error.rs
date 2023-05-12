#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// Container box size was invalid.
    InvalidBoxSize,
    /// `PadZeroToByte` read non-zero bits.
    NonZeroPadding,
    /// Parsed floating point value was Infinity or NaN.
    InvalidFloat,
    /// Parsed value couldn't be represented with the given enum.
    InvalidEnum {
        name: &'static str,
        value: u32,
    },
    /// The bitstream couldn't be skipped to the given position, mainly due to the direction being
    /// backwards.
    CannotSkip,
    /// The bistream offsed was not aligned to read byte-aligned data.
    NotAligned,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => {
                write!(f, "I/O error: {}", e)
            },
            Self::InvalidBoxSize => write!(f, "invalid box size"),
            Self::NonZeroPadding => {
                write!(f, "PadZeroToByte() read non-zero bits")
            },
            Self::InvalidFloat => {
                write!(f, "F16() read NaN or Infinity")
            },
            Self::InvalidEnum { name, value } => {
                write!(f, "Enum({}) read invalid enum value of {}", name, value)
            },
            Self::CannotSkip => {
                write!(f, "target bookmark already passed")
            },
            Self::NotAligned => {
                write!(f, "bitstream is unaligned")
            },
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
