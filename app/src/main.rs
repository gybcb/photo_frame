//! Framely — add camera-brand frames to photos.
//!
//! The GUI is a thin shell: every pixel is produced by `framely-core`,
//! the same pipeline export uses, so preview and output always agree.

use eframe::egui;

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
        Box::new(|_cc| Ok(Box::new(framely_app::FramelyApp::default()))),
    )
}
