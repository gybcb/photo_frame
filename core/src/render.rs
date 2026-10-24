//! CPU compositing: fills the frame background, pastes the photo, rasterizes
//! the brand logo at its target size, and draws the parameter text.
//!
//! The same `compose` runs for preview and export, so what the user sees is
//! exactly what is written — only the canvas scale differs.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use fontdue::{Font, FontSettings, Metrics};

use crate::assets::{FONT_REGULAR, LogoMode, logo_svg};
use crate::brand::Brand;
use crate::error::FrameError;
use crate::layout::{Align, DrawOp, FrameLayout};

thread_local! {
    static FONTS: RefCell<Option<Font>> = const { RefCell::new(None) };
}

fn with_font<R>(f: impl FnOnce(&Font) -> R) -> R {
    FONTS.with(|slot| {
        let mut slot = slot.borrow_mut();
        let font = slot.get_or_insert_with(|| {
            Font::from_bytes(FONT_REGULAR, FontSettings::default())
                .expect("embedded font must parse")
        });
        f(font)
    })
}

/// A straight-alpha RGBA sprite.
struct Sprite {
    width: u32,
    height: u32,
    pixels: Vec<[u8; 4]>,
}

/// Shared rasterized-logo cache (bounded).
static LOGOS: LazyLock<Mutex<HashMap<LogoKey, Sprite>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(PartialEq, Eq, Hash, Clone)]
struct LogoKey {
    stem: &'static str,
    mode: u8,
    fg: [u8; 4],
    bg: [u8; 4],
    width: u32,
    height: u32,
}

/// A rasterized glyph: coverage mask plus its metrics.
type Glyph = (Metrics, Vec<u8>);

/// Shared rasterized-glyph cache (bounded).
static GLYPHS: LazyLock<Mutex<HashMap<(char, u32), Glyph>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Compose `photo` into the framed `layout`, returning the full canvas RGBA.
pub fn compose(
    photo: &image::RgbaImage,
    layout: &FrameLayout,
    logo_mode: LogoMode,
) -> Result<image::RgbaImage, FrameError> {
    let (w, h) = (layout.canvas_width, layout.canvas_height);
    let mut canvas = image::RgbaImage::from_pixel(w, h, image::Rgba(layout.background));

    // 1. photo
    let (px, py, pw, ph) = layout.photo_rect;
    if pw != photo.width() || ph != photo.height() {
        return Err(FrameError::Render(format!(
            "photo {}x{} does not match layout {}x{}",
            photo.width(),
            photo.height(),
            pw,
            ph
        )));
    }
    // Row-wise bulk copies: same result as a per-pixel put, an order of
    // magnitude faster (pixels are contiguous 4-byte RGBA on both sides).
    let (src, dst) = (photo.as_raw(), canvas.as_mut());
    let row = pw as usize * 4;
    for y in 0..ph as usize {
        let d = (py as usize + y) * w as usize * 4 + px as usize * 4;
        dst[d..d + row].copy_from_slice(&src[y * row..(y + 1) * row]);
    }

    let fg = text_color(layout);

    // 2. ops in order (logo then text, as emitted by the layout)
    for op in &layout.ops {
        match op {
            DrawOp::Logo {
                brand,
                x,
                y,
                width,
                height,
            } => blit_logo(
                &mut canvas,
                *brand,
                logo_mode,
                fg,
                layout.background,
                (*x, *y, *width, *height),
            )?,
            DrawOp::Text {
                content,
                size_px,
                color,
                align,
                rect,
            } => draw_text(&mut canvas, content, *size_px, *color, *align, *rect),
        }
    }

    Ok(canvas)
}

fn text_color(layout: &FrameLayout) -> [u8; 4] {
    layout
        .ops
        .iter()
        .find_map(|o| match o {
            DrawOp::Text { color, .. } => Some(*color),
            _ => None,
        })
        .unwrap_or(layout.background)
}

