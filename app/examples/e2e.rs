//! Throwaway end-to-end driver: run the batch exporter over the real
//! fixtures and dump framed images to /tmp/framely-e2e for inspection.

use std::path::PathBuf;

use framely_app::batch::{BatchSpec, export_one};
use framely_core::assets::LogoMode;
use framely_core::template::{TemplateId, ThemeConfig};

fn main() {
    let out = PathBuf::from("/tmp/framely-e2e");
    std::fs::create_dir_all(&out).unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures");

    let big = PathBuf::from("/tmp/sony_big.jpg");
    let list: Vec<(PathBuf, TemplateId)> = vec![
        (fixtures.join("sony.jpg"), TemplateId::Magazine),
        (big.clone(), TemplateId::Magazine),
        (big.clone(), TemplateId::WhiteStudio),
        (big.clone(), TemplateId::BlackBar),
        (fixtures.join("nikon_rot6.jpg"), TemplateId::WhiteStudio),
        (fixtures.join("plain.jpg"), TemplateId::Minimal),
        (PathBuf::from("/tmp/probe.png"), TemplateId::WhiteStudio),
    ];
    for (src, tpl) in list {
        let s = BatchSpec {
            template: tpl,
            theme: ThemeConfig::default_for(tpl),
            logo_mode: if tpl == TemplateId::BlackBar {
                LogoMode::Color
            } else {
                LogoMode::Mono
            },
            quality: 92,
            custom_line: None,
        };
        match export_one(&src, &out, &s) {
            Ok(p) => println!("ok  {} {tpl:?}: {}", src.display(), p.display()),
            Err(e) => println!("err {} {tpl:?}: {e}", src.display()),
        }
    }
}
