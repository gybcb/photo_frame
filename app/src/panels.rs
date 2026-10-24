//! Side panels: theme controls (left) and field overrides (right).
//!
//! Panels only read and mutate state; all rendering decisions live in
//! `framely-core`.

use egui::{Color32, RichText, Ui, vec2};
use framely_core::assets::LogoMode;
use framely_core::template::{
    BarEdge, BarStructure, Field, Fields, Separator, TemplateId, ThemeConfig,
};
use std::hash::{Hash, Hasher};

use crate::i18n::{Lang, field_label, template_name};

/// Which field the user chose to edit by hand (None = use the EXIF text).
#[derive(Clone, Debug, Default)]
pub struct FieldOverrides {
    /// `field -> manual text`; present means "edit by hand".
    pub manual: Vec<(Field, String)>,
    /// Free line that replaces the whole parameter line when non-empty.
    pub custom_line: String,
    pub use_custom_line: bool,
}

impl PartialEq for FieldOverrides {
    fn eq(&self, other: &Self) -> bool {
        // Two empty drafts are the same state; only typed text counts.
        effective_custom_line(self) == effective_custom_line(other)
            && self
                .manual
                .iter()
                .filter(|(_, t)| !t.is_empty())
                .eq(other.manual.iter().filter(|(_, t)| !t.is_empty()))
    }
}

impl Hash for FieldOverrides {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.use_custom_line {
            self.custom_line.hash(state);
        }
        for (f, t) in &self.manual {
            if !t.is_empty() {
                f.hash(state);
                t.hash(state);
            }
        }
    }
}

/// The custom line as it affects rendering: empty unless the toggle is on.
fn effective_custom_line(o: &FieldOverrides) -> &str {
    if o.use_custom_line {
        &o.custom_line
    } else {
        ""
    }
}

impl FieldOverrides {
    /// Merge the EXIF-derived fields with the user's manual values.
    /// Manual wins per field; the custom line wins over everything.
    /// Empty manual texts and a unticked custom line don't count as changes.
    pub fn apply(&self, exif: &Fields) -> Fields {
        let mut parts = exif.parts.clone();
        for (f, text) in &self.manual {
            if text.is_empty() {
                continue; // a blank draft is not a value — keep the EXIF one
            }
            if let Some(slot) = parts.iter_mut().find(|(k, _)| k == f) {
                slot.1 = text.clone();
            } else {
                parts.push((*f, text.clone()));
            }
        }
        let custom = if self.use_custom_line && !self.custom_line.is_empty() {
            Some(self.custom_line.clone())
        } else {
            None
        };
        Fields {
            parts,
            custom_line: custom,
        }
    }
}

/// Pick the theme for `template`, seeding defaults if absent.
pub fn theme_for(
    themes: &mut Vec<(TemplateId, ThemeConfig)>,
    template: TemplateId,
) -> &mut ThemeConfig {
    if let Some(i) = themes.iter().position(|(t, _)| *t == template) {
        &mut themes[i].1
    } else {
        themes.push((template, ThemeConfig::default_for(template)));
        &mut themes.last_mut().unwrap().1
    }
}

