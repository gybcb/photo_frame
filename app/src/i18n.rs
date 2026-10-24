//! Language selection and the paired-string helper, plus the pieces
//! needed to render Chinese: system-locale detection, a persisted
//! config, and a probe for a CJK-capable system font.
//!
//! Strings live at the call site as pairs (`select(en, zh)`), the
//! egui-recommended pattern: no key table to drift, and an English
//! original is always visible next to its translation.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The two UI languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    /// First UI language from a `LANG` environment value: anything
    /// starting `zh` picks Chinese; everything else (and unset) is
    /// English.
    pub fn detect(env_lang: Option<&str>) -> Lang {
        match env_lang {
            Some(v) if v.starts_with("zh") => Lang::Zh,
            _ => Lang::En,
        }
    }

    /// Pick the translation for this language.
    pub fn select(self, en: &'static str, zh: &'static str) -> &'static str {
        match self {
            Lang::En => en,
            Lang::Zh => zh,
        }
    }

    /// The toolbar toggle shows the *other* language.
    pub fn switch_label(self) -> &'static str {
        match self {
            Lang::En => "中文",
            Lang::Zh => "EN",
        }
    }

    /// Render a status-line message in this language.
    /// Pick the translation, allowing owned/dynamic strings.
    pub fn say(self, en: impl Into<String>, zh: impl Into<String>) -> String {
        self.pick(en, zh)
    }

    fn pick(self, en: impl Into<String>, zh: impl Into<String>) -> String {
        match self {
            Lang::En => en.into(),
            Lang::Zh => zh.into(),
        }
    }

    pub fn text(self, msg: &Msg) -> String {
        use Msg::*;
        match msg {
            BatchQueued(n) => self.pick(
                format!("{n} more queued for batch"),
                format!("已排入批量 {n} 张"),
            ),
            EmptyBatch => self.pick(
                "batch list is empty — open or drop more files",
                "批量列表为空——打开或拖入更多文件",
            ),
            BatchDone { ok, total, failed } => self.pick(
                format!("batch done: {ok}/{total} ok, {failed} failed"),
                format!("批量完成：{ok}/{total} 成功，{failed} 失败"),
            ),
            PresetSaved(p) => {
                self.pick(format!("preset saved to {p}"), format!("预设已保存到 {p}"))
            }
            PresetLoaded(p) => self.pick(
                format!("preset loaded from {p}"),
                format!("已从 {p} 载入预设"),
            ),
            Exported(p, carried) => self.pick(
                format!("saved {p} (EXIF carried: {carried})"),
                format!(
                    "已保存 {p}（EXIF 保留：{}）",
                    if *carried { "是" } else { "否" }
                ),
            ),
            Failed(action, why) => {
                let (en, zh) = match action {
                    Action::Write => ("write", "写入"),
                    Action::Export => ("export", "导出"),
                    Action::Serialize => ("serialize", "序列化"),
                    Action::PresetLoad => ("preset load", "预设载入"),
                    Action::Read => ("read", "读取"),
                    Action::Decode => ("decode", "解码"),
                    Action::Render => ("render", "渲染"),
                };
                self.pick(format!("{en} failed: {why}"), format!("{zh}失败：{why}"))
            }
        }
    }
}

/// The action that failed, for status messages.
#[derive(Clone)]
pub enum Action {
    Write,
    Export,
    Serialize,
    PresetLoad,
    Read,
    Decode,
    Render,
}

/// A status-line message with everything it needs to render. Centralized
/// so the whole line is translated as one unit, interpolations included.
#[derive(Clone)]
pub enum Msg {
    BatchQueued(usize),
    EmptyBatch,
    BatchDone {
        ok: usize,
        total: usize,
        failed: usize,
    },
    PresetSaved(String),
    PresetLoaded(String),
    /// path, EXIF carried
    Exported(String, bool),
    /// failed action + reason
    Failed(Action, String),
}

/// User preferences persisted to `~/.framely/config.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// `None` = follow the system locale.
    #[serde(default)]
    pub lang: Option<Lang>,
}

