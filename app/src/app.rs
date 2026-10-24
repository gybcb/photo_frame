//! Application state and layout: open → extract EXIF → live preview → export.
//!
//! Everything drawn here comes from `framely-core::render::compose`, the
//! exact function the batch/export path uses — the preview canvas is just
//! downscaled to fit the window.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, Color32, RichText, TextureHandle, TextureOptions, vec2};
use framely_core::assets::{LogoMode, logo_aspect};
use framely_core::exif::PhotoInfo;
use framely_core::jpeg;
use framely_core::layout::layout;
use framely_core::template::{Field, TemplateId, ThemeConfig};

use crate::batch::{self, BatchSpec, Done};
use crate::i18n::{Action, Config, Lang, Msg, config_path, load_config};
use crate::panels::{FieldOverrides, fields_panel, theme_panel};

/// Longest side kept for the resident preview photo, and the largest
/// preview texture ever built. A stitched ~300MP panorama decodes to
/// ~1.1GB of RGBA; keeping only this size down to ~16MB makes such files
/// ordinary to work with — exports re-read the file at full resolution.
const PREVIEW_MAX_SIDE: u32 = 2048;

/// The currently loaded photo and everything derived from it.
#[derive(Default)]
pub struct Loaded {
    path: Option<PathBuf>,
    info: Option<PhotoInfo>,
    /// Orientation-corrected photo, downscaled to preview size on load.
    /// Shared via `Arc` so each render doesn't copy it to the worker.
    photo: Option<Arc<image::RgbaImage>>,
}

/// One-shot result of a background render.
type Rendered = Result<(TextureHandle, (u32, u32)), String>;

pub struct FramelyApp {
    loaded: Loaded,
    template: TemplateId,
    themes: Vec<(TemplateId, ThemeConfig)>,
    logo_mode: LogoMode,
    enabled: Vec<Field>,
    overrides: FieldOverrides,
    quality: u8,

    /// Preview texture + its pixel size (aspect source).
    preview: Option<TextureHandle>,
    preview_size: (u32, u32),
    /// Render error shown in the viewport.
    render_error: Option<Msg>,
    /// Status line (save result, export counts…), localized on display.
    status: Option<Msg>,
    /// A render is in flight; don't start another.
    rendering: bool,
    rx: Option<Receiver<Rendered>>,

    /// Preset folder for save/load buttons.
    preset_dir: PathBuf,

    batch_files: Vec<PathBuf>,
    batch_total: usize,
    batch_done: usize,
    batch_errors: Vec<String>,
    batch_rx: Option<Receiver<Done>>,
    batch_running: bool,

    /// Last render signature (see `config_sig`).
    render_sig: u64,

    /// UI language (persisted choice, or system locale on first run).
    lang: Lang,
    /// A CJK system font exists, so the language toggle can render Chinese.
    cjk_ok: bool,
}

impl Default for FramelyApp {
    /// Seeds each template's theme with its defaults; every other field
    /// starts empty. (Cannot be `derive(Default)` + a `new()` that
    /// back-fills — that pair recursed, overflowing the stack at startup.)
    fn default() -> Self {
        FramelyApp {
            loaded: Loaded::default(),
            template: TemplateId::default(),
            themes: TemplateId::ALL
                .iter()
                .map(|t| (*t, ThemeConfig::default_for(*t)))
                .collect(),
            logo_mode: LogoMode::default(),
            enabled: vec![
                Field::Lens,
                Field::Focal,
                Field::Aperture,
                Field::Shutter,
                Field::Iso,
            ],
            overrides: FieldOverrides::default(),
            quality: 92,
            preview: None,
            preview_size: (0, 0),
            render_error: None,
            status: None,
            rendering: false,
            rx: None,
            preset_dir: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(".")),
            batch_files: Vec::new(),
            batch_total: 0,
            batch_done: 0,
            batch_errors: Vec::new(),
            batch_rx: None,
            batch_running: false,
            render_sig: 0,
            lang: {
                let cfg = load_config(&config_path());
                cfg.lang
                    .unwrap_or_else(|| Lang::detect(std::env::var("LANG").ok().as_deref()))
            },
            cjk_ok: false,
        }
    }
}

