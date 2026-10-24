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
