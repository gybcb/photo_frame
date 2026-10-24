//! Batch export: run the core pipeline over many files in parallel.
//!
//! Each file is independent, so a failure is recorded and the run
//! continues — a batch must never abort on one broken file. Progress
//! arrives on a channel so the GUI can stay responsive.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use rayon::prelude::*;

use framely_core::assets::LogoMode;
use framely_core::error::FrameError;
use framely_core::exif::PhotoInfo;
use framely_core::jpeg;
use framely_core::layout::layout;
use framely_core::template::{Fields, TemplateId, ThemeConfig};

/// One file finished, with its outcome.
pub struct Done {
    pub path: PathBuf,
    pub ok: bool,
    pub error: Option<String>,
}

/// One frame to render for the batch (shared across all files).
#[derive(Clone)]
pub struct BatchSpec {
    pub template: TemplateId,
    pub theme: ThemeConfig,
    pub logo_mode: LogoMode,
    pub quality: u8,
    /// Extra text line replacing fields (manual entry), if any.
    pub custom_line: Option<String>,
}

/// Render one photo (already decoded) and write it beside `out_dir`.
/// The fields come from the file's own EXIF, so the batch reflects real
/// per-camera data rather than a single hard-coded line.
pub fn export_one(src: &Path, out_dir: &Path, spec: &BatchSpec) -> Result<PathBuf, FrameError> {
    let bytes = std::fs::read(src).map_err(FrameError::Io)?;
    let info = PhotoInfo::from_bytes(&bytes);
    let (photo, _exif_carry) = jpeg::load_bytes(&bytes)?;

    let mut fields = Fields {
        parts: info.fields.parts.clone(),
        custom_line: spec.custom_line.clone(),
    };
    let _ = &mut fields; // fields may be overridden below; keep borrow alive

    let logo_ratio = framely_core::assets::logo_aspect(info.brand, spec.logo_mode);
    let l = layout(
        spec.template,
        photo.width(),
        photo.height(),
        info.brand,
        logo_ratio,
        &spec.theme,
        &fields,
    );
    let out_bytes = jpeg::frame_and_export(
        &photo,
        &l,
        spec.logo_mode,
        if info.exif.is_empty() {
            None
        } else {
            Some(&info.exif)
        },
        spec.quality,
    )?;

    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("photo");
    let target = unique_path(out_dir, stem, "jpg");
    std::fs::write(&target, &out_bytes).map_err(FrameError::Io)?;
    Ok(target)
}

/// A path in `dir` named `{stem}_framed.{ext}`, bumped with `_2`, `_3`…
/// if that name is taken (never overwriting anything).
pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let base = format!("{stem}_framed");
    let mut candidate = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{base}_{n}.{ext}"));
        n += 1;
    }
    candidate
}

/// Run the batch across threads, reporting each completion.
/// Returns `(ok_count, error_count)` when finished.
pub fn run(files: Vec<PathBuf>, out_dir: PathBuf, spec: BatchSpec, tx: mpsc::Sender<Done>) {
    files.par_iter().for_each(|path| {
        let done = match export_one(path, &out_dir, &spec) {
            Ok(target) => Done {
                path: target,
                ok: true,
                error: None,
            },
            Err(e) => Done {
                path: path.clone(),
                ok: false,
                error: Some(e.to_string()),
            },
        };
        let _ = tx.send(done);
    });
}