impl FramelyApp {
    /// Record that a CJK font was installed (see `crate::i18n`), which
    /// is what unlocks the language toggle.
    pub fn with_cjk_font(mut self, installed: bool) -> Self {
        // Without a CJK font, Chinese couldn't render: don't offer it.
        if !installed && self.lang == Lang::Zh {
            self.lang = Lang::En;
        }
        self.cjk_ok = installed;
        self
    }
}

impl FramelyApp {
    pub fn new() -> FramelyApp {
        FramelyApp::default()
    }

    fn theme(&self) -> ThemeConfig {
        self.themes
            .iter()
            .find(|(t, _)| *t == self.template)
            .map(|(_, c)| c.clone())
            .unwrap_or_else(|| ThemeConfig::default_for(self.template))
    }

    /// Index of the current template inside `themes`.
    fn theme_index(&self) -> usize {
        self.themes
            .iter()
            .position(|(t, _)| *t == self.template)
            .unwrap_or(0)
    }

    /// Everything that affects the rendered preview, hashed.
    fn config_sig(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let i = self.theme_index();
        let mut h = DefaultHasher::new();
        self.template.hash(&mut h);
        self.logo_mode.hash(&mut h);
        self.enabled.hash(&mut h);
        self.themes[i].1.hash(&mut h);
        self.overrides.hash(&mut h);
        match (&self.loaded.photo, &self.loaded.path) {
            (Some(p), path) => {
                (p.width(), p.height(), path).hash(&mut h);
            }
            (None, _) => 0_u32.hash(&mut h),
        }
        h.finish()
    }

    /// Replace the current template's theme (panels borrow `themes` mutably
    /// on their own, so this stays a whole-slot write).
    fn set_theme(&mut self, cfg: ThemeConfig) {
        let i = self.theme_index();
        self.themes[i].1 = cfg;
    }

    // ---- loading -------------------------------------------------------

    fn open_paths(&mut self, paths: Vec<PathBuf>, ctx: &egui::Context) {
        // First path becomes the live preview; the rest join the batch list.
        let mut paths = paths;
        if paths.is_empty() {
            return;
        }
        let first = paths.remove(0);
        self.load_file(&first, ctx);
        if !paths.is_empty() {
            self.batch_files.extend(paths);
            self.status = Some(Msg::BatchQueued(self.batch_files.len()));
        }
    }

    fn load_file(&mut self, path: &Path, ctx: &egui::Context) {
        match fs::read(path) {
            Ok(bytes) => {
                let info = PhotoInfo::from_bytes(&bytes);
                match jpeg::load_bytes(&bytes) {
                    Ok((photo, _)) => {
                        // Keep only preview-size pixels resident; export
                        // re-reads the file at full resolution. Decoding
                        // still happens at full size (framing needs the
                        // orientation-corrected pixels), but the big
                        // buffer is freed right here instead of held for
                        // the whole session.
                        let photo = downscale(photo, PREVIEW_MAX_SIDE);
                        self.loaded = Loaded {
                            path: Some(path.to_path_buf()),
                            info: Some(info),
                            photo: Some(Arc::new(photo)),
                        };
                        self.overrides = FieldOverrides::default();
                        self.render(ctx);
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        self.render_error = Some(Msg::Failed(Action::Decode, e.to_string()));
                        self.loaded = Loaded::default();
                    }
                }
            }
            Err(e) => self.render_error = Some(Msg::Failed(Action::Read, e.to_string())),
        }
    }

    // ---- rendering -----------------------------------------------------

