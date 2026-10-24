//! Error types for the framing pipeline.

use std::fmt;

#[derive(Debug)]
pub enum FrameError {
    /// The file is not a supported image format (or is corrupt).
    UnsupportedFormat,
    /// Image decoding failed.
    Decode(String),
    /// Image encoding failed.
    Encode(String),
    /// Logo rasterization failed (bad SVG or zero size).
    Render(String),
    Io(std::io::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::UnsupportedFormat => write!(f, "unsupported image format"),
            FrameError::Decode(e) => write!(f, "decode failed: {e}"),
            FrameError::Encode(e) => write!(f, "encode failed: {e}"),
            FrameError::Render(e) => write!(f, "render failed: {e}"),
            FrameError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<std::io::Error> for FrameError {
    fn from(e: std::io::Error) -> Self {
        FrameError::Io(e)
    }
}
