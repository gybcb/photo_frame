//! Geometry for the templates: turns a photo size + theme into a list of
//! drawing operations, every dimension derived from the canvas scale.

use crate::brand::Brand;
use crate::template::{BarEdge, BarStructure, Field, Fields, TemplateId, ThemeConfig};

/// Text anchor within its region box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// A prepared drawing instruction for `render::compose`.
#[derive(Clone, Debug)]
pub enum DrawOp {
    Text {
        content: String,
        size_px: u32,
        color: [u8; 4],
        align: Align,
        /// Region box (x, y, width, height) the text is anchored inside.
        rect: (u32, u32, u32, u32),
    },
    Logo {
        brand: Brand,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
}

/// The complete layout of one framed image.
#[derive(Clone, Debug)]
pub struct FrameLayout {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub border: u32,
    pub bar_height: u32,
    /// (x, y, width, height) of the photo inside the frame.
    pub photo_rect: (u32, u32, u32, u32),
    pub background: [u8; 4],
    /// Theme text/mono-logo color. Carried here so the mono logo can be
    /// painted even when no text op exists to infer it from.
    pub foreground: [u8; 4],
    pub ops: Vec<DrawOp>,
}

/// Everything the bar structures share, computed once per `layout()` call.
struct BarCtx<'a> {
    /// Canvas y of the bar band's top edge.
    band_y: u32,
    mt: u32,
    ml: u32,
    canvas_w: u32,
    photo_w: u32,
    photo_h: u32,
    /// Band thickness (`bar` for top/bottom edges, the left margin itself
    /// for the left-edge caption).
    bar_h: u32,
    font: u32,
    inner_pad: u32,
    logo_h: u32,
    logo_slot: u32,
    brand: Brand,
    theme: &'a ThemeConfig,
    fields: &'a Fields,
    sep: &'static str,
}

/// Logo top position inside the bar: vertically centered, nudged by the
/// theme's fraction of the bar height, clamped so the logo never leaves
/// the bar at extreme offsets.
fn logo_y(bar_y: u32, bar: u32, h: u32, offset: f64) -> u32 {
    let slack = bar.saturating_sub(h);
    let nudge = (bar as f64 * offset).round() as i64;
    ((slack / 2) as i64 + nudge).clamp(0, slack as i64) as u32 + bar_y
}

/// Logo box height and horizontal slot for a bar of `bar_h` (0 slot when
/// the brand is unknown or the logo is disabled).
fn logo_metrics(brand: Brand, logo_ratio: f64, theme: &ThemeConfig, bar_h: u32) -> (u32, u32) {
    let logo_h = (bar_h as f64 * theme.logo_scale).round() as u32;
    let logo_slot = if brand == Brand::Unknown || theme.logo_scale <= 0.0 {
        0
    } else {
        ((logo_h as f64) * logo_ratio).round().max(1.0) as u32
    };
    (logo_h, logo_slot)
}

