//! Serialization tests for theme presets (user JSON files).

use framely_core::template::{Field, Fields, TemplateId, ThemeConfig};

#[test]
fn theme_config_roundtrips_through_json() {
    let mut cfg = ThemeConfig::default_for(TemplateId::Magazine);
    cfg.border_ratio = 0.05;
    cfg.foreground = [10, 20, 30, 255];
    let json = serde_json::to_string(&cfg).unwrap();
    let back: ThemeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.border_ratio, 0.05);
    assert_eq!(back.foreground, [10, 20, 30, 255]);
    assert_eq!(back.bar_ratio, cfg.bar_ratio);
}

#[test]
fn fields_roundtrip_through_json() {
    let f = Fields {
        parts: vec![
            (Field::Shutter, "1/250".into()),
            (Field::Iso, "ISO100".into()),
        ],
        custom_line: Some("handmade".into()),
    };
    let json = serde_json::to_string(&f).unwrap();
    let back: Fields = serde_json::from_str(&json).unwrap();
    assert_eq!(back.parts, f.parts);
    assert_eq!(back.custom_line.as_deref(), Some("handmade"));
}

#[test]
fn field_labels_are_display_friendly() {
    assert_eq!(Field::Iso.label(), "ISO");
    assert_eq!(Field::Shutter.label(), "Shutter");
    assert_eq!(Field::Aperture.label(), "Aperture");
    assert_eq!(Field::Lens.label(), "Lens");
    assert_eq!(Field::Focal.label(), "Focal");
    assert_eq!(Field::Date.label(), "Date");
}

#[test]
fn template_id_defaults_to_white_studio() {
    assert_eq!(TemplateId::default(), TemplateId::WhiteStudio);
}

#[test]
fn old_preset_without_margins_uses_border_ratio() {
    let json = r#"{"border_ratio":0.05,"bar_ratio":0.06,"font_scale":0.45,
        "logo_scale":0.6,"background":[255,255,255,255],
        "foreground":[26,26,26,255],"part_spacing":0.8}"#;
    let cfg: ThemeConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.effective_margins(), [0.05, 0.05, 0.05, 0.05]);
}

#[test]
fn new_preset_roundtrips_with_asymmetric_margins() {
    let mut cfg = ThemeConfig::default_for(TemplateId::Polaroid);
    cfg.margin_top = 0.03;
    cfg.margin_bottom = 0.14;
    cfg.normalize_for_save();
    let json = serde_json::to_string(&cfg).unwrap();
    let back: ThemeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.effective_margins(), [0.03, 0.14, 0.03, 0.03]);
    // Old versions must still read a sane scalar border.
    assert!((back.border_ratio - (0.03 + 0.14 + 0.03 + 0.03) / 4.0).abs() < 1e-9);
}

#[test]
fn new_template_ids_serialize_kebab_case() {
    for (t, id) in [
        (TemplateId::Polaroid, "polaroid"),
        (TemplateId::TwoColumn, "two-column"),
        (TemplateId::TopBar, "top-bar"),
        (TemplateId::CaptionCard, "caption-card"),
    ] {
        assert_eq!(serde_json::to_string(&t).unwrap(), format!("\"{id}\""));
        assert_eq!(
            serde_json::from_str::<TemplateId>(&format!("\"{id}\"")).unwrap(),
            t
        );
    }
}

#[test]
fn camera_field_is_in_drop_order_after_date() {
    let order = &framely_core::template::FIELD_DROP_ORDER;
    let di = order.iter().position(|f| *f == Field::Date).unwrap();
    let ci = order.iter().position(|f| *f == Field::Camera).unwrap();
    assert_eq!(ci, di + 1);
}

#[test]
fn template_id_serializes_to_its_stable_id() {
    // Preset files must stay valid across refactors: serialize the stable
    // kebab-case id, not the Rust variant name.
    assert_eq!(
        serde_json::to_string(&TemplateId::BlackBar).unwrap(),
        "\"black-bar\""
    );
    let back: TemplateId = serde_json::from_str("\"black-bar\"").unwrap();
    assert_eq!(back, TemplateId::BlackBar);
}
