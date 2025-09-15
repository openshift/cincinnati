use core::fmt;

/// Result type with the `aes-kw` crate's [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// Errors emitted from the wrap and unwrap operations.
#[derive(Debug)]
pub enum Error {
    /// Input data length invalid.
    InvalidDataSize,

    /// Invalid KEK size.
    InvalidKekSize {
        /// KEK size provided in bytes (expected 8, 12, or 24).
        size: usize,
    },

    /// Output buffer size invalid.
    InvalidOutputSize {
        /// Expected size in bytes.
        expected: usize,
    },

    /// Integrity check did not pass.
    IntegrityCheckFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidDataSize => write!(f, "data must be a multiple of 64 bits for AES-KW and less than 2^32 bytes for AES-KWP"),
            Error::InvalidKekSize { size } => {
                write!(f, "invalid AES KEK size: {}", size)
            }
            Error::InvalidOutputSize { expected } => {
                write!(f, "invalid output buffer size: expected {}", expected)
            }
            Error::IntegrityCheckFailed => {
                write!(f, "integrity check failed")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
