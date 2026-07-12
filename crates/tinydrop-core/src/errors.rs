//! Domain-level error types. `thiserror` here at the domain layer; `anyhow`
//! is reserved for application boundaries (see `ARCHITECTURE.md`).

use std::path::PathBuf;
use thiserror::Error;

/// Crate-wide result alias.
pub type CoreResult<T> = Result<T, CoreError>;

/// Domain errors emitted by `tinydrop-core`.
///
/// Every variant must be translatable into a user-facing message by the UI
/// layer. Do not leak raw engine error strings into the UI without a wrapper.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The input path does not exist or is not a regular file.
    #[error("input not found: {0}")]
    InputNotFound(PathBuf),

    /// A TinyDrop sidecar output was submitted as a new source image.
    #[error("'{0}' is already a TinyDrop output; select the original image instead")]
    AlreadyOptimized(PathBuf),

    /// The input file's format is not supported by TinyDrop.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    /// The requested engine does not support the given format.
    #[error("engine {engine} does not support format {format}")]
    EngineFormatMismatch {
        /// Engine name that was asked to handle the format.
        engine: &'static str,
        /// Format string that was rejected.
        format: String,
    },

    /// I/O failure while reading input or writing output.
    #[error("io error on {path}: {source}")]
    Io {
        /// Path involved in the failure.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Engine-side compression failure (codec, decode, etc.).
    #[error("engine {engine} failed: {message}")]
    Engine {
        /// Engine that produced the failure. Owned so we can carry runtime names.
        engine: String,
        /// Human-readable message safe to surface to the user.
        message: String,
    },

    /// The candidate result failed the configured DSSIM quality gate.
    #[error("candidate failed quality gate: dssim={dssim:.6} > threshold={threshold:.6}")]
    QualityGateFailed {
        /// Measured DSSIM of the candidate.
        dssim: f64,
        /// Configured threshold.
        threshold: f64,
    },

    /// Every candidate failed; we have no winner to return.
    #[error("no candidate produced a usable result")]
    NoWinner,

    /// Caller cancelled the operation.
    #[error("operation cancelled")]
    Cancelled,
}

impl CoreError {
    /// Stable string code for IPC payloads / log filtering.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InputNotFound(_) => "INPUT_NOT_FOUND",
            Self::AlreadyOptimized(_) => "ALREADY_OPTIMIZED",
            Self::UnsupportedFormat(_) => "UNSUPPORTED_FORMAT",
            Self::EngineFormatMismatch { .. } => "ENGINE_FORMAT_MISMATCH",
            Self::Io { .. } => "IO",
            Self::Engine { .. } => "ENGINE",
            Self::QualityGateFailed { .. } => "QUALITY_GATE_FAILED",
            Self::NoWinner => "NO_WINNER",
            Self::Cancelled => "CANCELLED",
        }
    }
}
