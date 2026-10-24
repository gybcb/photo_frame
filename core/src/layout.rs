//! Geometry for the templates: turns a photo size + theme into a list of
//! drawing operations, every dimension derived from the canvas scale.

use crate::brand::Brand;
use crate::template::{Field, Fields, TemplateId, ThemeConfig};

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
    pub ops: Vec<DrawOp>,
}

pub(crate) const SEP: &str = " · ";

/// Logo top position inside the bar: vertically centered, nudged by the
/// theme's fraction of the bar height, clamped so the logo never leaves
/// the bar at extreme offsets.
fn logo_y(bar_y: u32, bar: u32, h: u32, offset: f64) -> u32 {
    let slack = bar.saturating_sub(h);
    let nudge = (bar as f64 * offset).round() as i64;
    ((slack / 2) as i64 + nudge).clamp(0, slack as i64) as u32 + bar_y
}

/// Lays out `tpl` around a `photo_w x photo_h` image. The parameter bar sits
/// below the photo, inside the border. `logo_ratio` is the brand logo's
/// width/height aspect, used to size the logo box without distorting it.
pub fn layout(
    tpl: TemplateId,
    photo_w: u32,
    photo_h: u32,
    brand: Brand,
    logo_ratio: f64,
    theme: &ThemeConfig,
    fields: &Fields,
) -> FrameLayout {
    let short = photo_w.min(photo_h) as f64;
    let border = (short * theme.border_ratio).round().max(1.0) as u32;
    let bar = (short * theme.bar_ratio).round().max(12.0) as u32;
    let font = (bar as f64 * theme.font_scale).round().max(9.0) as u32;
    let inner_pad = (bar as f64 * 0.25).round() as u32;

    let logo_h = (bar as f64 * theme.logo_scale).round() as u32;
    let logo_slot = if brand == Brand::Unknown || theme.logo_scale <= 0.0 {
        0
    } else {
        ((logo_h as f64) * logo_ratio).round().max(1.0) as u32
    };

    let has_logo = logo_slot > 0;
    let two_lines = matches!(tpl, TemplateId::Magazine)
        && fields.custom_line.is_none()
        && fields.parts.len() > 1;

    // Fit the text: drop fields by priority, then shrink the font, until it
    // fits the region left of (or around) the logo.
    let gap = if has_logo { font } else { 0 };
    let text_region_w = photo_w.saturating_sub(2 * inner_pad + logo_slot + gap);
    let fitted = fit_fields(fields, font, text_region_w);

    let canvas_w = photo_w + 2 * border;
    let canvas_h = photo_h + 2 * border + bar;
    let bar_y = border + photo_h;
    let region_x = border + inner_pad + logo_slot + gap;

    let mut ops = Vec::new();

    // Optically-centered variant: measure the real rendered width so the
    // group's left and right canvas margins match to the pixel; the fit
    // decisions above stay on the cheap estimate.
    let centered = theme.center_group;
    let joined = joined_text(&fitted);
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
            (if has_logo { logo_slot } else { 0 }) + (if both { gap } else { 0 }) + text_w;
        canvas_w.saturating_sub(group_w) / 2
    } else {
        0
    };

    match tpl {
        TemplateId::WhiteStudio | TemplateId::BlackBar | TemplateId::Minimal => {
            if has_logo {
                let h = logo_h.max(1);
                ops.push(DrawOp::Logo {
                    brand,
                    x: if centered {
                        group_x
                    } else {
                        border + inner_pad
                    },
                    y: logo_y(bar_y, bar, h, theme.logo_y_offset),
                    width: logo_slot,
                    height: h,
                });
            }
            if !joined.is_empty() {
                ops.push(DrawOp::Text {
                    content: joined,
                    size_px: fitted.size_px,
                    color: theme.foreground,
                    align: if centered { Align::Left } else { Align::Right },
                    rect: if centered {
                        (
                            group_x + if has_logo { logo_slot + gap } else { 0 },
                            bar_y,
                            text_w,
                            bar,
                        )
                    } else {
                        (region_x, bar_y, text_region_w, bar)
                    },
                });
            }
        }
        TemplateId::Magazine => {
            // One logo, vertically centered in the bar; parameter text as
            // two right-anchored lines to its right.
            if has_logo {
                ops.push(DrawOp::Logo {
                    brand,
                    x: if centered {
                        group_x
                    } else {
                        border + inner_pad
                    },
                    y: logo_y(bar_y, bar, logo_h.max(1), theme.logo_y_offset),
                    width: logo_slot,
                    height: logo_h.max(1),
                });
            }
            let lines: Vec<String> = if two_lines {
                let mid = fields.parts.len().div_ceil(2);
                vec![join(&fields.parts[..mid]), join(&fields.parts[mid..])]
            } else {
                vec![joined_text(&fitted)]
            };
            let line_h = (bar / lines.len().max(1) as u32).max(1);
            for (i, content) in lines.iter().enumerate() {
                if content.is_empty() {
                    continue;
                }
                // Centered: each line right-aligns inside the measured
                // text block (no single line is wider than the joined
                // string), so the group's margins stay equal.
                let (rx, rw) = if centered {
                    (group_x + if has_logo { logo_slot + gap } else { 0 }, text_w)
                } else {
                    (region_x, text_region_w)
                };
                ops.push(DrawOp::Text {
                    content: content.clone(),
                    size_px: fitted.size_px.min((line_h as f64 * 0.8) as u32),
                    color: theme.foreground,
                    align: Align::Right,
                    rect: (rx, bar_y + i as u32 * line_h, rw, line_h),
                });
            }
        }
    }

    FrameLayout {
        canvas_width: canvas_w,
        canvas_height: canvas_h,
        border,
        bar_height: bar,
        photo_rect: (border, border, photo_w, photo_h),
        background: theme.background,
        ops,
    }
}

/// A fitted text selection.
struct Fitted {
    parts: Vec<(Field, String)>,
    custom_line: Option<String>,
    size_px: u32,
}

fn join(parts: &[(Field, String)]) -> String {
    parts
        .iter()
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>()
        .join(SEP)
}

fn joined_text(f: &Fitted) -> String {
    match &f.custom_line {
        Some(c) => c.clone(),
        None => join(&f.parts),
    }
}

/// Fit the line into `avail_w`: shrink the font first (never below 60% of
/// nominal); only when still overflowing, drop fields in
/// `FIELD_DROP_ORDER` priority. The custom line never loses content — it
/// only shrinks.
fn fit_fields(fields: &Fields, font: u32, avail_w: u32) -> Fitted {
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
        if estimate_width(&join(&parts), size) <= avail_w {
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
