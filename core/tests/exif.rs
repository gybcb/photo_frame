//! Tests for EXIF extraction into display-ready fields.

use framely_core::brand::Brand;
use framely_core::exif::PhotoInfo;
use framely_core::template::Field;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(path).unwrap()
}

fn field_text(info: &PhotoInfo, f: Field) -> Option<String> {
    info.fields
        .parts
        .iter()
        .find(|(k, _)| *k == f)
        .map(|(_, v)| v.clone())
}

#[test]
fn sony_fixture_resolves_brand_and_all_fields() {
    let info = PhotoInfo::from_bytes(&fixture("sony.jpg"));
    assert_eq!(info.brand, Brand::Sony);
    assert_eq!(info.make, "SONY");
    assert_eq!(info.model, "ILCE-7RM4");
    assert_eq!(
        field_text(&info, Field::Lens).as_deref(),
        Some("FE 35mm F1.8")
    );
    assert_eq!(field_text(&info, Field::Aperture).as_deref(), Some("f/1.8"));
    assert_eq!(field_text(&info, Field::Shutter).as_deref(), Some("1/250"));
    assert_eq!(field_text(&info, Field::Iso).as_deref(), Some("ISO100"));
    assert_eq!(field_text(&info, Field::Focal).as_deref(), Some("35mm"));
    assert_eq!(
        field_text(&info, Field::Date).as_deref(),
        Some("2026-10-24")
    );
}

#[test]
fn missing_exif_yields_an_unknown_brand_and_no_fields() {
    let info = PhotoInfo::from_bytes(&fixture("plain.jpg"));
    assert_eq!(info.brand, Brand::Unknown);
    assert!(info.make.is_empty());
    assert!(info.fields.parts.is_empty());
}

#[test]
fn orientation_is_reported() {
    let info = PhotoInfo::from_bytes(&fixture("nikon_rot6.jpg"));
    assert_eq!(info.orientation, 6);
    assert_eq!(info.brand, Brand::Nikon);
    assert_eq!(
        field_text(&info, Field::Lens).as_deref(),
        Some("24-70 mm f/4")
    );
}

#[test]
fn garbage_input_degrades_to_unknown() {
    let info = PhotoInfo::from_bytes(b"definitely not a jpeg");
    assert_eq!(info.brand, Brand::Unknown);
}

#[test]
fn camera_field_carries_the_body_model() {
    let info = PhotoInfo::from_bytes(&fixture("sony.jpg"));
    assert_eq!(
        field_text(&info, Field::Camera).as_deref(),
        Some("ILCE-7RM4")
    );
    assert_eq!(
        info.fields.parts.first().map(|(f, _)| *f),
        Some(Field::Camera)
    );
}

#[test]
fn no_model_means_no_camera_field() {
    let info = PhotoInfo::from_bytes(&fixture("plain.jpg"));
    assert!(info.fields.parts.is_empty());
}
