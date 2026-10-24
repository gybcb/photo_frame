use framely_core::assets::LogoMode;
use framely_core::brand::Brand;
use framely_core::layout::layout;
use framely_core::render::compose;
use framely_core::template::{Field, Fields, TemplateId, ThemeConfig};
use sha2::{Digest, Sha256};

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

fn cases() -> Vec<(String, u32, u32, TemplateId, Brand, LogoMode)> {
    let sizes = [(2000u32, 1333u32), (1333, 2000), (1600, 1600)];
    let tpls = [
        TemplateId::WhiteStudio,
        TemplateId::BlackBar,
        TemplateId::Minimal,
        TemplateId::Magazine,
    ];
    let brands = [Brand::Sony, Brand::Leica, Brand::Unknown];
    let modes = [LogoMode::Mono, LogoMode::Color];
    let mut v = Vec::new();
    for &(w, h) in &sizes {
        for &t in &tpls {
            for &b in &brands {
                for &m in &modes {
                    v.push((format!("{t:?}-{w}x{h}-{b:?}-{m:?}"), w, h, t, b, m));
                }
            }
        }
    }
    v
}

fn png_hash(w: u32, h: u32, tpl: TemplateId, brand: Brand, mode: LogoMode) -> String {
    let photo = image::RgbaImage::from_pixel(w, h, image::Rgba([120, 130, 140, 255]));
    let theme = ThemeConfig::default_for(tpl);
    let l = layout(tpl, w, h, brand, 2.0, &theme, &fields());
    let canvas = compose(&photo, &l, mode).unwrap();
    let mut png = Vec::new();
    canvas.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    let mut d = Sha256::new();
    d.update(&png);
    hex(&d.finalize())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
#[ignore = "run with FRAMELY_GOLDEN=record to (re)write baselines"]
fn record_golden() {
    // Only re-run at the pre-rewrite baseline commit to capture references.
    let mut out = String::new();
    for (key, w, h, t, b, m) in cases() {
        out.push_str(&format!("{key}\t{}\n", png_hash(w, h, t, b, m)));
    }
    let path = format!("{}/tests/fixtures/golden.sha256", env!("CARGO_MANIFEST_DIR"));
    std::fs::write(&path, out).unwrap();
}

#[test]
fn golden_matches_baseline() {
    let path = format!("{}/tests/fixtures/golden.sha256", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).expect("run record_golden at baseline to create golden.sha256");
    let want: std::collections::BTreeMap<String, String> = text
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    for (key, w, h, t, b, m) in cases() {
        let got = png_hash(w, h, t, b, m);
        assert_eq!(got, want[&key], "golden mismatch for {key}");
    }
}
