//! Theme definitions: the template identity, its descriptor, and the
//! user-adjustable knobs.

use serde::{Deserialize, Serialize};

/// Which canvas edge the parameter bar runs along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarEdge {
    #[default]
    Bottom,
    Top,
    Left,
}

/// How the bar's content is arranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BarStructure {
    /// Logo and one parameter line in a single row.
    SingleLine,
    /// Logo left, parameters split over two lines (Magazine).
    TwoLine,
    /// Logo and parameters in two columns (Two-Column).
    TwoColumn,
    /// Each parameter on its own labeled row (Caption Card).
    StackedLabeled,
}

/// Joiner between parameter parts on one line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Separator {
    #[default]
    Dot,
    Bar,
    Space,
}

/// The built-in frame templates.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateId {
    /// White border all around; bottom bar with logo left, parameters right.
    #[default]
    WhiteStudio,
    /// Dark border and dark bar with light text; suited to night shots.
    BlackBar,
    /// Very thin border with a single small text line, logo inline.
    Minimal,
    /// Wide white border; logo on its own line, parameters on another.
    Magazine,
    /// Polaroid look: white frame, deep bottom margin, small bottom bar.
    Polaroid,
    /// White frame; bar split into a left logo column and right parameters.
    TwoColumn,
    /// White frame; parameter bar above the photo, thin bottom margin.
    TopBar,
    /// White frame with a wide left column of stacked, labeled parameters.
    CaptionCard,
}

impl TemplateId {
    pub const ALL: [TemplateId; 8] = [
        TemplateId::WhiteStudio,
        TemplateId::BlackBar,
        TemplateId::Minimal,
        TemplateId::Magazine,
        TemplateId::Polaroid,
        TemplateId::TwoColumn,
        TemplateId::TopBar,
        TemplateId::CaptionCard,
    ];

    pub fn id(self) -> &'static str {
        match self {
            TemplateId::WhiteStudio => "white-studio",
            TemplateId::BlackBar => "black-bar",
            TemplateId::Minimal => "minimal",
            TemplateId::Magazine => "magazine",
            TemplateId::Polaroid => "polaroid",
            TemplateId::TwoColumn => "two-column",
            TemplateId::TopBar => "top-bar",
            TemplateId::CaptionCard => "caption-card",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            TemplateId::WhiteStudio => "White Studio",
            TemplateId::BlackBar => "Black Bar",
            TemplateId::Minimal => "Minimal",
            TemplateId::Magazine => "Magazine",
            TemplateId::Polaroid => "Polaroid",
            TemplateId::TwoColumn => "Two-Column",
            TemplateId::TopBar => "Top Bar",
            TemplateId::CaptionCard => "Caption Card",
        }
    }

    /// Static descriptor for this template: geometry and structure the
    /// layout engine drives off of.
    pub fn spec(self) -> &'static TemplateSpec {
        match self {
            TemplateId::WhiteStudio => &WHITE_STUDIO,
            TemplateId::BlackBar => &BLACK_BAR,
            TemplateId::Minimal => &MINIMAL,
            TemplateId::Magazine => &MAGAZINE,
            TemplateId::Polaroid => &POLAROID,
            TemplateId::TwoColumn => &TWO_COLUMN,
            TemplateId::TopBar => &TOP_BAR,
            TemplateId::CaptionCard => &CAPTION_CARD,
        }
    }
}

/// Everything about a template that is fixed by design (as opposed to the
/// user-adjustable knobs on `ThemeConfig`). Margins are fractions of the
/// photo's short side, in top/bottom/left/right order.
#[derive(Debug)]
pub struct TemplateSpec {
    pub id: TemplateId,
    pub display_name: &'static str,
    pub bar_edge: BarEdge,
    pub margins: [f64; 4],
    pub structure: BarStructure,
    pub separator: Separator,
    /// Caption-label height relative to the parameter font (labeled
    /// structures only).
    pub label_ratio: f64,
}