/// Rasterize the brand logo to (width x height) and source-over it at (x, y).
fn blit_logo(
    canvas: &mut image::RgbaImage,
    brand: Brand,
    mode: LogoMode,
    fg: [u8; 4],
    bg: [u8; 4],
    (x, y, width, height): (u32, u32, u32, u32),
) -> Result<(), FrameError> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let (fg_c, bg_c) = match mode {
        LogoMode::Mono => (fg, bg),
        // Color variant ignores the theme colors; placeholders never occur.
        LogoMode::Color => ([0, 0, 0, 255], [255, 255, 255, 255]),
    };
    let key = LogoKey {
        stem: brand.asset_stem(),
        mode: mode as u8,
        fg: fg_c,
        bg: bg_c,
        width,
        height,
    };

    let sprite = {
        let mut guard = LOGOS.lock().unwrap();
        match guard.get(&key) {
            Some(s) => s.clone_sprite(),
            None => {
                let s = rasterize_logo(brand, mode, fg_c, bg_c, width, height)?;
                if guard.len() >= 48 {
                    guard.clear();
                }
                guard.insert(key, s.clone_sprite());
                s
            }
        }
    };

    blend(canvas, &sprite, x, y);
    Ok(())
}

impl Sprite {
    fn clone_sprite(&self) -> Sprite {
        Sprite {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
        }
    }
}

fn rasterize_logo(
    brand: Brand,
    mode: LogoMode,
    fg: [u8; 4],
    bg: [u8; 4],
    width: u32,
    height: u32,
) -> Result<Sprite, FrameError> {
    let Some(svg) = logo_svg(brand, mode, fg, bg) else {
        return Ok(Sprite {
            width,
            height,
            pixels: vec![[0, 0, 0, 0]; (width * height) as usize],
        });
    };
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg, &opts)
        .map_err(|e| FrameError::Render(format!("logo svg parse: {e}")))?;
    let src = tree.size();
    let sx = width as f32 / src.width();
    let sy = height as f32 / src.height();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| FrameError::Render("bad logo size".into()))?;
    resvg::render(
        &tree,
        usvg::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    // tiny-skia stores premultiplied RGBA; un-premultiply for our blend.
    let pixels = pixmap
        .pixels()
        .iter()
        .map(|p| {
            let a = p.alpha();
            if a == 0 {
                [0, 0, 0, 0]
            } else {
                [
                    unpremul(p.red(), a),
                    unpremul(p.green(), a),
                    unpremul(p.blue(), a),
                    a,
                ]
            }
        })
        .collect();
    Ok(Sprite {
        width,
        height,
        pixels,
    })
}

fn unpremul(c: u8, a: u8) -> u8 {
    ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8
}

/// Source-over blend of a straight-alpha sprite onto the canvas.
fn blend(canvas: &mut image::RgbaImage, logo: &Sprite, x: u32, y: u32) {
    for j in 0..logo.height {
        let cy = y + j;
        if cy >= canvas.height() {
            break;
        }
        for i in 0..logo.width {
            let cx = x + i;
            if cx >= canvas.width() {
                break;
            }
            let [r, g, b, a] = logo.pixels[(j * logo.width + i) as usize];
            if a == 0 {
                continue;
            }
            canvas.put_pixel(cx, cy, over([r, g, b], a, canvas.get_pixel(cx, cy).0));
        }
    }
}

fn over(src: [u8; 3], sa: u8, dst: [u8; 4]) -> image::Rgba<u8> {
    let na = sa as u32;
    let c = |sc: u8, dc: u8| ((sc as u32 * na + dc as u32 * (255 - na)) / 255) as u8;
    image::Rgba([c(src[0], dst[0]), c(src[1], dst[1]), c(src[2], dst[2]), 255])
}

