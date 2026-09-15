//! Structured errors for the domain core.

use thiserror::Error;

/// Failures raised by the pure domain layer.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A page marker line could not be parsed.
    #[error("malformed page marker line: {0}")]
    PageMarker(String),
    /// A number in a page marker was invalid.
    #[error("invalid number in page marker: {0}")]
    Number(String),
    /// A static regular expression failed to compile.
    #[error("regex compilation failed: {0}")]
    Regex(String),
}
