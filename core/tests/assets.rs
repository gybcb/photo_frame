//! Tests for the embedded brand-logo lookup and color substitution.

use framely_core::assets::{LogoMode, logo_svg};
use framely_core::brand::Brand;

const KNOWN: [Brand; 7] = [
    Brand::Sony,
    Brand::Nikon,
    Brand::Canon,
    Brand::Leica,
    Brand::Hasselblad,
    Brand::PhaseOne,
    Brand::Alpa,
];

#[test]
fn every_known_brand_has_both_logo_variants() {
    for brand in KNOWN {
        assert!(
            logo_svg(brand, LogoMode::Color, [0, 0, 0, 255], [255, 255, 255, 255]).is_some(),
            "color {brand:?}"
        );
        assert!(
            logo_svg(brand, LogoMode::Mono, [0, 0, 0, 255], [255, 255, 255, 255]).is_some(),
            "mono {brand:?}"
        );
    }
}

#[test]
fn unknown_brand_has_no_logo() {
    assert!(logo_svg(Brand::Unknown, LogoMode::Mono, [0, 0, 0, 255], [255, 255, 255, 255]).is_none());
    assert!(logo_svg(Brand::Unknown, LogoMode::Color, [0, 0, 0, 255], [255, 255, 255, 255]).is_none());}

#[test]
fn mono_logo_substitutes_foreground_color() {
    let svg =
        String::from_utf8(logo_svg(Brand::Sony, LogoMode::Mono, [255, 255, 255, 255], [0, 0, 0, 255]).unwrap())
            .unwrap();
    assert!(!svg.contains("%FG%"), "placeholder should be replaced");
    assert!(svg.contains("#ffffff") || svg.contains("#FFFFFF"), "fg color present");
}

#[test]
fn mono_logos_all_lose_their_placeholders() {
    for brand in KNOWN {
        let svg = logo_svg(brand, LogoMode::Mono, [17, 34, 51, 255], [250, 250, 250, 255]).unwrap();
        let text = String::from_utf8(svg).unwrap();
        assert!(!text.contains("%FG%"), "{brand:?} still has fg placeholder");
        assert!(!text.contains("%BG%"), "{brand:?} still has bg placeholder");
        assert!(text.contains("#112233"), "{brand:?} fg color not substituted");
    }
}

#[test]
fn leica_mono_uses_the_background_color_for_its_wordmark() {
    let svg =
        String::from_utf8(logo_svg(Brand::Leica, LogoMode::Mono, [0, 0, 0, 255], [250, 250, 250, 255]).unwrap())
            .unwrap();
    assert!(svg.contains("#fafafa"), "leica wordmark should carry the bg color");
}

#[test]
fn color_logo_is_left_untouched() {
    let svg =
        logo_svg(Brand::Leica, LogoMode::Color, [0, 0, 0, 255], [255, 255, 255, 255]).unwrap();
    let text = String::from_utf8(svg).unwrap();
    assert!(text.contains("#ED1C24"), "leica red should remain in color variant");
}
