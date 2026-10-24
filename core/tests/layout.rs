//! Tests for proportional frame layout: geometry scales linearly with the
//! canvas, fields drop in priority order, and every theme keeps its shape.

use framely_core::brand::Brand;
use framely_core::layout::{Align, DrawOp, FrameLayout, layout};
use framely_core::template::{Field, Fields, TemplateId, ThemeConfig};

fn all_fields() -> Fields {
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

fn lay(w: u32, h: u32, tpl: TemplateId, fields: &Fields, brand: Brand) -> FrameLayout {
    layout(
        tpl,
        w,
        h,
        brand,
        2.0, // logo width/height ratio
        &ThemeConfig::default_for(tpl),
        fields,
    )
}

fn texts(l: &FrameLayout) -> Vec<String> {
    l.ops
        .iter()
        .filter_map(|o| match o {
            DrawOp::Text { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn white_studio_places_photo_inside_border_and_adds_bar_to_bottom() {
    // short side 1333, default border ratio 4% -> 53px
    let l = lay(
        2000,
        1333,
        TemplateId::WhiteStudio,
        &all_fields(),
        Brand::Sony,
    );
    assert_eq!(l.border, 53);
    assert_eq!(l.canvas_width, 2000 + 2 * 53);
    assert_eq!(l.photo_rect, (53, 53, 2000, 1333));
    assert_eq!(l.canvas_height, 1333 + 2 * 53 + l.bar_height);
    assert_eq!(l.background, [255, 255, 255, 255]);
}

#[test]
fn geometry_scales_linearly_with_canvas() {
    // Dimensions chosen so ratio rounding lands on exact integers.
    let small = lay(
        1875,
        1250,
        TemplateId::WhiteStudio,
        &all_fields(),
        Brand::Sony,
    );
    let big = lay(
        3750,
        2500,
        TemplateId::WhiteStudio,
        &all_fields(),
        Brand::Sony,
    );
    assert_eq!(big.border, small.border * 2);
    assert_eq!(big.bar_height, small.bar_height * 2);
    let small_font = small.ops.iter().find_map(|o| match o {
        DrawOp::Text { size_px, .. } => Some(*size_px),
        _ => None,
    });
    let big_font = big.ops.iter().find_map(|o| match o {
        DrawOp::Text { size_px, .. } => Some(*size_px),
        _ => None,
    });
    assert_eq!(big_font, small_font.map(|f| f * 2));
}

#[test]
fn white_studio_has_logo_left_and_text_right() {
    let l = lay(
        2000,
        1333,
        TemplateId::WhiteStudio,
        &all_fields(),
        Brand::Sony,
    );
    assert!(
        l.ops.iter().any(|o| matches!(o, DrawOp::Logo { .. })),
        "logo missing"
    );
    let aligns: Vec<Align> = l
        .ops
        .iter()
        .filter_map(|o| match o {
            DrawOp::Text { align, .. } => Some(*align),
            _ => None,
        })
        .collect();
    assert!(aligns.contains(&Align::Right), "params right-anchored");
}

#[test]
fn logo_keeps_aspect_ratio_and_fits_the_bar() {
    let l = lay(
        2000,
        1333,
        TemplateId::WhiteStudio,
        &all_fields(),
        Brand::Sony,
    );
    let (w, h) = l
        .ops
        .iter()
        .find_map(|o| match o {
            DrawOp::Logo { width, height, .. } => Some((*width, *height)),
            _ => None,
        })
        .unwrap();
    assert_eq!(w as f64 / h as f64, 2.0);
    assert!(h <= l.bar_height);
}

#[test]
fn unknown_brand_omits_logo_but_keeps_text() {
    let l = lay(
        2000,
        1333,
        TemplateId::WhiteStudio,
        &all_fields(),
        Brand::Unknown,
    );
    assert!(!l.ops.iter().any(|o| matches!(o, DrawOp::Logo { .. })));
    assert!(!texts(&l).is_empty());
}

#[test]
fn magazine_breaks_fields_across_two_lines() {
    let l = lay(
        2000,
        1333,
        TemplateId::Magazine,
        &all_fields(),
        Brand::Hasselblad,
    );
    assert!(
        texts(&l).len() >= 2,
        "magazine should split fields into lines, got {:?}",
        texts(&l)
    );
}

#[test]
fn magazine_shows_the_logo_once() {
    // Two text lines must not mean two logos: the brand mark stands once.
    let l = lay(
        2000,
        1333,
        TemplateId::Magazine,
        &all_fields(),
        Brand::Hasselblad,
    );
    let logos = l
        .ops
        .iter()
        .filter(|o| matches!(o, DrawOp::Logo { .. }))
        .count();
    assert_eq!(logos, 1, "magazine drew {logos} logos, expected exactly 1");
}

#[test]
fn black_bar_uses_dark_background_and_light_text() {
    let l = lay(
        2000,
        1333,
        TemplateId::BlackBar,
        &all_fields(),
        Brand::Nikon,
    );
    assert_eq!(l.background, [17, 17, 17, 255]);
    let color = l.ops.iter().find_map(|o| match o {
        DrawOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(color, Some([240, 240, 240, 255]));
}

#[test]
fn overflowing_fields_drop_by_priority_iso_survives_date_goes_first() {
    let l = lay(
        190,
        2000,
        TemplateId::Minimal,
        &Fields {
            parts: vec![
                (Field::Date, "2026-10-24".into()),
                (
                    Field::Lens,
                    "Some Remarkably Verbose Zoom Lens Model Edition".into(),
                ),
                (Field::Aperture, "f/1.4".into()),
                (Field::Shutter, "1/8000".into()),
                (Field::Iso, "ISO25600".into()),
            ],
            custom_line: None,
        },
        Brand::Unknown,
    );
    let shown = texts(&l).join(" ");
    assert!(!shown.contains("2026"), "date dropped first, got {shown}");
    assert!(
        !shown.contains("long"),
        "lens dropped before the exposure trio"
    );
    assert!(
        shown.contains("ISO25600") && shown.contains("f/1.4"),
        "core trio survives: {shown}"
    );
}

#[test]
fn custom_line_replaces_all_fields() {
    let l = lay(
        2000,
        1333,
        TemplateId::WhiteStudio,
        &Fields {
            parts: vec![(Field::Iso, "ISO100".into())],
            custom_line: Some("ALPA 12 plus · Rodenstock 50mm".into()),
        },
        Brand::Unknown,
    );
    let shown = texts(&l).join(" ");
    assert!(shown.contains("ALPA 12 plus · Rodenstock 50mm"));
    assert!(!shown.contains("ISO100"), "fields replaced by custom line");
}

#[test]
fn no_template_overflows_its_canvas() {
    for tpl in [
        TemplateId::WhiteStudio,
        TemplateId::BlackBar,
        TemplateId::Minimal,
        TemplateId::Magazine,
    ] {
        let l = lay(1600, 1067, tpl, &all_fields(), Brand::Leica);
        assert!(l.bar_height > 0 && l.border > 0);
        for op in &l.ops {
            if let DrawOp::Text { rect, .. } = op {
                assert!(
                    rect.0 + rect.2 <= l.canvas_width && rect.1 + rect.3 <= l.canvas_height,
                    "{tpl:?} text outside canvas: {rect:?} vs {}x{}",
                    l.canvas_width,
                    l.canvas_height
                );
            }
        }
    }
}