/// Lays out `tpl` around a `photo_w x photo_h` image, driven entirely by
/// the template's `TemplateSpec`. The parameter bar sits along the edge the
/// spec (or, where allowed, the theme's `bar_position`) selects.
/// `logo_ratio` is the brand logo's width/height aspect, used to size the
/// logo box without distorting it.
pub fn layout(
    tpl: TemplateId,
    photo_w: u32,
    photo_h: u32,
    brand: Brand,
    logo_ratio: f64,
    theme: &ThemeConfig,
    fields: &Fields,
) -> FrameLayout {
    let spec = tpl.spec();
    let short = photo_w.min(photo_h) as f64;
    let m = theme.effective_margins();
    let (mt, mb, ml, mr) = (m[0], m[1], m[2], m[3]);
    let mt = (short * mt).round().max(1.0) as u32;
    let mb = (short * mb).round().max(1.0) as u32;
    let ml = (short * ml).round().max(1.0) as u32;
    let mr = (short * mr).round().max(1.0) as u32;
    let bar = (short * theme.bar_ratio).round().max(12.0) as u32;

    let edge = match spec.bar_edge {
        // Caption Card keeps its left column whatever the knob says.
        BarEdge::Left => BarEdge::Left,
        _ => theme.bar_position,
    };

    // Canvas & photo rect; the bar occupies its own band on `edge`.
    let (bar_h, canvas_w, canvas_h, photo_x, photo_y);
    match edge {
        BarEdge::Bottom => {
            canvas_w = photo_w + ml + mr;
            canvas_h = photo_h + mt + mb + bar;
            photo_x = ml;
            photo_y = mt;
            bar_h = bar;
        }
        BarEdge::Top => {
            canvas_w = photo_w + ml + mr;
            canvas_h = photo_h + mt + mb + bar;
            photo_x = ml;
            photo_y = mt + bar;
            bar_h = bar;
        }
        BarEdge::Left => {
            // Caption Card: text lives inside the left margin; no extra band.
            canvas_w = photo_w + ml + mr;
            canvas_h = photo_h + mt + mb;
            photo_x = ml;
            photo_y = mt;
            bar_h = ml;
        }
    }

    let band_y = match edge {
        BarEdge::Bottom => mt + photo_h,
        BarEdge::Top => mt,
        BarEdge::Left => mt,
    };

    // Shared bar inputs (logo slot, font, inner pad) — mirror current math.
    let (logo_h, logo_slot) = logo_metrics(brand, logo_ratio, theme, bar_h);
    let font = (bar_h as f64 * theme.font_scale).round().max(9.0) as u32;
    let inner_pad = (bar_h as f64 * 0.25).round() as u32;

    let ctx = BarCtx {
        band_y,
        mt,
        ml,
        canvas_w,
        photo_w,
        photo_h,
        bar_h,
        font,
        inner_pad,
        logo_h,
        logo_slot,
        brand,
        theme,
        fields,
        sep: ThemeConfig::separator_str(theme.separator),
    };

    let mut ops = Vec::new();
    match spec.structure {
        BarStructure::SingleLine => bar_single(&mut ops, &ctx),
        BarStructure::TwoLine => bar_two_line(&mut ops, &ctx),
        BarStructure::TwoColumn => bar_two_column(&mut ops, &ctx),
        BarStructure::StackedLabeled => bar_stacked_labeled(&mut ops, &ctx),
    }

    FrameLayout {
        canvas_width: canvas_w,
        canvas_height: canvas_h,
        border: mt.max(mb).max(ml).max(mr),
        bar_height: bar_h,
        photo_rect: (photo_x, photo_y, photo_w, photo_h),
        background: theme.background,
        foreground: theme.foreground,
        ops,
    }
}

/// Logo left, one right-anchored parameter line in the band.
fn bar_single(ops: &mut Vec<DrawOp>, c: &BarCtx) {
    let has_logo = c.logo_slot > 0;
    // Fit the text: drop fields by priority, then shrink the font, until it
    // fits the region left of (or around) the logo.
    let gap = if has_logo { c.font } else { 0 };
    let text_region_w = c
        .photo_w
        .saturating_sub(2 * c.inner_pad + c.logo_slot + gap);
    let fitted = fit_fields(c.fields, c.font, text_region_w, c.sep);

    // Optically-centered variant: measure the real rendered width so the
    // group's left and right canvas margins match to the pixel; the fit
    // decisions above stay on the cheap estimate.
    let centered = c.theme.center_group;
    let joined = joined_text(&fitted, c.sep);
    let text_w = if centered && !joined.is_empty() {
        crate::render::measure_text(&joined, fitted.size_px)
    } else {
        0
    };
    let group_x = if centered {
        // The gap only exists between two real pieces; one lone piece
        // must not carry phantom space.
        let both = has_logo && text_w > 0;
        let group_w =
            (if has_logo { c.logo_slot } else { 0 }) + (if both { gap } else { 0 }) + text_w;
        c.canvas_w.saturating_sub(group_w) / 2
    } else {
        0
    };

    if has_logo {
        let h = c.logo_h.max(1);
        ops.push(DrawOp::Logo {
            brand: c.brand,
            x: if centered {
                group_x
            } else {
                c.ml + c.inner_pad
            },
            y: logo_y(c.band_y, c.bar_h, h, c.theme.logo_y_offset),
            width: c.logo_slot,
            height: h,
        });
    }
    if !joined.is_empty() {
        ops.push(DrawOp::Text {
            content: joined,
            size_px: fitted.size_px,
            color: c.theme.foreground,
            align: if centered { Align::Left } else { Align::Right },
            rect: if centered {
                (
                    group_x + if has_logo { c.logo_slot + gap } else { 0 },
                    c.band_y,
                    text_w,
                    c.bar_h,
                )
            } else {
                (
                    c.ml + c.inner_pad + c.logo_slot + gap,
                    c.band_y,
                    text_region_w,
                    c.bar_h,
                )
            },
        });
    }
}

