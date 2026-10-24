//! Embedded brand logo assets and font data.

use crate::brand::Brand;

/// How a logo is painted onto the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogoMode {
    /// Single color, following the theme foreground color.
    Mono,
    /// Original brand colors as downloaded.
    Color,
}

/// Returns the logo SVG bytes for a brand, ready for `usvg` parsing.
///
/// Mono variants embed `%FG%` / `%BG%` fill placeholders (theme foreground
/// and background, e.g. Leica's disc takes the fg and its knocked-out
/// wordmark takes the bg); they are substituted here. `Brand::Unknown` has
/// no logo and returns `None` — callers fall back to a text-only frame.
pub fn logo_svg(brand: Brand, mode: LogoMode, fg: [u8; 4], bg: [u8; 4]) -> Option<Vec<u8>> {
    if brand == Brand::Unknown {
        return None;
    }
    let stem = brand.asset_stem();
    let raw: &[u8] = match (stem, mode) {
        ("sony", LogoMode::Mono) => include_bytes!("../../assets/brand/sony.mono.svg"),
        ("sony", LogoMode::Color) => include_bytes!("../../assets/brand/sony.color.svg"),
        ("nikon", LogoMode::Mono) => include_bytes!("../../assets/brand/nikon.mono.svg"),
        ("nikon", LogoMode::Color) => include_bytes!("../../assets/brand/nikon.color.svg"),
        ("canon", LogoMode::Mono) => include_bytes!("../../assets/brand/canon.mono.svg"),
        ("canon", LogoMode::Color) => include_bytes!("../../assets/brand/canon.color.svg"),
        ("leica", LogoMode::Mono) => include_bytes!("../../assets/brand/leica.mono.svg"),
        ("leica", LogoMode::Color) => include_bytes!("../../assets/brand/leica.color.svg"),
        ("hasselblad", LogoMode::Mono) => include_bytes!("../../assets/brand/hasselblad.mono.svg"),
        ("hasselblad", LogoMode::Color) => {
            include_bytes!("../../assets/brand/hasselblad.color.svg")
        }
        ("phaseone", LogoMode::Mono) => include_bytes!("../../assets/brand/phaseone.mono.svg"),
        ("phaseone", LogoMode::Color) => include_bytes!("../../assets/brand/phaseone.color.svg"),
        ("alpa", LogoMode::Mono) => include_bytes!("../../assets/brand/alpa.mono.svg"),
        ("alpa", LogoMode::Color) => include_bytes!("../../assets/brand/alpa.color.svg"),
        _ => return None,
    };
    if mode == LogoMode::Mono {
        let fg_hex = format!("#{:02x}{:02x}{:02x}", fg[0], fg[1], fg[2]);
        let bg_hex = format!("#{:02x}{:02x}{:02x}", bg[0], bg[1], bg[2]);
        let text = String::from_utf8_lossy(raw)
            .replace("%FG%", &fg_hex)
            .replace("%BG%", &bg_hex);
        Some(text.into_bytes())
    } else {
        Some(raw.to_vec())
    }
}

pub const FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
pub const FONT_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Inter-Medium.ttf");

/// The logo's natural width/height aspect for a brand, read from its SVG
/// `viewBox`. `Brand::Unknown` (no logo) reports `1.0`.
pub fn logo_aspect(brand: Brand, mode: LogoMode) -> f64 {
    // Mono variants differ from color in fills only; the viewBox is the
    // same. Substitution happens at return time, so pass neutral colors.
    let Some(svg) = logo_svg(brand, mode, [0, 0, 0, 255], [255, 255, 255, 255]) else {
        return 1.0;
    };
    viewBox_aspect(&svg).unwrap_or(1.0)
}

/// Parse `viewBox="minx miny w h"` out of raw SVG bytes.
fn viewBox_aspect(svg: &[u8]) -> Option<f64> {
    let text = String::from_utf8_lossy(svg);
    let at = text.find("viewBox")?;
    let rest = &text[at + "viewBox".len()..];
    let quoted = rest.trim_start().strip_prefix('=')?.trim_start();
    let inner = quoted
        .strip_prefix('"')
        .or_else(|| quoted.strip_prefix('\''))?;
    let inner = inner.split(['"', '\'']).next()?;
    let mut nums = inner.split_whitespace().filter_map(|t| t.parse::<f64>().ok());
    nums.next()?; // min-x
    nums.next()?; // min-y
    let w = nums.next()?;
    let h = nums.next()?;
    if w > 0.0 && h > 0.0 {
        Some(w / h)
    } else {
        None
    }
}
