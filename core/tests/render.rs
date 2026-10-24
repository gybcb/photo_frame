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

#[test]
fn separator_dots_ride_mid_xheight_not_below_the_baseline() {
    // Render a line that is *only* separators (U+00B7) so no letter ink can
    // mask their placement. Correct placement puts the dot's ink band near
    // mid x-height; the broken baseline math drops it well below the
    // baseline, into the bar's lower third.
    let fields = Fields {
        parts: vec![],
        custom_line: Some("· · · · ·".into()),
    };
    let l = layout(
        TemplateId::WhiteStudio,
        2000,
        1500,
        Brand::Unknown,
        5.0,
        &ThemeConfig::default_for(TemplateId::WhiteStudio),
        &fields,
    );
    let out = compose(&gray_photo(2000, 1500, 128), &l, LogoMode::Mono).unwrap();
    let (bx, by, bw, bh) = (
        l.border,
        l.border + l.photo_rect.3,
        l.canvas_width - 2 * l.border,
        l.bar_height,
    );
    let ink = count_near(&out, [26, 26, 26], 90, (bx, by, bw, bh));
    assert!(ink > 20, "dots must be drawn at all, got {ink}");
    // Lower third of the bar: only a wrongly-placed dot reaches this far down.
    let low = count_near(&out, [26, 26, 26], 90, (bx, by + bh * 2 / 3, bw, bh / 3));
    assert_eq!(low, 0, "dots sank below the baseline into the bar's bottom");
    // Their band: the bar's middle (upper third would only catch ascenders).
    let mid = count_near(&out, [26, 26, 26], 90, (bx, by + bh / 4, bw, bh / 2));
    assert!(mid > 20, "dot ink belongs mid-bar, upper-mid ink was {mid}");
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

#[test]
fn the_photo_is_blitted_pixel_exact_across_its_whole_rect() {
    // Characterization guard for the paste step: a gradient photo means a
    // wrong row offset, stride or off-by-one shows up as a mismatch rather
    // than hiding inside a flat fill. Everything inside photo_rect must
    // equal the source exactly; everything outside must equal the theme bg.
    let (pw, ph) = (37u32, 23); // odd sizes: no accidental row alignment
    let photo = RgbaImage::from_fn(pw, ph, |x, y| {
        image::Rgba([(x * 7) as u8, (y * 11) as u8, ((x + y) * 5) as u8, 255])
    });
    let l = white_studio_dims(pw, ph);
    let out = compose(&photo, &l, LogoMode::Mono).unwrap();
    let (bx, by, _, _) = l.photo_rect;
    for y in 0..l.canvas_height {
        for x in 0..l.canvas_width {
            let inside = (bx..bx + pw).contains(&x) && (by..by + ph).contains(&y);
            let want = if inside {
                *photo.get_pixel(x - bx, y - by)
            } else {
                image::Rgba(l.background)
            };
            // The bar's logo/text can overwrite pixels below the photo;
            // only compare regions the paste step owns.
            if y >= by + ph {
                continue;
            }
            assert_eq!(out.get_pixel(x, y), &want, "pixel ({x},{y})");
        }
    }
}

/// White Studio layout sized to an arbitrary photo (logo ratio 5).
fn white_studio_dims(w: u32, h: u32) -> FrameLayout {
    layout(
        TemplateId::WhiteStudio,
        w,
        h,
        Brand::Sony,
        5.0,
        &ThemeConfig::default_for(TemplateId::WhiteStudio),
        &fields(),
    )
}