const WHITE_STUDIO: TemplateSpec = TemplateSpec {
    id: TemplateId::WhiteStudio,
    display_name: "White Studio",
    bar_edge: BarEdge::Bottom,
    margins: [0.04, 0.04, 0.04, 0.04],
    structure: BarStructure::SingleLine,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const BLACK_BAR: TemplateSpec = TemplateSpec {
    id: TemplateId::BlackBar,
    display_name: "Black Bar",
    bar_edge: BarEdge::Bottom,
    margins: [0.04, 0.04, 0.04, 0.04],
    structure: BarStructure::SingleLine,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const MINIMAL: TemplateSpec = TemplateSpec {
    id: TemplateId::Minimal,
    display_name: "Minimal",
    bar_edge: BarEdge::Bottom,
    margins: [0.015, 0.015, 0.015, 0.015],
    structure: BarStructure::SingleLine,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const MAGAZINE: TemplateSpec = TemplateSpec {
    id: TemplateId::Magazine,
    display_name: "Magazine",
    bar_edge: BarEdge::Bottom,
    margins: [0.08, 0.08, 0.08, 0.08],
    structure: BarStructure::TwoLine,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const POLAROID: TemplateSpec = TemplateSpec {
    id: TemplateId::Polaroid,
    display_name: "Polaroid",
    bar_edge: BarEdge::Bottom,
    margins: [0.03, 0.14, 0.03, 0.03],
    structure: BarStructure::SingleLine,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const TWO_COLUMN: TemplateSpec = TemplateSpec {
    id: TemplateId::TwoColumn,
    display_name: "Two-Column",
    bar_edge: BarEdge::Bottom,
    margins: [0.04, 0.04, 0.04, 0.04],
    structure: BarStructure::TwoColumn,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const TOP_BAR: TemplateSpec = TemplateSpec {
    id: TemplateId::TopBar,
    display_name: "Top Bar",
    bar_edge: BarEdge::Top,
    margins: [0.04, 0.06, 0.04, 0.04],
    structure: BarStructure::SingleLine,
    separator: Separator::Dot,
    label_ratio: 0.6,
};
const CAPTION_CARD: TemplateSpec = TemplateSpec {
    id: TemplateId::CaptionCard,
    display_name: "Caption Card",
    bar_edge: BarEdge::Left,
    margins: [0.03, 0.03, 0.06, 0.03],
    structure: BarStructure::StackedLabeled,
    separator: Separator::Dot,
    label_ratio: 0.62,
};

/// One parameter slot a frame can show, in default display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Field {
    Lens,
    Focal,
    Aperture,
    Shutter,
    Iso,
    Date,
    Camera,
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
            Field::Camera => "camera",
        }
    }

    /// Human-readable name for UI labels.
    pub fn label(self) -> &'static str {
        match self {
            Field::Lens => "Lens",
            Field::Focal => "Focal",
            Field::Aperture => "Aperture",
            Field::Shutter => "Shutter",
            Field::Iso => "ISO",
            Field::Date => "Date",
            Field::Camera => "Camera",
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

/// Sentinel for a margin the preset never set, so it can fall back to
/// `border_ratio`.
const MARGIN_UNSET: f64 = -1.0;

fn margin_unset() -> f64 {
    MARGIN_UNSET
}

/// User-adjustable look of a template. Colors are RGBA 0..=255.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Outer border width as a fraction of the photo's short side. Kept as
    /// the mean of the four margins on save so older readers stay sane.
    pub border_ratio: f64,
    /// Parameter bar height as a fraction of the photo's short side.
    pub bar_ratio: f64,
    /// Text size relative to the bar height.
    pub font_scale: f64,
    /// Logo height relative to the bar height (0 disables the logo).
    pub logo_scale: f64,
    /// Logo vertical nudge as a fraction of the bar height (0 = centered
    /// in the bar; negative raises, positive sinks; clamped so the logo
    /// never leaves the bar). `serde(default)` keeps old presets valid.
    #[serde(default)]
    pub logo_y_offset: f64,
    /// Frame background color.
    pub background: [u8; 4],
    /// Text color.
    pub foreground: [u8; 4],
    /// Gaps between parts, relative to the text size.
    pub part_spacing: f64,
    /// Center the logo + parameters as one group in the bar (default:
    /// logo left, parameters right). `serde(default)` keeps old presets
    /// loadable.
    #[serde(default)]
    pub center_group: bool,
    /// Per-edge margins as fractions of the photo's short side, top/bottom/
    /// left/right. A negative value means "unset": the edge follows
    /// `border_ratio` (see `effective_margins`). Presets from before these
    /// fields existed deserialize to all-unset.
    #[serde(default = "margin_unset")]
    pub margin_top: f64,
    #[serde(default = "margin_unset")]
    pub margin_bottom: f64,
    #[serde(default = "margin_unset")]
    pub margin_left: f64,
    #[serde(default = "margin_unset")]
    pub margin_right: f64,
    /// Which canvas edge the parameter bar runs along.
    #[serde(default)]
    pub bar_position: BarEdge,
    /// Joiner between parameter parts on one line.
    #[serde(default)]
    pub separator: Separator,
    /// Show per-parameter labels above the values (labeled structures).
    #[serde(default)]
    pub show_labels: bool,
    /// Caption-label size relative to the parameter font.
    #[serde(default = "default_label_ratio")]
    pub label_ratio: f64,
}

fn default_label_ratio() -> f64 {
    0.6
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::default_for(TemplateId::WhiteStudio)
    }
}

impl std::hash::Hash for ThemeConfig {
    /// Bit-wise, so it can live in the app's dirty-check hash. Sliders
    /// quantise to discrete steps during interaction, so the preview
    /// re-renders on every visible change.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for v in [
            self.border_ratio,
            self.bar_ratio,
            self.font_scale,
            self.logo_scale,
            self.logo_y_offset,
            self.part_spacing,
            self.margin_top,
            self.margin_bottom,
            self.margin_left,
            self.margin_right,
            self.label_ratio,
        ] {
            state.write(&v.to_bits().to_le_bytes());
        }
        self.background.hash(state);
        self.foreground.hash(state);
        self.center_group.hash(state);
        self.bar_position.hash(state);
        self.separator.hash(state);
        self.show_labels.hash(state);
    }
}

impl ThemeConfig {
    /// Per-edge margins to lay out with: any edge still at the sentinel
    /// (or below zero) follows `border_ratio`; if all four are unset the
    /// frame is uniform at `border_ratio`.
    pub fn effective_margins(&self) -> [f64; 4] {
        if self.margin_top < 0.0
            && self.margin_bottom < 0.0
            && self.margin_left < 0.0
            && self.margin_right < 0.0
        {
            [self.border_ratio; 4]
        } else {
            [
                self.margin_top.max(0.0),
                self.margin_bottom.max(0.0),
                self.margin_left.max(0.0),
                self.margin_right.max(0.0),
            ]
        }
    }

    /// Materializes the effective margins into the four fields and folds
    /// their mean back into `border_ratio`, so saved presets carry both
    /// the detailed and the legacy-compatible numbers.
    pub fn normalize_for_save(&mut self) {
        let m = self.effective_margins();
        self.margin_top = m[0];
        self.margin_bottom = m[1];
        self.margin_left = m[2];
        self.margin_right = m[3];
        self.border_ratio = (m[0] + m[1] + m[2] + m[3]) / 4.0;
    }

    /// The string a separator renders as between parts.
    pub fn separator_str(sep: Separator) -> &'static str {
        match sep {
            Separator::Dot => " · ",
            Separator::Bar => " | ",
            Separator::Space => " ",
        }
    }

    /// Sensible defaults per template (colors follow the template concept).
    /// The four original templates keep their historical values, with the
    /// new margin fields pre-filled to the same scalar border so layout
    /// behavior is unchanged; the newer templates carry asymmetric margins
    /// from their `TemplateSpec`.
    pub fn default_for(tpl: TemplateId) -> ThemeConfig {
        let base = ThemeConfig {
            border_ratio: 0.04,
            bar_ratio: 0.06,
            font_scale: 0.45,
            logo_scale: 0.6,
            logo_y_offset: 0.0,
            background: [255, 255, 255, 255],
            foreground: [26, 26, 26, 255],
            part_spacing: 0.8,
            center_group: false,
            margin_top: 0.04,
            margin_bottom: 0.04,
            margin_left: 0.04,
            margin_right: 0.04,
            bar_position: tpl.spec().bar_edge,
            separator: Separator::Dot,
            show_labels: false,
            label_ratio: tpl.spec().label_ratio,
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
                margin_top: 0.015,
                margin_bottom: 0.015,
                margin_left: 0.015,
                margin_right: 0.015,
                ..base
            },
            TemplateId::Magazine => ThemeConfig {
                border_ratio: 0.08,
                bar_ratio: 0.11,
                font_scale: 0.38,
                logo_scale: 0.5,
                margin_top: 0.08,
                margin_bottom: 0.08,
                margin_left: 0.08,
                margin_right: 0.08,
                ..base
            },
            TemplateId::Polaroid => ThemeConfig {
                border_ratio: 0.0575,
                bar_ratio: 0.035,
                logo_scale: 0.0,
                margin_top: 0.03,
                margin_bottom: 0.14,
                margin_left: 0.03,
                margin_right: 0.03,
                ..base
            },
            TemplateId::TwoColumn => ThemeConfig {
                logo_scale: 0.5,
                ..base
            },
            TemplateId::TopBar => ThemeConfig {
                border_ratio: 0.045,
                margin_bottom: 0.06,
                ..base
            },
            TemplateId::CaptionCard => ThemeConfig {
                border_ratio: 0.0375,
                font_scale: 0.5,
                logo_scale: 0.5,
                margin_top: 0.03,
                margin_bottom: 0.03,
                margin_left: 0.06,
                margin_right: 0.03,
                show_labels: true,
                ..base
            },
        }
    }
}

/// Drop order when the line does not fit: least important first.
/// Exposure trio (aperture/shutter/ISO) is protected until last.
pub const FIELD_DROP_ORDER: [Field; 7] = [
    Field::Date,
    Field::Camera,
    Field::Lens,
    Field::Focal,
    Field::Aperture,
    Field::Shutter,
    Field::Iso,
];