/// Measure the painted width of `text` at `size_px` (advances + kerning).
pub fn measure_text(text: &str, size_px: u32) -> u32 {
    with_font(|font| {
        let size = size_px as f32;
        let mut width = 0.0f32;
        let mut prev: Option<char> = None;
        for ch in text.chars() {
            if let Some(p) = prev {
                width += font.horizontal_kern(p, ch, size).unwrap_or(0.0);
            }
            width += font.rasterize(ch, size).0.advance_width;
            prev = Some(ch);
        }
        width.round().max(0.0) as u32
    })
}

/// Layout and paint one line of text, aligned inside its region box.
fn draw_text(
    canvas: &mut image::RgbaImage,
    content: &str,
    size_px: u32,
    color: [u8; 4],
    align: Align,
    rect: (u32, u32, u32, u32),
) {
    if content.is_empty() || size_px < 1 {
        return;
    }
    let (rx, ry, rw, rh) = rect;
    let text_w = measure_text(content, size_px) as f32;
    let mut pen_x = match align {
        Align::Left => rx as f32,
        Align::Right => rx as f32 + rw as f32 - text_w,
        Align::Center => rx as f32 + (rw as f32 - text_w) / 2.0,
    };
    // Vertically center the x-height band in the region.
    let baseline = ry as f32 + rh as f32 / 2.0 + size_px as f32 * 0.35;

    with_font(|font| {
        let size = size_px as f32;
        let mut prev: Option<char> = None;
        for ch in content.chars() {
            if let Some(p) = prev {
                pen_x += font.horizontal_kern(p, ch, size).unwrap_or(0.0);
            }
            let glyph = rasterize_glyph(font, ch, size_px);
            draw_glyph(canvas, pen_x, baseline, &glyph, color);
            pen_x += glyph.0.advance_width;
            prev = Some(ch);
        }
    });
}

fn rasterize_glyph(font: &Font, ch: char, size_px: u32) -> Glyph {
    let key = (ch, size_px);
    {
        let guard = GLYPHS.lock().unwrap();
        if let Some(glyph) = guard.get(&key) {
            return (glyph.0, glyph.1.clone());
        }
    }
    let glyph @ (metrics, _) = font.rasterize(ch, size_px as f32);
    {
        let mut guard = GLYPHS.lock().unwrap();
        if guard.len() > 4096 {
            guard.clear();
        }
        guard.insert(key, (metrics, coverage_of(&glyph)));
    }
    glyph
}

fn coverage_of(glyph: &Glyph) -> Vec<u8> {
    glyph.1.clone()
}

/// Draw one coverage-mask glyph at pen position (pen_x, baseline).
fn draw_glyph(
    canvas: &mut image::RgbaImage,
    pen_x: f32,
    baseline: f32,
    glyph: &(Metrics, Vec<u8>),
    color: [u8; 4],
) {
    let (m, coverage) = glyph;
    if m.width == 0 || m.height == 0 {
        return;
    }
    let left = (pen_x + m.xmin as f32).floor() as i32;
    // The glyph box is `height` tall and its bottom edge sits `ymin` above
    // the baseline, so the top is at `baseline - (ymin + height)`. (Letters
    // have ymin ≈ 0, which hid the mistake: mid-height marks like `·` were
    // placed below the baseline instead of at x-height.)
    let top = (baseline - (m.ymin + m.height as i32) as f32).floor() as i32;
    for row in 0..m.height {
        let py = top + row as i32;
        if py < 0 || py as u32 >= canvas.height() {
            continue;
        }
        for col in 0..m.width {
            let cov = coverage[row * m.width + col] as u32;
            if cov == 0 {
                continue;
            }
            let px = left + col as i32;
            if px < 0 || px as u32 >= canvas.width() {
                continue;
            }
            canvas.put_pixel(
                px as u32,
                py as u32,
                over(
                    [color[0], color[1], color[2]],
                    cov as u8,
                    canvas.get_pixel(px as u32, py as u32).0,
                ),
            );
        }
    }
}
