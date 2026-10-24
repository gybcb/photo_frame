//! Theme definitions: the template identity and its user-adjustable knobs.

use serde::{Deserialize, Serialize};

/// The built-in frame templates.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateId {
    /// White border all around; bottom bar with logo left, parameters right.
    WhiteStudio,
    /// Dark border and dark bar with light text; suited to night shots.
    BlackBar,
    /// Very thin border with a single small text line, logo inline.
    Minimal,
    /// Wide white border; logo on its own line, parameters on another.
    Magazine,
}

impl TemplateId {
    pub const ALL: [TemplateId; 4] = [
        TemplateId::WhiteStudio,
        TemplateId::BlackBar,
        TemplateId::Minimal,
        TemplateId::Magazine,
    ];

    pub fn id(self) -> &'static str {
        match self {
            TemplateId::WhiteStudio => "white-studio",
            TemplateId::BlackBar => "black-bar",
            TemplateId::Minimal => "minimal",
            TemplateId::Magazine => "magazine",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            TemplateId::WhiteStudio => "White Studio",
            TemplateId::BlackBar => "Black Bar",
            TemplateId::Minimal => "Minimal",
            TemplateId::Magazine => "Magazine",
        }
    }
}

/// One parameter slot a frame can show, in default display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Field {
    Lens,
    Focal,
    Aperture,
    Shutter,
    Iso,
    Date,
}

impl Field {
    pub fn id(self) -> &'static str {
        match self {
            Field::Lens => "lens",
            Field::Focal => "focal",
            Field::Aperture => "aperture",
            Field::Shutter => "shutter",
            Field::Iso => "iso",
            Field::Date => "date",
        }
    }
}

/// The field values to render, already formatted for display.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fields {
    /// `(field, formatted value)` pairs, in display order, missing ones absent.
    pub parts: Vec<(Field, String)>,
    /// Free text that replaces all parts wholesale (manual "full line").
    pub custom_line: Option<String>,
}

/// User-adjustable look of a template. Colors are RGBA 0..=255.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Outer border width as a fraction of the photo's short side.
    pub border_ratio: f64,
    /// Parameter bar height as a fraction of the photo's short side.
    pub bar_ratio: f64,
    /// Text size relative to the bar height.
    pub font_scale: f64,
    /// Logo height relative to the bar height (0 disables the logo).
    pub logo_scale: f64,
    /// Frame background color.
    pub background: [u8; 4],
    /// Text color.
    pub foreground: [u8; 4],
    /// Gaps between parts, relative to the text size.
    pub part_spacing: f64,
}

impl ThemeConfig {
    /// Sensible defaults per template (colors follow the template concept).
    pub fn default_for(tpl: TemplateId) -> ThemeConfig {
        let base = ThemeConfig {
            border_ratio: 0.04,
            bar_ratio: 0.06,
            font_scale: 0.45,
            logo_scale: 0.6,
            background: [255, 255, 255, 255],
            foreground: [26, 26, 26, 255],
            part_spacing: 0.8,
        };
        match tpl {
            TemplateId::WhiteStudio => base,
            TemplateId::BlackBar => ThemeConfig {
                background: [17, 17, 17, 255],
                foreground: [240, 240, 240, 255],
                ..base
            },
            TemplateId::Minimal => ThemeConfig {
                border_ratio: 0.015,
                bar_ratio: 0.035,
                font_scale: 0.55,
                logo_scale: 0.5,
                ..base
            },
            TemplateId::Magazine => ThemeConfig {
                border_ratio: 0.08,
                bar_ratio: 0.11,
                font_scale: 0.38,
                logo_scale: 0.5,
                ..base
            },
        }
    }
}

/// Drop order when the line does not fit: least important first.
/// Exposure trio (aperture/shutter/ISO) is protected until last.
pub const FIELD_DROP_ORDER: [Field; 6] = [
    Field::Date,
    Field::Lens,
    Field::Focal,
    Field::Aperture,
    Field::Shutter,
    Field::Iso,
];
