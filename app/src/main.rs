//! Framely — add camera-brand frames to photos.
//!
//! The GUI is a thin shell: every pixel is produced by `framely-core`,
//! the same pipeline export uses, so preview and output always agree.

use eframe::egui;
use egui::{FontData, FontDefinitions};

use framely_app::i18n::{CJK_CANDIDATES, find_cjk_font};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 560.0])
            .with_title("Framely"),
        ..Default::default()
    };
    eframe::run_native(
        "Framely",
        options,
        Box::new(|cc| {
            // Chinese renders only via a system CJK font (kept out of the
            // binary); if none is found, the app stays English-only.
            let font = find_cjk_font(CJK_CANDIDATES);
            if let Some(f) = &font {
                let mut fonts = FontDefinitions::default();
                let bytes: Vec<u8> = f.bytes.clone();
                fonts.font_data.insert(
                    "cjk".into(),
                    std::sync::Arc::new(FontData::from_blob(std::sync::Arc::new(bytes), f.index)),
                );
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts.families.entry(family).or_default().push("cjk".into());
                }
                cc.egui_ctx.set_fonts(fonts);
            }
            let app = framely_app::FramelyApp::default().with_cjk_font(font.is_some());
            Ok(Box::new(app))
        }),
    )
}
