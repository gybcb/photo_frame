//! Batch export tests. The GUI wiring isn't worth automating, but the
//! file-processing core is: it must reuse the exact single-image pipeline,
//! carry EXIF through, keep output names unique, and turn a broken file
//! into an error rather than a panic.

use std::fs;
use std::path::PathBuf;

use framely_app::batch::{BatchSpec, export_one, unique_path};
use framely_core::assets::LogoMode;
use framely_core::template::{TemplateId, ThemeConfig};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures")
        .join(name)
}

/// A fresh, empty output directory (removed first so unique-path numbering
/// and assertions don't inherit anything from a previous run).
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("framely-batch-{tag}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn spec() -> BatchSpec {
    BatchSpec {
        template: TemplateId::WhiteStudio,
        theme: ThemeConfig::default_for(TemplateId::WhiteStudio),
        logo_mode: LogoMode::Mono,
        quality: 90,
        custom_line: None,
    }
}

/// Locate the Exif APP1 segment in a JPEG, independently of the crate under
/// test, so we can prove the spliced bytes are the source's own.
fn app1_segment(bytes: &[u8]) -> Vec<u8> {
    let mut i = 2; // past SOI
    while i + 4 <= bytes.len() && bytes[i] == 0xFF {
        let marker = bytes[i + 1];
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if marker == 0xE1 {
            return bytes[i + 2..i + 2 + len].to_vec();
        }
        if marker == 0xDA {
            break; // start of scan: no APP1 can follow
        }
        i += 2 + len;
    }
    Vec::new()
}

#[test]
fn export_one_writes_a_framed_jpeg_with_the_expected_name() {
    let dir = scratch("write");
    let out = export_one(&fixture("sony.jpg"), &dir, &spec()).unwrap();
    assert!(out.exists());
    assert_eq!(out.extension().unwrap(), "jpg");
    assert_eq!(
        out.file_stem().unwrap(),
        "sony_framed",
        "name follows the <stem>_framed rule"
    );
    // The frame grows the canvas, so output pixels must exceed the source's.
    let framed = image::ImageReader::open(&out).unwrap().decode().unwrap();
    let src = image::ImageReader::open(fixture("sony.jpg"))
        .unwrap()
        .decode()
        .unwrap();
    assert!(
        framed.width() > src.width() && framed.height() > src.height(),
        "output should be the framed (larger) canvas"
    );
}

#[test]
fn export_one_carries_the_source_app1_byte_for_byte() {
    let dir = scratch("exif");
    let out = export_one(&fixture("sony.jpg"), &dir, &spec()).unwrap();
    assert_eq!(
        app1_segment(&fs::read(fixture("sony.jpg")).unwrap()),
        app1_segment(&fs::read(&out).unwrap()),
        "the batch path must splice the original APP1 unchanged"
    );
}

#[test]
fn export_one_handles_a_file_without_exif() {
    let dir = scratch("plain");
    let out = export_one(&fixture("plain.jpg"), &dir, &spec()).unwrap();
    assert!(out.exists(), "a photo without EXIF still frames fine");
    assert!(app1_segment(&fs::read(&out).unwrap()).is_empty());
}

#[test]
fn a_broken_file_reports_an_error_not_a_panic() {
    let dir = scratch("broken");
    let bad = dir.join("broken.jpg");
    fs::write(&bad, b"not a jpeg at all").unwrap();
    assert!(export_one(&bad, &dir, &spec()).is_err());
}

#[test]
fn unique_path_bumps_rather_than_overwrites() {
    let dir = scratch("unique");
    let first = unique_path(&dir, "shot", "jpg");
    assert_eq!(first.file_name().unwrap(), "shot_framed.jpg");
    fs::write(&first, b"x").unwrap();
    let second = unique_path(&dir, "shot", "jpg");
    assert_eq!(second.file_name().unwrap(), "shot_framed_2.jpg");
    fs::write(&second, b"x").unwrap();
    assert_eq!(
        unique_path(&dir, "shot", "jpg").file_name().unwrap(),
        "shot_framed_3.jpg"
    );
}

#[test]
fn a_batch_custom_line_changes_the_rendered_bar() {
    // Same source, same theme — the only difference is the shared custom
    // line. If it reaches the layout at all, the drawn parameters differ,
    // so the encoded pixels must differ too.
    let plain = scratch("line-plain");
    let custom = scratch("line-custom");
    let mut s = spec();
    let auto = export_one(&fixture("sony.jpg"), &plain, &s).unwrap();
    s.custom_line = Some("ALPA 12 · manual".into());
    let manual = export_one(&fixture("sony.jpg"), &custom, &s).unwrap();
    assert_ne!(
        fs::read(&auto).unwrap(),
        fs::read(&manual).unwrap(),
        "custom_line must replace the per-file fields in the output"
    );
}