/// Logo left, parameters split over two right-anchored lines (Magazine).
fn bar_two_line(ops: &mut Vec<DrawOp>, c: &BarCtx) {
    let has_logo = c.logo_slot > 0;
    let gap = if has_logo { c.font } else { 0 };
    let text_region_w = c
        .photo_w
        .saturating_sub(2 * c.inner_pad + c.logo_slot + gap);
    let fitted = fit_fields(c.fields, c.font, text_region_w, c.sep);

    let two_lines = c.fields.custom_line.is_none() && c.fields.parts.len() > 1;

    let centered = c.theme.center_group;
    let joined = joined_text(&fitted, c.sep);
    let text_w = if centered && !joined.is_empty() {
        crate::render::measure_text(&joined, fitted.size_px)
    } else {
        0
    };
    let group_x = if centered {
        let both = has_logo && text_w > 0;
        let group_w =
            (if has_logo { c.logo_slot } else { 0 }) + (if both { gap } else { 0 }) + text_w;
        c.canvas_w.saturating_sub(group_w) / 2
    } else {
        0
    };

    if has_logo {
        ops.push(DrawOp::Logo {
            brand: c.brand,
            x: if centered {
                group_x
            } else {
                c.ml + c.inner_pad
            },
            y: logo_y(c.band_y, c.bar_h, c.logo_h.max(1), c.theme.logo_y_offset),
            width: c.logo_slot,
            height: c.logo_h.max(1),
        });
    }
    let lines: Vec<String> = if two_lines {
        let mid = c.fields.parts.len().div_ceil(2);
        vec![
            join(&c.fields.parts[..mid], c.sep),
            join(&c.fields.parts[mid..], c.sep),
        ]
    } else {
        vec![joined_text(&fitted, c.sep)]
    };
    let line_h = (c.bar_h / lines.len().max(1) as u32).max(1);
    for (i, content) in lines.iter().enumerate() {
        if content.is_empty() {
            continue;
        }
        // Centered: each line right-aligns inside the measured
        // text block (no single line is wider than the joined
        // string), so the group's margins stay equal.
        let (rx, rw) = if centered {
            (
                group_x + if has_logo { c.logo_slot + gap } else { 0 },
                text_w,
            )
        } else {
            (c.ml + c.inner_pad + c.logo_slot + gap, text_region_w)
        };
        ops.push(DrawOp::Text {
            content: content.clone(),
            size_px: fitted.size_px.min((line_h as f64 * 0.8) as u32),
            color: c.theme.foreground,
            align: Align::Right,
            rect: (rx, c.band_y + i as u32 * line_h, rw, line_h),
        });
    }
}

