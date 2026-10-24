//! Tests for the CPU compositing pipeline (photo + border + logo + text).

use framely_core::assets::LogoMode;
use framely_core::brand::Brand;
use framely_core::layout::{FrameLayout, layout};
use framely_core::render::compose;
use framely_core::template::TemplateId;
use framely_core::template::{Field, Fields, ThemeConfig};
use image::RgbaImage;

fn gray_photo(w: u32, h: u32, v: u8) -> RgbaImage {
    RgbaImage::from_pixel(w, h, image::Rgba([v, v, v, 255]))
}

fn fields() -> Fields {
    Fields {
        parts: vec![
            (Field::Lens, "FE 35mm F1.8".into()),
            (Field::Aperture, "f/1.8".into()),
            (Field::Shutter, "1/250".into()),
            (Field::Iso, "ISO100".into()),
        ],
        custom_line: None,
    }
}

fn white_studio(brand: Brand) -> FrameLayout {
    layout(
        TemplateId::WhiteStudio,
        400,
        300,
        brand,
        5.0,
        &ThemeConfig::default_for(TemplateId::WhiteStudio),
        &fields(),
    )
}

#[test]
fn canvas_size_and_background_follow_the_layout() {
    // photo 400x300, short side 300: border 12, bar 18.
    let l = white_studio(Brand::Sony);
    let out = compose(&gray_photo(400, 300, 128), &l, LogoMode::Mono).unwrap();
    assert_eq!(out.width(), l.canvas_width);
    assert_eq!(out.height(), l.canvas_height);
    assert_eq!(out.get_pixel(3, 3).0, [255, 255, 255, 255]);
}

#[test]
fn photo_is_pasted_unchanged_into_the_frame() {
    let l = white_studio(Brand::Sony);
    let out = compose(&gray_photo(400, 300, 128), &l, LogoMode::Mono).unwrap();
    // inside photo area must be exactly the photo color
    assert_eq!(out.get_pixel(12 + 200, 12 + 150).0, [128, 128, 128, 255]);
    // and the opposite corner of the photo
    assert_eq!(out.get_pixel(12 + 1, 12 + 1).0, [128, 128, 128, 255]);
}

#[test]
fn mono_logo_paints_theme_foreground_into_the_bar() {
    let l = white_studio(Brand::Sony);
    let out = compose(&gray_photo(400, 300, 128), &l, LogoMode::Mono).unwrap();
    let dark = count_near(&out, [26, 26, 26], 60, logo_box(&l));
    assert!(dark > 30, "expected dark logo pixels in bar, got {dark}");
}

#[test]
fn text_paints_foreground_pixels_in_the_bar() {
    // Unknown brand: no logo, only text right-aligned in the bar.
    let l = white_studio(Brand::Unknown);
    let out = compose(&gray_photo(400, 300, 128), &l, LogoMode::Mono).unwrap();
    let (bx, by, bw, bh) = (
        l.border,
        l.border + l.photo_rect.3,
        l.canvas_width - 2 * l.border,
        l.bar_height,
    );
    let dark = count_near(&out, [26, 26, 26], 90, (bx, by, bw, bh));
    assert!(dark > 100, "expected text pixels, got {dark}");
}

#[test]
fn color_logo_keeps_official_brand_colors() {
    let l = white_studio(Brand::Leica);
    let out = compose(&gray_photo(400, 300, 128), &l, LogoMode::Color).unwrap();
    let red = count_near(&out, [0xED, 0x1C, 0x24], 80, logo_box(&l));
    assert!(red > 100, "expected leica-red pixels, got {red}");
}

#[test]
fn dark_bar_theme_paints_light_text() {
    let tpl = TemplateId::BlackBar;
    let l = layout(
        tpl,
        400,
        300,
        Brand::Unknown,
        5.0,
        &ThemeConfig::default_for(tpl),
        &fields(),
    );
    let out = compose(&gray_photo(400, 300, 128), &l, LogoMode::Mono).unwrap();
    assert_eq!(out.get_pixel(3, 3).0, [17, 17, 17, 255]);
    let (bx, by, bw, bh) = (
        l.border,
        l.border + l.photo_rect.3,
        l.canvas_width - 2 * l.border,
        l.bar_height,
    );
    let light = count_near(&out, [240, 240, 240], 90, (bx, by, bw, bh));
    assert!(
        light > 100,
        "expected light text pixels on dark bar, got {light}"
    );
}

/// (x, y, w, h) of the first logo op, as a region for sampling.
fn logo_box(l: &FrameLayout) -> (u32, u32, u32, u32) {
    l.ops
        .iter()
        .find_map(|o| match o {
            framely_core::layout::DrawOp::Logo {
                x,
                y,
                width,
                height,
                ..
            } => Some((*x, *y, *width, *height)),
            _ => None,
        })
        .expect("layout should contain a logo op")
}

/// Count pixels within `region` whose RGB is within `tol` of `target`.
fn count_near(img: &RgbaImage, target: [u8; 3], tol: i32, region: (u32, u32, u32, u32)) -> u32 {
    let (x0, y0, w, h) = region;
    let mut n = 0;
    for y in y0..(y0 + h).min(img.height()) {
        for x in x0..(x0 + w).min(img.width()) {
            let p = img.get_pixel(x, y).0;
            let close = (0..3).all(|i| (p[i] as i32 - target[i] as i32).abs() <= tol);
            if close {
                n += 1;
            }
        }
    }
    n
}
