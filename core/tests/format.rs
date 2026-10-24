//! Tests for field formatting (shutter, aperture, ISO, focal length, lens name).

use framely_core::format::{aperture, clean_lens_name, focal, focal_range, iso, shutter};

#[test]
fn shutter_under_one_second_formats_as_unit_fraction() {
    assert_eq!(shutter(0.004), "1/250");
    assert_eq!(shutter(0.001), "1/1000");
    assert_eq!(shutter(0.008), "1/125");
}

#[test]
fn shutter_rounds_fraction_denominator_to_nearest_integer() {
    // A camera-reported 1/249.6 must display as 1/250, not 1/249.
    assert_eq!(shutter(1.0 / 249.6), "1/250");
}

#[test]
fn shutter_at_or_over_one_second_uses_double_prime() {
    assert_eq!(shutter(1.0), "1\"");
    assert_eq!(shutter(2.0), "2\"");
    assert_eq!(shutter(30.0), "30\"");
}

#[test]
fn shutter_over_one_second_keeps_fractional_part() {
    assert_eq!(shutter(1.5), "1.5\"");
    assert_eq!(shutter(1.3), "1.3\"");
}

#[test]
fn shutter_rounds_two_point_five_to_three() {
    // 0.4s = 1/2.5 -> conventional display is 1/3.
    assert_eq!(shutter(0.4), "1/3");
}

#[test]
fn aperture_omits_decimal_for_whole_numbers() {
    assert_eq!(aperture(8.0), "f/8");
    assert_eq!(aperture(4.0), "f/4");
    assert_eq!(aperture(10.0), "f/10");
}

#[test]
fn aperture_keeps_one_decimal_otherwise() {
    assert_eq!(aperture(2.8), "f/2.8");
    assert_eq!(aperture(1.4), "f/1.4");
    assert_eq!(aperture(3.5), "f/3.5");
    assert_eq!(aperture(1.8), "f/1.8");
}

#[test]
fn aperture_snaps_near_integer_exif_rationals() {
    // f/8 recorded as APEX-derived 8.00000001 must display as f/8.
    assert_eq!(aperture(8.0000001), "f/8");
    // f/2.8 commonly lands as 45/16 = 2.8125 -> display f/2.8.
    assert_eq!(aperture(2.8125), "f/2.8");
}

#[test]
fn iso_prefixes_number() {
    assert_eq!(iso(100), "ISO100");
    assert_eq!(iso(6400), "ISO6400");
    assert_eq!(iso(64), "ISO64");
}

#[test]
fn focal_omits_decimal_for_whole_numbers() {
    assert_eq!(focal(85.0), "85mm");
    assert_eq!(focal(50.0), "50mm");
    assert_eq!(focal(35.5), "35.5mm");
}

#[test]
fn focal_range_joins_two_values() {
    assert_eq!(focal_range(24.0, 70.0), "24-70mm");
    assert_eq!(focal_range(100.0, 400.0), "100-400mm");
}

#[test]
fn lens_name_trims_and_collapses_spaces() {
    assert_eq!(
        clean_lens_name("  Nikon   Z 24-70mm f/4 S  "),
        "Nikon Z 24-70mm f/4 S"
    );
}

#[test]
fn lens_name_strips_trailing_zero_from_mm_values() {
    assert_eq!(clean_lens_name("24.0-70.0 mm f/2.8"), "24-70 mm f/2.8");
    assert_eq!(clean_lens_name("50.0 mm f/1.8"), "50 mm f/1.8");
}

#[test]
fn lens_name_drops_trailing_model_annotation() {
    assert_eq!(
        clean_lens_name("Tamron 28-75mm F/2.8 Di III RXD (Model A036)"),
        "Tamron 28-75mm F/2.8 Di III RXD"
    );
}

#[test]
fn lens_name_strips_wrapping_quotes() {
    assert_eq!(
        clean_lens_name("\"Sigma 35mm F1.4 DG HSN\""),
        "Sigma 35mm F1.4 DG HSN"
    );
}