/// Read the config; any problem (missing, corrupt) yields the default.
pub fn load_config(path: &Path) -> Config {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Write the config, creating the parent directory if needed.
pub fn save_config(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

/// A system font file found usable for Chinese text.
pub struct CjkFont {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    /// Face to use within the file (TTC collections hold several).
    pub index: u32,
    /// How many faces the file contains.
    pub face_count: u32,
}

/// Where Chinese might come from on this machine, in preference order.
pub const CJK_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
];

/// CJK glyphs fill their em box while Latin only reaches cap height, so
/// at equal point sizes Chinese looks noticeably bigger. It also keeps
/// mixed-script rows from breaking: egui sizes a row by the *primary*
/// font's metrics, but Hiragino-class fonts carry `hhea.lineGap = 0.5em`
/// (line ratio 1.50 vs the defaults' 1.15), and egui's per-glyph
/// vertical centering shifts the fallback glyph up by half that gap —
/// enough to clip the top of Chinese glyphs out of their rows. Scaling
/// to 0.84 equalizes visual size and brings the effective line ratio
/// (1.50 × 0.84 ≈ 1.26) close enough to the row height that centering
/// no longer clips.
pub const CJK_SCALE: f32 = 0.84;

/// Font definitions with `font` registered as the Chinese fallback:
/// appended last in both families (so Latin keeps the default fonts and
/// their metrics) and scaled down by [`CJK_SCALE`].
pub fn egui_fonts(font: &CjkFont) -> egui::FontDefinitions {
    use egui::{FontData, FontDefinitions, FontFamily, FontTweak};
    let mut fonts = FontDefinitions::default();
    let tweak = FontTweak {
        scale: CJK_SCALE,
        ..Default::default()
    };
    fonts.font_data.insert(
        "cjk".into(),
        std::sync::Arc::new(
            FontData::from_blob(std::sync::Arc::new(font.bytes.clone()), font.index).tweak(tweak),
        ),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".into());
    }
    fonts
}

/// The persisted-config location: `~/.framely/config.json` (cwd fallback
/// when HOME is unset).
pub fn config_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".framely").join("config.json"))
        .unwrap_or_else(|| PathBuf::from("framely-config.json"))
}

/// Localized template display name. Translations live on the app side;
/// core only knows the English identifiers baked into the render.
pub fn template_name(lang: Lang, t: framely_core::template::TemplateId) -> &'static str {
    use framely_core::template::TemplateId as T;
    lang.select(
        t.display_name(),
        match t {
            T::WhiteStudio => "白底影棚",
            T::BlackBar => "黑色参数条",
            T::Minimal => "极简",
            T::Magazine => "杂志",
        },
    )
}

/// Localized label for a parameter field.
pub fn field_label(lang: Lang, f: framely_core::template::Field) -> &'static str {
    use framely_core::template::Field as F;
    lang.select(
        f.label(),
        match f {
            F::Lens => "镜头",
            F::Focal => "焦距",
            F::Aperture => "光圈",
            F::Shutter => "快门",
            F::Iso => "ISO",
            F::Date => "日期",
        },
    )
}

/// Try each candidate path in order; return the first readable file
/// whose contents parse as a font.
pub fn find_cjk_font(candidates: &[&str]) -> Option<CjkFont> {
    candidates.iter().find_map(|p| {
        let bytes = fs::read(p).ok()?;
        let (face_count, index) = pick_face(&bytes)?;
        Some(CjkFont {
            path: PathBuf::from(p),
            bytes,
            index,
            face_count,
        })
    })
}

/// Validate a font blob and choose a face index within it.
///
/// Deliberately light: check the sfnt/TTC/OTF magic (so a text file is
/// never handed to egui as a font) and read the TTC face count; within
/// a collection, prefer the first face whose raw name table mentions
/// `SC`/`GB` (Simplified Chinese). egui does the real parsing later and
/// rejects anything malformed, which just hides the language toggle.
fn pick_face(bytes: &[u8]) -> Option<(u32, u32)> {
    let magic: &[u8] = bytes.get(..4)?;
    if magic != b"ttcf" {
        let sfnt = magic == [0x00, 0x01, 0x00, 0x00] || magic == b"true" || magic == b"OTTO";
        return if sfnt { Some((1, 0)) } else { None };
    }
    let count = u32::from_be_bytes(bytes.get(8..12)?.try_into().ok()?).max(1);
    for i in 0..count.min(16) {
        // TTC header: tag(4) version(4) numFonts(4) then u32 offsets[].
        let off = u32::from_be_bytes(
            bytes
                .get(12 + i as usize * 4..16 + i as usize * 4)?
                .try_into()
                .ok()?,
        ) as usize;
        let n_tables = u16::from_be_bytes(bytes.get(off + 4..off + 6)?.try_into().ok()?) as usize;
        if face_prefers_simplified(bytes, off, n_tables) {
            return Some((count, i));
        }
    }
    Some((count, 0))
}

/// True if a face's `name` table mentions SC/GB. Scans the table's raw
/// bytes (UTF-16 or ASCII bigrams) instead of parsing records.
fn face_prefers_simplified(bytes: &[u8], sfnt_off: usize, n_tables: usize) -> bool {
    for t in 0..n_tables.min(64) {
        let Some(rec) = bytes.get(sfnt_off + 12 + t * 16..sfnt_off + 28 + t * 16) else {
            return false;
        };
        if rec[..4] != *b"name" {
            continue;
        }
        let name_off = u32::from_be_bytes(rec[8..12].try_into().unwrap()) as usize;
        let Some(region) = bytes.get(name_off..(name_off + 2048).min(bytes.len())) else {
            return false;
        };
        return region.windows(4).any(|w| w == [0, b'S', 0, b'C'])
            || region.windows(4).any(|w| w == [0, b'G', 0, b'B'])
            || region.windows(2).any(|w| w == *b"SC")
            || region.windows(2).any(|w| w == *b"GB");
    }
    false
}
