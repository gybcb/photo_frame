//! Language selection and the paired-string helper, plus the pieces
//! needed to render Chinese: system-locale detection, a persisted
//! config, and a probe for a CJK-capable system font.
//!
//! Strings live at the call site as pairs (`select(en, zh)`), the
//! egui-recommended pattern: no key table to drift, and an English
//! original is always visible next to its translation.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Host platform family, for the pieces that differ per OS: where Chinese
/// fonts live, and where preferences belong. Injected explicitly (never
/// read from `cfg!`) so all three paths compile and test on any host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Macos,
    Windows,
    Linux,
}

impl Platform {
    /// The platform this binary is running on.
    pub fn current() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::Macos
        } else if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Linux
        }
    }
}

/// Candidate font files for Chinese on `platform`, in preference order.
/// `env` reads an environment variable (injectable for tests); Windows
/// uses it for `WINDIR`.
pub fn cjk_candidates_on(
    platform: Platform,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Vec<PathBuf> {
    let paths: Vec<PathBuf> = match platform {
        Platform::Macos => [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ]
        .iter()
        .map(PathBuf::from)
        .collect(),
        Platform::Linux => [
            // Debian/Ubuntu (fonts-noto-cjk)
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            // Arch (noto-fonts-cjk)
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            // Fedora 38+ (google-noto-sans-cjk-fonts)
            "/usr/share/fonts/google-noto-sans-cjk-fonts/NotoSansCJK-Regular.ttc",
            // Older Fedora/RHEL
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            // WenQuanYi Micro Hei (the classic Linux CJK fallback)
            "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        ]
        .iter()
        .map(PathBuf::from)
        .collect(),
        Platform::Windows => {
            // Microsoft YaHei ships with every Chinese-capable Windows;
            // its TTC's face 0 is the SC face. Fall back to GDI's msyh.ttf
            // shape only as a name guess — reading is the real filter.
            let windir = env("WINDIR")
                .and_then(|w| w.into_string().ok())
                .unwrap_or_else(|| "C:\\Windows".into());
            [
                format!("{windir}\\Fonts\\msyh.ttc"),
                format!("{windir}\\Fonts\\msyh.ttf"),
                format!("{windir}\\Fonts\\simhei.ttf"),
                // Older installs kept fonts under the Windows dir itself.
                format!("{windir}\\Fonts\\simsun.ttc"),
            ]
            .iter()
            .map(PathBuf::from)
            .collect()
        }
    };
    paths
}

/// Preference-file location on `platform`, reading vars through `env`:
/// macOS keeps the historical `~/.framely` (existing users' configs stay
/// valid); Linux follows XDG; Windows uses `%APPDATA%`. A cwd file is the
/// last resort so nothing ever panics on a bare environment.
pub fn config_path_on(platform: Platform, env: &dyn Fn(&str) -> Option<OsString>) -> PathBuf {
    let file = |dir: &OsStr| PathBuf::from(dir).join("framely").join("config.json");
    match platform {
        // Historical location — do not "fix" this to XDG, or upgrades lose
        // the saved language choice.
        Platform::Macos => env("HOME")
            .map(|h| PathBuf::from(h).join(".framely").join("config.json"))
            .unwrap_or_else(|| PathBuf::from("framely-config.json")),
        Platform::Linux => env("XDG_CONFIG_HOME")
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".config").into_os_string()))
            .map(|d| file(&d))
            .unwrap_or_else(|| PathBuf::from("framely-config.json")),
        Platform::Windows => env("APPDATA")
            .map(|d| file(&d))
            .unwrap_or_else(|| PathBuf::from("framely-config.json")),
    }
}

/// Home directory on `platform` (`%USERPROFILE%` on Windows, `$HOME`
/// elsewhere; `.` when nothing is set). Used for dialog defaults.
pub fn home_dir_on(platform: Platform, env: &dyn Fn(&str) -> Option<OsString>) -> PathBuf {
    let key = match platform {
        Platform::Windows => "USERPROFILE",
        Platform::Macos | Platform::Linux => "HOME",
    };
    env(key)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// This machine's home directory (see [`home_dir_on`]).
pub fn home_dir() -> PathBuf {
    home_dir_on(Platform::current(), &|k| std::env::var_os(k))
}

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

/// User preferences, persisted to the per-platform config file
/// (see [`config_path`]).
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
pub fn cjk_candidates() -> Vec<PathBuf> {
    cjk_candidates_on(Platform::current(), &|k| std::env::var_os(k))
}

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

/// The persisted-config location on this platform (see
/// [`config_path_on`]: `~/.framely`, `$XDG_CONFIG_HOME/framely`, or
/// `%APPDATA%\framely`; cwd file as last resort).
pub fn config_path() -> PathBuf {
    config_path_on(Platform::current(), &|k| std::env::var_os(k))
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
            T::Polaroid => "拍立得",
            T::TwoColumn => "双栏相机",
            T::TopBar => "顶部参数条",
            T::CaptionCard => "侧边签条",
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
            F::Camera => "机身",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use framely_core::template::{Field, TemplateId};

    #[test]
    fn all_templates_have_chinese_names() {
        for t in TemplateId::ALL {
            assert_ne!(template_name(Lang::Zh, t), t.display_name(), "{t:?} missing zh name");
        }
    }

    /// The four templates added with the template system must carry the
    /// finalized names, not the placeholder guesses left by the compile
    /// lifeline (which a bare non-English check would not catch).
    #[test]
    fn new_template_names_are_finalized() {
        assert_eq!(template_name(Lang::Zh, TemplateId::Polaroid), "拍立得");
        assert_eq!(template_name(Lang::Zh, TemplateId::TwoColumn), "双栏相机");
        assert_eq!(template_name(Lang::Zh, TemplateId::TopBar), "顶部参数条");
        assert_eq!(template_name(Lang::Zh, TemplateId::CaptionCard), "侧边签条");
    }

    #[test]
    fn camera_label_is_body() {
        assert_eq!(field_label(Lang::Zh, Field::Camera), "机身");
    }
}

/// Try each candidate path in order; return the first readable file
/// whose contents parse as a font.
pub fn find_cjk_font<P: AsRef<Path>>(candidates: &[P]) -> Option<CjkFont> {
    candidates.iter().find_map(|p| {
        let path = p.as_ref();
        let bytes = fs::read(path).ok()?;
        let (face_count, index) = pick_face(&bytes)?;
        Some(CjkFont {
            path: path.to_path_buf(),
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