/// Left panel: templates, geometry, colors, logo mode, field toggles.
pub fn theme_panel(
    ui: &mut Ui,
    lang: Lang,
    template: &mut TemplateId,
    themes: &mut Vec<(TemplateId, ThemeConfig)>,
    logo_mode: &mut LogoMode,
    enabled: &mut Vec<Field>,
) {
    ui.heading(lang.select("Template", "模板"));
    ui.separator();
    let bar_group = [
        TemplateId::WhiteStudio,
        TemplateId::BlackBar,
        TemplateId::Minimal,
        TemplateId::Magazine,
        TemplateId::TwoColumn,
        TemplateId::Polaroid,
    ];
    let special_group = [TemplateId::TopBar, TemplateId::CaptionCard];
    for (title_en, title_zh, list) in [
        ("Bottom bar", "底条类", &bar_group[..]),
        ("Special structures", "特殊结构", &special_group[..]),
    ] {
        let holding = list.contains(template);
        egui::CollapsingHeader::new(lang.select(title_en, title_zh))
            .default_open(holding)
            .show(ui, |ui| {
                for &t in list {
                    if ui
                        .radio_value(template, t, template_name(lang, t))
                        .changed()
                        && t == TemplateId::TwoColumn
                        && !enabled.contains(&Field::Camera)
                    {
                        enabled.push(Field::Camera); // spec §5: Camera 默认开（仅该模板）
                    }
                }
            });
    }

    let cfg = theme_for(themes, *template);
    let spec = template.spec(); // radio may have switched templates
    ui.add_space(10.0);
    ui.heading(lang.select("Layout", "布局"));
    ui.separator();
    if cfg.margin_top < 0.0 {
        let b = cfg.border_ratio;
        cfg.margin_top = b;
        cfg.margin_bottom = b;
        cfg.margin_left = b;
        cfg.margin_right = b;
    }
    for (en, zh, slot) in [
        ("Margin top", "边宽·顶", 0),
        ("Margin bottom", "边宽·底", 1),
        ("Margin left", "边宽·左", 2),
        ("Margin right", "边宽·右", 3),
    ] {
        let f = match slot {
            0 => &mut cfg.margin_top,
            1 => &mut cfg.margin_bottom,
            2 => &mut cfg.margin_left,
            _ => &mut cfg.margin_right,
        };
        slider(ui, lang.select(en, zh), f, 0.005..=0.25);
    }
    cfg.border_ratio =
        (cfg.margin_top + cfg.margin_bottom + cfg.margin_left + cfg.margin_right) / 4.0;
    if spec.bar_edge != BarEdge::Left {
        ui.horizontal(|ui| {
            ui.label(lang.select("Bar position", "参数条位置"));
            for (edge, en, zh) in [
                (BarEdge::Bottom, "bottom", "下"),
                (BarEdge::Top, "top", "上"),
            ] {
                ui.selectable_value(&mut cfg.bar_position, edge, lang.select(en, zh));
            }
        });
    }
    if spec.structure != BarStructure::StackedLabeled {
        slider(
            ui,
            lang.select("Bar height", "参数条高度"),
            &mut cfg.bar_ratio,
            0.02..=0.20,
        );
        slider(
            ui,
            lang.select("Font size", "字号"),
            &mut cfg.font_scale,
            0.2..=0.9,
        );
        slider(
            ui,
            lang.select("Field spacing", "字段间距"),
            &mut cfg.part_spacing,
            0.2..=1.5,
        );
        ui.checkbox(
            &mut cfg.center_group,
            lang.select("Center logo + fields", "Logo 与参数整体居中"),
        );
        egui::ComboBox::from_label(lang.select("Separator", "分隔符"))
            .selected_text(match cfg.separator {
                Separator::Dot => "·",
                Separator::Bar => "|",
                Separator::Space => lang.select("space", "空格"),
            })
            .show_ui(ui, |ui| {
                for (s, lab) in [
                    (Separator::Dot, "·"),
                    (Separator::Bar, "|"),
                    (Separator::Space, lang.select("space", "空格")),
                ] {
                    ui.selectable_value(&mut cfg.separator, s, lab);
                }
            });
    }
    ui.checkbox(
        &mut cfg.show_labels,
        lang.select("Field labels", "字段标签"),
    )
    .on_hover_text(lang.select("only applies to Caption Card", "仅侧边签条生效"));
    if spec.structure == BarStructure::StackedLabeled && cfg.show_labels {
        slider(
            ui,
            lang.select("Label size", "标签比例"),
            &mut cfg.label_ratio,
            0.3..=0.9,
        );
    }

    ui.add_space(10.0);
    ui.heading(lang.select("Colors", "颜色"));
    ui.separator();
    let mut bg = Color32::from_rgba_unmultiplied(
        cfg.background[0],
        cfg.background[1],
        cfg.background[2],
        cfg.background[3],
    );
    if ui
        .color_edit_button_srgba(&mut bg)
        .on_hover_text(lang.select("frame background", "边框背景色"))
        .changed()
    {
        cfg.background = bg.to_array();
    }
    let mut fg = Color32::from_rgba_unmultiplied(
        cfg.foreground[0],
        cfg.foreground[1],
        cfg.foreground[2],
        cfg.foreground[3],
    );
    if ui
        .color_edit_button_srgba(&mut fg)
        .on_hover_text(lang.select("text / mono logo", "文字 / 单色 Logo"))
        .changed()
    {
        cfg.foreground = fg.to_array();
    }

    ui.add_space(10.0);
    ui.heading("Logo");
    ui.separator();
    ui.radio_value(
        logo_mode,
        LogoMode::Mono,
        lang.select("Monochrome (theme color)", "单色（随主题色）"),
    );
    ui.radio_value(
        logo_mode,
        LogoMode::Color,
        lang.select("Official colors", "官方配色"),
    );
    slider(
        ui,
        lang.select("Logo size", "Logo 大小"),
        &mut cfg.logo_scale,
        0.0..=1.0,
    );
    slider(
        ui,
        lang.select("Logo vertical", "Logo 上下位置"),
        &mut cfg.logo_y_offset,
        -0.5..=0.5,
    );

    ui.add_space(10.0);
    ui.heading(lang.select("Fields", "字段"));
    ui.separator();
    ui.label(
        RichText::new(lang.select("shown, in order:", "显示项（按顺序）："))
            .weak()
            .small(),
    );
    for f in [
        Field::Camera,
        Field::Lens,
        Field::Focal,
        Field::Aperture,
        Field::Shutter,
        Field::Iso,
        Field::Date,
    ] {
        let mut on = enabled.contains(&f);
        if ui.checkbox(&mut on, field_label(lang, f)).changed() {
            if on {
                enabled.retain(|x| *x != f);
                enabled.push(f); // append → display order follows toggles
            } else {
                enabled.retain(|x| *x != f);
            }
        }
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(lang.select(
            "Some options don't apply to the current template.",
            "部分选项不适用于当前模板。",
        ))
        .weak()
        .small(),
    );
}