/// Logo and model left, parameters right in a smaller size (Two-Column).
/// Degrades to the single-line bar when no camera value is available.
fn bar_two_column(ops: &mut Vec<DrawOp>, c: &BarCtx) {
    let camera = c
        .fields
        .parts
        .iter()
        .find(|(f, _)| *f == Field::Camera)
        .map(|(_, v)| v.as_str().to_string());
    let has_camera = matches!(&camera, Some(v) if !v.is_empty());
    if !has_camera || c.fields.custom_line.is_some() {
        // No model to give the left column: one joined parameter line,
        // laid out exactly like the single-line bar.
        bar_single(ops, c);
        return;
    }
    let camera = camera.unwrap();

    let has_logo = c.logo_slot > 0;
    let gap = if has_logo { c.font } else { 0 };
    let right_size = ((c.font as f64) * 0.85).round().max(1.0) as u32;
    // Left column width: the model's estimated width at its real size,
    // capped so the right column keeps room.
    let left_cap = (0.4 * (c.canvas_w - 2 * c.inner_pad) as f64) as u32;
    let left_w = estimate_width(&camera, c.font).min(left_cap);
    let right_fields = Fields {
        parts: c
            .fields
            .parts
            .iter()
            .filter(|(f, _)| *f != Field::Camera)
            .cloned()
            .collect(),
        custom_line: None,
    };
    let right_avail = c
        .photo_w
        .saturating_sub(2 * c.inner_pad + c.logo_slot + 2 * gap + left_w);
    let right = fit_fields(&right_fields, right_size, right_avail, c.sep);
    let right_joined = joined_text(&right, c.sep);

    let centered = c.theme.center_group;
    let x0 = c.ml + c.inner_pad;
    if centered {
        // The whole logo + two-column group centers as one unit.
        let wl = crate::render::measure_text(&camera, c.font);
        let wr = if right_joined.is_empty() {
            0
        } else {
            crate::render::measure_text(&right_joined, right.size_px)
        };
        let g1 = if has_logo && (wl > 0 || wr > 0) {
            gap
        } else {
            0
        };
        let g2 = if wl > 0 && wr > 0 { gap } else { 0 };
        let group_w = (if has_logo { c.logo_slot } else { 0 }) + g1 + wl + g2 + wr;
        let gx = c.canvas_w.saturating_sub(group_w) / 2;
        if has_logo {
            ops.push(DrawOp::Logo {
                brand: c.brand,
                x: gx,
                y: logo_y(c.band_y, c.bar_h, c.logo_h.max(1), c.theme.logo_y_offset),
                width: c.logo_slot,
                height: c.logo_h.max(1),
            });
        }
        let left_x = gx + (if has_logo { c.logo_slot + g1 } else { 0 });
        ops.push(DrawOp::Text {
            content: camera,
            size_px: c.font,
            color: c.theme.foreground,
            align: Align::Left,
            rect: (left_x, c.band_y, wl, c.bar_h),
        });
        if !right_joined.is_empty() {
            ops.push(DrawOp::Text {
                content: right_joined,
                size_px: right.size_px,
                color: c.theme.foreground,
                align: Align::Left,
                rect: (left_x + wl + g2, c.band_y, wr, c.bar_h),
            });
        }
        return;
    }

    if has_logo {
        ops.push(DrawOp::Logo {
            brand: c.brand,
            x: x0,
            y: logo_y(c.band_y, c.bar_h, c.logo_h.max(1), c.theme.logo_y_offset),
            width: c.logo_slot,
            height: c.logo_h.max(1),
        });
    }
    let left_x = x0 + c.logo_slot + gap;
    ops.push(DrawOp::Text {
        content: camera,
        size_px: c.font,
        color: c.theme.foreground,
        align: Align::Left,
        rect: (left_x, c.band_y, left_w, c.bar_h),
    });
    if !right_joined.is_empty() {
        // Right column is right-anchored at the photo's right padding line.
        let right_start = left_x + left_w + gap;
        let right_w = (c.ml + c.photo_w - c.inner_pad).saturating_sub(right_start);
        ops.push(DrawOp::Text {
            content: right_joined,
            size_px: right.size_px,
            color: c.theme.foreground,
            align: Align::Right,
            rect: (right_start, c.band_y, right_w, c.bar_h),
        });
    }
}

