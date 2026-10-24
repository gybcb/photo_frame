//! Tests for EXIF make/model → brand matching.

use framely_core::brand::Brand;

#[test]
fn sony_make_matches() {
    assert_eq!(Brand::resolve("SONY", "ILCE-7M4"), Brand::Sony);
    assert_eq!(Brand::resolve("Sony", "α7 III"), Brand::Sony);
}

#[test]
fn nikon_make_matches_including_corporation_spelling() {
    assert_eq!(
        Brand::resolve("NIKON CORPORATION", "NIKON Z 6"),
        Brand::Nikon
    );
    assert_eq!(Brand::resolve("Nikon", "D850"), Brand::Nikon);
}

#[test]
fn canon_make_matches() {
    assert_eq!(Brand::resolve("Canon", "Canon EOS R5"), Brand::Canon);
    assert_eq!(Brand::resolve("CANON", "EOS 5D Mark IV"), Brand::Canon);
}

#[test]
fn leica_make_matches_but_not_leica_geosystems() {
    assert_eq!(Brand::resolve("Leica Camera AG", "M11"), Brand::Leica);
    assert_eq!(Brand::resolve("LEICA", "Q3"), Brand::Leica);
    // "LEICA CAMERA GEOSYSTEMS" bodies carry Phase One backs.
    assert_eq!(
        Brand::resolve("LEICA CAMERA GEOSYSTEMS", "RCP 8.0"),
        Brand::PhaseOne
    );
}

#[test]
fn hasselblad_matches_make_or_model_fallback() {
    assert_eq!(Brand::resolve("Hasselblad", "X2D 100C"), Brand::Hasselblad);
    // Some Hasselblad files write an empty/odd make but a V-system model.
    assert_eq!(Brand::resolve("", "CFV 120"), Brand::Hasselblad);
    assert_eq!(Brand::resolve("", "V"), Brand::Hasselblad);
}

#[test]
fn phase_one_matches_with_spacings_and_mamiya() {
    assert_eq!(Brand::resolve("Phase One", "P3 645DF+"), Brand::PhaseOne);
    assert_eq!(Brand::resolve("PHASE ONE", "XT"), Brand::PhaseOne);
    assert_eq!(Brand::resolve("MAMIYA", "Phase One P45+"), Brand::PhaseOne);
    assert_eq!(Brand::resolve("MAM", "Mamiya DF"), Brand::PhaseOne);
}

#[test]
fn alpa_matches() {
    assert_eq!(Brand::resolve("ALPA", "ALPA 12"), Brand::Alpa);
    assert_eq!(Brand::resolve("Alpa", "12 TC"), Brand::Alpa);
}

#[test]
fn unknown_make_falls_back_to_unknown() {
    assert_eq!(Brand::resolve("FUJIFILM", "X-T5"), Brand::Unknown);
    assert_eq!(Brand::resolve("", ""), Brand::Unknown);
    assert_eq!(Brand::resolve("OM DIGITAL", "OM-1"), Brand::Unknown);
}

#[test]
fn empty_make_never_matches_by_make_alone() {
    // Empty make + non-hasselblad model must stay Unknown (no accidental match).
    assert_eq!(Brand::resolve("", "NIKON Z 6"), Brand::Unknown);
}

#[test]
fn brand_has_asset_stem_for_svg_lookup() {
    assert_eq!(Brand::Sony.asset_stem(), "sony");
    assert_eq!(Brand::PhaseOne.asset_stem(), "phaseone");
    assert_eq!(Brand::Unknown.asset_stem(), "unknown");
}
