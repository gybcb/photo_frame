//! Framely — add camera-brand frames to photos.
//!
//! The GUI is a thin shell: every pixel is produced by `framely-core`,
//! the same pipeline export uses, so preview and output always agree.

mod app;
pub mod batch;
pub mod i18n;
mod panels;

pub use app::FramelyApp;