/// Each parameter on its own labeled row, stacked top-down inside the wide
/// left margin (Caption Card).
fn bar_stacked_labeled(ops: &mut Vec<DrawOp>, c: &BarCtx) {
    let has_logo = c.logo_slot > 0;
    let show_labels = c.theme.show_labels;
    let label_size = ((c.font as f64) * c.theme.label_ratio).round().max(6.0) as u32;
    let row_gap = (c.font as f64 * 0.35).round() as u32;
    let avail_w = c.ml.saturating_sub(2 * c.inner_pad);
    let avail_h = c.photo_h.saturating_sub(2 * c.inner_pad);
    let fg_dim = {
        let [r, g, b, _] = c.theme.foreground;
        [r, g, b, 150]
    };

    // Height budget per field: value row plus (when shown) the label row
    // above it, plus the gap. Drop fields until the stack fits.
    let block_h = |label_rows: bool| c.font + if label_rows { label_size } else { 0 };
    let mut parts = c.fields.parts.clone();
    let custom = c.fields.custom_line.clone();
    let label_rows = show_labels && custom.is_none();
    loop {
        let total = match &custom {
            Some(_) => c.font,
            None => {
                if parts.is_empty() {
                    0
                } else {
                    parts
                        .iter()
                        .map(|_| block_h(label_rows) + row_gap)
                        .sum::<u32>()
                        - row_gap
                }
            }
        };
        if total <= avail_h {
            break;
        }
        if custom.is_some() {
            break; // the custom line only ever shrinks, never drops
        }
        if let Some(victim) = crate::template::FIELD_DROP_ORDER
            .into_iter()
            .find(|f| parts.iter().any(|(pf, _)| pf == f))
        {
            parts.retain(|(f, _)| *f != victim);
        } else {
            break; // nothing left to drop: overflow and clip at worst
        }
    }

    let logo_slot = c.logo_slot.min(avail_w);
    let mut y = c.mt + c.inner_pad;
    if has_logo && c.logo_h.max(1) <= avail_h / 3 {
        ops.push(DrawOp::Logo {
            brand: c.brand,
            x: c.inner_pad,
            y,
            width: logo_slot,
            height: c.logo_h.max(1),
        });
        y += c.logo_h.max(1) + row_gap;
    }

    if let Some(text) = custom {
        let mut size = c.font;
        while size > 6 && estimate_width(&text, size) > avail_w {
            size -= 1;
        }
        ops.push(DrawOp::Text {
            content: text,
            size_px: size,
            color: c.theme.foreground,
            align: Align::Left,
            rect: (c.inner_pad, y, avail_w, size),
        });
        return;
    }

    for (field, value) in &parts {
        if value.is_empty() {
            continue;
        }
        if show_labels {
            ops.push(DrawOp::Text {
                content: field.label().to_string(),
                size_px: label_size,
                color: fg_dim,
                align: Align::Left,
                rect: (c.inner_pad, y, avail_w, label_size),
            });
            y += label_size;
        }
        let mut size = c.font;
        while size > 6 && estimate_width(value, size) > avail_w {
            size -= 1;
        }
        ops.push(DrawOp::Text {
            content: value.clone(),
            size_px: size,
            color: c.theme.foreground,
            align: Align::Left,
            rect: (c.inner_pad, y, avail_w, size),
        });
        y += size + row_gap;
    }
}

/// A fitted text selection.
struct Fitted {
    parts: Vec<(Field, String)>,
    custom_line: Option<String>,
    size_px: u32,
}

fn join(parts: &[(Field, String)], sep: &str) -> String {
    parts
        .iter()
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>()
        .join(sep)
}

fn joined_text(f: &Fitted, sep: &str) -> String {
    match &f.custom_line {
        Some(c) => c.clone(),
        None => join(&f.parts, sep),
    }
}

/// Fit the line into `avail_w`: shrink the font first (never below 60% of
/// nominal); only when still overflowing, drop fields in
/// `FIELD_DROP_ORDER` priority. The custom line never loses content — it
/// only shrinks.
fn fit_fields(fields: &Fields, font: u32, avail_w: u32, sep: &str) -> Fitted {
    let floor = ((font as f64 * 0.6).round() as u32).max(1);

    if let Some(custom) = fields.custom_line.clone() {
        let mut size = font;
        while size > floor && estimate_width(&custom, size) > avail_w {
            size -= 1;
        }
        return Fitted {
            parts: Vec::new(),
            custom_line: Some(custom),
            size_px: size,
        };
    }

    let mut parts = fields.parts.clone();
    let mut size = font;
    loop {
        if estimate_width(&join(&parts, sep), size) <= avail_w {
            return Fitted {
                parts,
                custom_line: None,
                size_px: size,
            };
        }
        if size > floor {
            size -= 1;
            continue;
        }
        // At the floor size and still overflowing: drop one field.
        if let Some(victim) = crate::template::FIELD_DROP_ORDER
            .into_iter()
            .find(|f| parts.iter().any(|(pf, _)| pf == f))
        {
            parts.retain(|(f, _)| *f != victim);
            continue;
        }
        // Nothing left to drop: render what remains (clipped at worst).
        return Fitted {
            parts,
            custom_line: None,
            size_px: size,
        };
    }
}

/// Cheap advance-width estimate for fit decisions (Inter-ish metrics).
pub(crate) fn estimate_width(text: &str, size_px: u32) -> u32 {
    let mut units = 0.0f64;
    for ch in text.chars() {
        units += match ch {
            ' ' => 0.30,
            '·' => 0.45,
            '.' | ':' | '/' | '-' => 0.36,
            'I' | 'l' | 'j' | '!' | '|' => 0.30,
            c if c.is_ascii_uppercase() => 0.68,
            _ => 0.58,
        };
    }
    (units * size_px as f64).ceil() as u32
}