    /// Kick off a preview render on a worker thread (single shot: the
    /// result channel is drained once).
    fn render(&mut self, ctx: &egui::Context) {
        self.render_error = None;
        let (Some(photo), Some(info)) = (self.loaded.photo.clone(), self.loaded.info.clone())
        else {
            self.preview = None;
            return;
        };
        let fields = self.overrides.apply(&info.fields);
        let fields = apply_enable(fields, &self.enabled);
        let spec = (
            self.template,
            self.theme(),
            info.brand,
            logo_aspect(info.brand, self.logo_mode),
            fields,
            self.logo_mode,
        );
        if self.rendering {
            return; // one in flight is enough; the next repaint re-checks
        }
        self.rendering = true;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<(TextureHandle, (u32, u32)), String> {
                // The resident photo is already preview-sized, so the
                // frame is composed (and its text/logo rasterized) at
                // preview scale: sharp, and a few tens of MB regardless
                // of how huge the source file is.
                let l = layout(
                    spec.0,
                    photo.width(),
                    photo.height(),
                    spec.2,
                    spec.3,
                    &spec.1,
                    &spec.4,
                );
                let (w, h) = (l.canvas_width, l.canvas_height);
                let canvas =
                    framely_core::render::compose(&photo, &l, spec.5).map_err(|e| e.to_string())?;
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [canvas.width() as usize, canvas.height() as usize],
                    canvas.as_raw(),
                );
                let tex = ctx.load_texture("framed", img, TextureOptions::LINEAR);
                Ok((tex, (w, h)))
            })();
            let _ = tx.send(result);
        });
    }

    fn poll_render(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.rx
            && let Ok(res) = rx.try_recv()
        {
            match res {
                Ok((tex, size)) => {
                    self.preview = Some(tex);
                    self.preview_size = size;
                }
                Err(e) => self.render_error = Some(Msg::Failed(Action::Render, e)),
            }
            self.rx = None;
            self.rendering = false;
            ctx.request_repaint();
        }
    }

    // ---- presets -------------------------------------------------------

    fn save_preset(&mut self) {
        let Some(dir) = rfd::FileDialog::new()
            .set_directory(&self.preset_dir)
            .set_file_name("framely-theme.json")
            .save_file()
        else {
            return;
        };
        self.preset_dir = dir.parent().map(Path::to_path_buf).unwrap_or(dir.clone());
        let payload = PresetFile {
            template: self.template,
            theme: self.theme(),
            enabled: self.enabled.clone(),
        };
        match serde_json::to_string_pretty(&payload) {
            Ok(json) => {
                self.status = match fs::write(&dir, json) {
                    Ok(()) => Some(Msg::PresetSaved(dir.display().to_string())),
                    Err(e) => Some(Msg::Failed(Action::Write, e.to_string())),
                };
            }
            Err(e) => self.status = Some(Msg::Failed(Action::Serialize, e.to_string())),
        }
    }

    fn load_preset(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .set_directory(&self.preset_dir)
            .pick_file()
        else {
            return;
        };
        self.preset_dir = path.parent().map(Path::to_path_buf).unwrap_or(path.clone());
        match fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<PresetFile>(&s).map_err(|e| e.to_string()))
        {
            Ok(p) => {
                self.template = p.template;
                self.set_theme(p.theme);
                self.enabled = p.enabled;
                self.status = Some(Msg::PresetLoaded(path.display().to_string()));
                self.render(ctx);
            }
            Err(e) => self.status = Some(Msg::Failed(Action::PresetLoad, e.to_string())),
        }
    }

    // ---- single export -------------------------------------------------

    fn export_current(&mut self) {
        let (Some(path), Some(info)) = (self.loaded.path.clone(), self.loaded.info.clone()) else {
            return;
        };
        let Some(out) = rfd::FileDialog::new()
            .set_file_name(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| format!("{s}_framed.jpg"))
                    .unwrap_or_default(),
            )
            .add_filter("JPEG", &["jpg"])
            .save_file()
        else {
            return;
        };
        match self.frame_current_bytes() {
            Ok(bytes) => match fs::write(&out, &bytes) {
                Ok(()) => {
                    self.status = Some(Msg::Exported(
                        out.display().to_string(),
                        !info.exif.is_empty(),
                    ));
                }
                Err(e) => self.status = Some(Msg::Failed(Action::Write, e.to_string())),
            },
            Err(e) => self.status = Some(Msg::Failed(Action::Export, e.to_string())),
        }
    }

    /// Export at the file's true full resolution: re-read and re-decode
    /// (the resident preview photo is downscaled), compose, then drop
    /// the decoded photo *before* encoding so the export peak is
    /// canvas + encoded bytes rather than photo + canvas + bytes.
    fn frame_current_bytes(&self) -> Result<Vec<u8>, framely_core::error::FrameError> {
        let info = self.loaded.info.as_ref().unwrap();
        let path = self.loaded.path.as_ref().unwrap();
        let fields = self.overrides.apply(&info.fields);
        let fields = apply_enable(fields, &self.enabled);
        let bytes = fs::read(path).map_err(framely_core::error::FrameError::Io)?;
        let (photo, exif) = jpeg::load_bytes(&bytes)?;
        let l = layout(
            self.template,
            photo.width(),
            photo.height(),
            info.brand,
            logo_aspect(info.brand, self.logo_mode),
            &self.theme(),
            &fields,
        );
        let canvas = framely_core::render::compose(&photo, &l, self.logo_mode)?;
        drop(photo);
        jpeg::encode_canvas(canvas, Some(&exif), self.quality)
    }

    // ---- batch ---------------------------------------------------------

    fn start_batch(&mut self) {
        if self.batch_files.is_empty() {
            self.status = Some(Msg::EmptyBatch);
            return;
        }
        let Some(dir) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let files = std::mem::take(&mut self.batch_files);
        self.batch_total = files.len();
        self.batch_done = 0;
        self.batch_errors.clear();
        self.batch_running = true;
        let spec = BatchSpec {
            template: self.template,
            theme: self.theme(),
            logo_mode: self.logo_mode,
            quality: self.quality,
            custom_line: self
                .overrides
                .use_custom_line
                .then(|| self.overrides.custom_line.clone())
                .filter(|s| !s.is_empty()),
        };
        let (tx, rx) = mpsc::channel();
        self.batch_rx = Some(rx);
        std::thread::spawn(move || batch::run(files, dir, spec, tx));
    }

    fn poll_batch(&mut self, ctx: &egui::Context) {
        let mut alive = false;
        if let Some(rx) = &self.batch_rx {
            while let Ok(d) = rx.try_recv() {
                self.batch_done += 1;
                if !d.ok
                    && let Some(e) = d.error
                {
                    self.batch_errors.push(format!("{}: {e}", d.path.display()));
                }
                alive = true;
            }
            if self.batch_done >= self.batch_total {
                self.batch_running = false;
                self.batch_rx = None;
                self.status = Some(Msg::BatchDone {
                    ok: self.batch_done - self.batch_errors.len(),
                    total: self.batch_total,
                    failed: self.batch_errors.len(),
                });
            }
        }
        if alive {
            ctx.request_repaint();
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PresetFile {
    template: TemplateId,
    theme: ThemeConfig,
    enabled: Vec<Field>,
}

/// A stand-in info for the "nothing loaded" state of the fields panel.
fn empty_info() -> PhotoInfo {
    PhotoInfo {
        make: String::new(),
        model: String::new(),
        brand: framely_core::brand::Brand::Unknown,
        orientation: 1,
        fields: Default::default(),
        exif: Vec::new(),
    }
}

/// Drop disabled fields from a merged field list.
fn apply_enable(
    mut f: framely_core::template::Fields,
    enabled: &[Field],
) -> framely_core::template::Fields {
    if f.custom_line.is_some() {
        return f;
    }
    f.parts.retain(|(k, _)| enabled.contains(k));
    f
}

fn downscale(img: image::RgbaImage, max_side: u32) -> image::RgbaImage {
    let (w, h) = (img.width(), img.height());
    let long = w.max(h);
    if long <= max_side {
        return img;
    }
    let s = max_side as f32 / long as f32;
    image::imageops::resize(
        &img,
        ((w as f32 * s) as u32).max(1),
        ((h as f32 * s) as u32).max(1),
        image::imageops::FilterType::Triangle,
    )
}

impl eframe::App for FramelyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_render(&ctx);
        self.poll_batch(&ctx);

        // Drag & drop: any dropped image file opens.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            let allowed = |ext: Option<&std::ffi::OsStr>| {
                matches!(
                    ext.and_then(|e| e.to_str()),
                    Some("jpg" | "jpeg" | "tif" | "tiff" | "png")
                )
            };
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| allowed(p.extension()))
                .collect()
        });
        if !dropped.is_empty() {
            self.open_paths(dropped, &ctx);
        }

        // --- menu bar ----------------------------------------------------
        let mut open_clicked = false;
        let mut export_clicked = false;
        let mut save_preset_clicked = false;
        let mut load_preset_clicked = false;
        let mut lang_clicked = false;
        let lang = self.lang;
        let cjk_ok = self.cjk_ok;
        egui::Panel::top("menubar").show(ui, |ui| {
            ui.horizontal(|ui| {
                open_clicked |= ui.button(lang.select("Open…", "打开…")).clicked();
                export_clicked |= ui
                    .button(lang.select("Export", "导出"))
                    .on_hover_text(lang.select("save the framed current photo", "保存当前加框照片"))
                    .clicked();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    load_preset_clicked |=
                        ui.button(lang.select("Load preset", "载入预设")).clicked();
                    save_preset_clicked |=
                        ui.button(lang.select("Save preset", "保存预设")).clicked();
                    if cjk_ok
                        && ui
                            .button(lang.switch_label())
                            .on_hover_text(lang.select("switch language", "切换语言"))
                            .clicked()
                    {
                        lang_clicked = true;
                    }
                });
            });
        });

        // --- status bar ---------------------------------------------------
        let mut batch_clicked = false;
        let status = self.status.clone();
        let batch_running = self.batch_running;
        let (batch_done, batch_total) = (self.batch_done, self.batch_total);
        let n_errors = self.batch_errors.len();
        let photo_loaded = self.loaded.photo.is_some();
        let queued = self.batch_files.len();
        let FramelyApp { quality, .. } = self;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(m) = &status {
                    ui.label(RichText::new(lang.text(m)).weak());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(photo_loaded, |ui| {
                        batch_clicked |= ui
                            .button(lang.select("Batch export…", "批量导出…"))
                            .on_hover_text(lang.say(
                                format!("{queued} files queued"),
                                format!("已排队 {queued} 张"),
                            ))
                            .clicked();
                        ui.add(
                            egui::Slider::new(quality, 60..=100)
                                .text(lang.select("JPEG quality", "JPEG 质量")),
                        );
                    });
                });
            });
            if batch_running {
                let frac = batch_done as f32 / batch_total.max(1) as f32;
                ui.add(
                    egui::ProgressBar::new(frac)
                        .text(format!("{batch_done} / {batch_total}  ({n_errors} failed)")),
                );
            }
        });

        // --- left: theme ---------------------------------------------------
        let mut reset_clicked = false;
        let FramelyApp {
            template,
            themes,
            logo_mode,
            enabled,
            ..
        } = self;
        egui::Panel::left("theme").exact_size(230.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                theme_panel(ui, lang, template, themes, logo_mode, enabled);
                ui.add_space(10.0);
                reset_clicked |= ui
                    .button(lang.select("Reset theme defaults", "恢复主题默认"))
                    .clicked();
                ui.add_space(6.0);
                open_clicked |= ui.button(lang.select("Open…", "打开…")).clicked();
            });
        });

        // --- right: fields ---------------------------------------------------
        let info = self.loaded.info.clone().unwrap_or_else(empty_info);
        let loaded_none = self.loaded.path.is_none();
        let rendering = self.rendering;
        let render_err = self.render_error.clone();
        let FramelyApp { overrides, .. } = self;
        let mut export_clicked2 = false;
        egui::Panel::right("fields")
            .exact_size(280.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    fields_panel(ui, lang, &info, overrides);
                    ui.add_space(10.0);
                    ui.weak(if rendering {
                        lang.select("rendering…", "渲染中…").to_string()
                    } else if let Some(e) = &render_err {
                        lang.text(e)
                    } else {
                        String::new()
                    });
                    if loaded_none {
                        ui.add_space(10.0);
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(lang.select(
                                    "drop a photo here\n(or use Open…)",
                                    "把照片拖到这里\n（或点“打开…”）",
                                ))
                                .weak()
                                .heading(),
                            )
                        });
                    }
                    export_clicked2 |= ui
                        .button(lang.select("Export current", "导出当前"))
                        .clicked();
                });
            });

        // --- center: preview -------------------------------------------------
        let preview = self.preview.clone();
        let (pw, ph) = self.preview_size;
        let brand_label = self.loaded.info.as_ref().map(|i| format!("{:?}", i.brand));
        let err = self.render_error.clone();
        egui::CentralPanel::default().show(ui, |ui| {
            if let (Some(tex), Some(brand)) = (preview, brand_label) {
                let avail = ui.available_size();
                let scale = (avail.x / pw as f32)
                    .min(avail.y / ph as f32)
                    .clamp(0.01, 1.0);
                let size = vec2(pw as f32 * scale, ph as f32 * scale);
                let rect = egui::Align2::CENTER_CENTER
                    .align_size_within_rect(size, ui.available_rect_before_wrap());
                ui.allocate_rect(rect, egui::Sense::hover());
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                ui.painter().text(
                    rect.min + vec2(8.0, 12.0),
                    egui::Align2::LEFT_TOP,
                    brand,
                    egui::FontId::proportional(11.0),
                    Color32::from_gray(140),
                );
            } else if let Some(e) = err {
                let msg = lang.text(&e);
                ui.centered_and_justified(|ui| ui.colored_label(Color32::LIGHT_RED, msg));
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Framely").heading());
                    ui.label(lang.select(
                        "drop a JPEG / TIFF / PNG, or use Open…",
                        "把 JPEG / TIFF / PNG 拖进来，或点“打开”…",
                    ));
                });
            }
        });

        // --- deferred actions (panels only set flags; self is free now) ------
        if lang_clicked {
            self.lang = match self.lang {
                Lang::En => Lang::Zh,
                Lang::Zh => Lang::En,
            };
            let cfg = Config {
                lang: Some(self.lang),
            };
            let _ = crate::i18n::save_config(&config_path(), &cfg);
            ctx.request_repaint();
        }
        if reset_clicked {
            let t = self.template;
            self.set_theme(ThemeConfig::default_for(t));
        }
        if open_clicked
            && let Some(paths) = rfd::FileDialog::new()
                .add_filter("Images", &["jpg", "jpeg", "tif", "tiff", "png"])
                .pick_files()
        {
            self.open_paths(paths, &ctx);
        }
        if export_clicked || export_clicked2 {
            self.export_current();
        }
        if save_preset_clicked {
            self.save_preset();
        }
        if load_preset_clicked {
            self.load_preset(&ctx);
        }
        if batch_clicked {
            self.start_batch();
        }

        // Re-render when anything that affects the frame changed.
        let sig = self.config_sig();
        if sig != self.render_sig {
            self.render_sig = sig;
            self.render(&ctx);
        }
    }
}
