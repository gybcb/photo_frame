//! Tests for language selection: system detection, config persistence,
//! and the CJK font probe. All pure logic — the GUI wiring is manual.

use std::fs;
use std::path::PathBuf;

use framely_app::i18n::{
    CJK_CANDIDATES, CJK_SCALE, Config, Lang, Msg, egui_fonts, find_cjk_font, load_config,
    save_config,
};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("framely-i18n-{tag}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_zh_locale_prefix_selects_chinese() {
    assert_eq!(Lang::detect(Some("zh_CN.UTF-8")), Lang::Zh);
    assert_eq!(Lang::detect(Some("zh_Hans_CN.UTF-8")), Lang::Zh);
    assert_eq!(Lang::detect(Some("en_US.UTF-8")), Lang::En);
    assert_eq!(Lang::detect(Some("de_DE")), Lang::En);
    assert_eq!(Lang::detect(None), Lang::En);
}

#[test]
fn config_roundtrips_and_defaults_to_auto() {
    let dir = scratch("cfg");
    let path = dir.join("config.json");
    // Missing file behaves like "no preference".
    assert_eq!(load_config(&path), Config::default());
    let cfg = Config {
        lang: Some(Lang::Zh),
    };
    save_config(&path, &cfg).unwrap();
    assert_eq!(load_config(&path), cfg);
}

#[test]
fn a_corrupt_config_file_degrades_to_default() {
    let dir = scratch("corrupt");
    let path = dir.join("config.json");
    fs::write(&path, b"{ not json").unwrap();
    assert_eq!(load_config(&path), Config::default());
}

#[test]
fn the_font_probe_skips_missing_and_non_font_candidates() {
    // A nonexistent path must simply lose to the next candidate.
    assert!(find_cjk_font(&["/nope/nope.ttc"]).is_none());
    // A present file that isn't a font must also be rejected, not
    // handed to the font engine as if it were one.
    let dir = scratch("font");
    let junk = dir.join("junk.ttf");
    fs::write(&junk, b"hello world, definitely not sfnt").unwrap();
    assert!(find_cjk_font(&[junk.to_str().unwrap()]).is_none());
}

#[test]
fn the_font_probe_finds_a_real_system_font() {
    // One real, CJK-capable system font must be detected on macOS.
    let found = find_cjk_font(&[
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
    ]);
    let font = found.expect("some CJK system font should exist on macOS");
    assert!(!font.bytes.is_empty());
    assert!(font.face_count >= 1);
    assert!(font.index < font.face_count);
}

#[test]
fn status_messages_format_in_both_languages() {
    assert_eq!(
        Lang::En.text(&Msg::BatchQueued(3)),
        "3 more queued for batch"
    );
    assert_eq!(Lang::Zh.text(&Msg::BatchQueued(3)), "已排入批量 3 张");
    let s = Lang::Zh.text(&Msg::BatchDone {
        ok: 9,
        total: 10,
        failed: 1,
    });
    assert!(
        s.contains("9") && s.contains("10") && s.contains("1"),
        "{s}"
    );
    // English messages must stay pure ASCII.
    assert!(
        Lang::En
            .text(&Msg::PresetSaved("/tmp/x.json".into()))
            .is_ascii()
    );
    assert_eq!(
        Lang::En.text(&Msg::PresetSaved("/tmp/x.json".into())),
        "preset saved to /tmp/x.json"
    );
    assert_eq!(
        Lang::Zh.text(&Msg::EmptyBatch),
        "批量列表为空——打开或拖入更多文件"
    );
}

#[test]
fn the_probe_returns_the_first_winning_candidate_in_order() {
    let dir = scratch("order");
    // Copy a real font twice; the first listed candidate must win.
    let real = std::path::Path::new("/System/Library/Fonts/Hiragino Sans GB.ttc");
    if !real.exists() {
        return; // non-macOS dev box: nothing to assert
    }
    let a = dir.join("a.ttc");
    fs::copy(real, &a).unwrap();
    let found = find_cjk_font(&[a.to_str().unwrap(), "/nope.ttc"]).unwrap();
    assert_eq!(found.path, a);
}

/// The CJK face must be APPENDED (Latin keeps its metrics), rendered
/// slightly smaller than Latin (CJK glyphs fill the em box; Latin only
/// reaches cap height), and never allowed to inflate the family metrics.
#[test]
fn cjk_is_registered_last_and_slightly_shrunk() {
    let Some(font) = find_cjk_font(CJK_CANDIDATES) else {
        return; // no CJK on this machine: nothing to assert
    };
    let defs = egui_fonts(&font);
    let data = &defs.font_data["cjk"];
    assert!(
        (data.tweak.scale - CJK_SCALE).abs() < 1e-6,
        "cjk registered at scale {}",
        data.tweak.scale
    );
    let prop = &defs.families[&egui::FontFamily::Proportional];
    assert_eq!(prop.last().map(String::as_str), Some("cjk"));
    assert!(prop.len() >= 2, "cjk must be appended after the defaults");
}