fn slider(ui: &mut Ui, label: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>) {
    ui.add(egui::Slider::new(v, range).text(label));
}

/// Right panel: EXIF values with manual overrides.
pub fn fields_panel(
    ui: &mut Ui,
    lang: Lang,
    info: &framely_core::exif::PhotoInfo,
    overrides: &mut FieldOverrides,
) {
    ui.heading(lang.select("Shooting data", "拍摄数据"));
    ui.separator();
    ui.label(format!("Make: {}", dash_if_empty(&info.make)));
    ui.label(format!("Model: {}", dash_if_empty(&info.model)));

    ui.add_space(8.0);
    for f in [
        Field::Camera,
        Field::Lens,
        Field::Focal,
        Field::Aperture,
        Field::Shutter,
        Field::Iso,
        Field::Date,
    ] {
        let exif_text = info
            .fields
            .parts
            .iter()
            .find(|(k, _)| *k == f)
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        let manual = overrides
            .manual
            .iter()
            .find(|(k, _)| *k == f)
            .map(|(_, v)| v.clone());
        let editing = manual.is_some();

        ui.horizontal(|ui| {
            ui.label(RichText::new(field_label(lang, f)).strong().small());
            let mut txt = manual.clone().unwrap_or_default();
            let resp = ui.add_sized(
                vec2(140.0, 0.0),
                egui::TextEdit::singleline(&mut txt).hint_text(if exif_text.is_empty() {
                    lang.select("(no EXIF)", "（无 EXIF）")
                } else {
                    exif_text
                }),
            );
            if resp.changed() {
                set_manual(overrides, f, txt);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut on = editing;
                if ui
                    .checkbox(&mut on, lang.select("edit", "手动"))
                    .on_hover_text(lang.select("tick to type a manual value", "勾选后可手动输入"))
                    .changed()
                {
                    if on {
                        set_manual(overrides, f, exif_text.to_string());
                    } else {
                        overrides.manual.retain(|(k, _)| *k != f);
                    }
                }
            });
        });
    }

    ui.add_space(10.0);
    ui.weak(lang.select(
        "custom line (replaces all fields):",
        "整行自定义文本（替换所有字段）：",
    ));
    ui.checkbox(
        &mut overrides.use_custom_line,
        lang.select("use custom line", "使用整行自定义"),
    );
    ui.add_sized(
        vec2(ui.available_width(), 0.0),
        egui::TextEdit::singleline(&mut overrides.custom_line)
            .hint_text("e.g. ALPA 12 · 90mm · f/8 · 1/125"),
    );
}

fn dash_if_empty(s: &str) -> &str {
    if s.is_empty() { "—" } else { s }
}

fn set_manual(o: &mut FieldOverrides, f: Field, text: String) {
    match o.manual.iter_mut().find(|(k, _)| *k == f) {
        Some(slot) => slot.1 = text,
        None => o.manual.push((f, text)),
    }
}
