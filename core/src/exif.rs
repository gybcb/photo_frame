//! EXIF extraction: raw bytes in, display-ready fields out.
//!
//! Everything here is best-effort and non-fatal: a photo without EXIF, or
//! with only some tags, still yields a usable `PhotoInfo` (empty fields,
//! `Brand::Unknown`). The GUI then lets the user fill gaps by hand.

use std::io::Cursor;

use exif::{Context, In, Reader, Tag, Value};

use crate::brand::Brand;
use crate::format;
use crate::template::{Field, Fields};

/// Everything the framer needs to know about one photo.
#[derive(Clone, Debug)]
pub struct PhotoInfo {
    pub make: String,
    pub model: String,
    pub brand: Brand,
    /// EXIF orientation (1..=8), already applied to any decoded pixels.
    pub orientation: u16,
    /// Ordered, formatted parameter fields for the frame text.
    pub fields: Fields,
    /// Raw Exif APP1 payload to carry into the export (empty if none).
    pub exif: Vec<u8>,
}

/// Field order shown on the frame (camera body first, exposure group after).
const ORDER: [Field; 7] = [
    Field::Camera,
    Field::Lens,
    Field::Focal,
    Field::Aperture,
    Field::Shutter,
    Field::Iso,
    Field::Date,
];

impl PhotoInfo {
    /// Parse EXIF from image bytes (JPEG/TIFF/PNG). Never fails.
    pub fn from_bytes(bytes: &[u8]) -> PhotoInfo {
        let parsed = Reader::new()
            .read_from_container(&mut Cursor::new(bytes))
            .ok();

        let get = |tag: Tag| parsed.as_ref().and_then(|e| e.get_field(tag, In::PRIMARY));
        let ascii = |tag: Tag| -> String {
            get(tag)
                .and_then(|f| match &f.value {
                    Value::Ascii(parts) => parts
                        .first()
                        .map(|p| String::from_utf8_lossy(p).trim_end().to_string()),
                    _ => None,
                })
                .unwrap_or_default()
        };
        let short = |tag: Tag| -> Option<u32> {
            get(tag).and_then(|f| match &f.value {
                Value::Short(vs) => vs.first().map(|v| *v as u32),
                Value::Long(vs) => vs.first().copied(),
                _ => None,
            })
        };
        let rat = |tag: Tag| -> Option<f64> {
            get(tag).and_then(|f| match &f.value {
                Value::Rational(rs) => rs.first().map(|r| r.num as f64 / r.denom as f64),
                _ => None,
            })
        };
        let strat = |tag: Tag| -> Option<f64> {
            get(tag).and_then(|f| match &f.value {
                Value::SRational(rs) => rs.first().map(|r| r.num as f64 / r.denom as f64),
                _ => None,
            })
        };

        let make = ascii(Tag(Context::Tiff, 0x010F));
        let model = ascii(Tag(Context::Tiff, 0x0110));
        let orientation = short(Tag(Context::Tiff, 0x0112)).unwrap_or(1) as u16;

        let mut parts: Vec<(Field, String)> = Vec::new();

        // Body model as its own field: the camera that took the shot (the Lens
        // field may show a lens name instead). Trim to keep the joined line clean.
        let model_trim = model.trim().to_string();
        if !model_trim.is_empty() {
            parts.push((Field::Camera, model_trim));
        }

        // Lens: prefer the explicit lens name, fall back to the body model.
        let lens_raw = ascii(Tag(Context::Exif, 0xA434));
        let lens = format::clean_lens_name(if lens_raw.is_empty() {
            model.as_str()
        } else {
            &lens_raw
        });
        if !lens.is_empty() {
            parts.push((Field::Lens, lens));
        }

        // Focal length: prefer the 35mm-equivalent when present.
        let focal35 = rat(Tag(Context::Exif, 0xA405));
        let focal_actual = rat(Tag(Context::Exif, 0x920A));
        if let Some(f) = focal35.or(focal_actual) {
            parts.push((Field::Focal, format::focal(f)));
        }

        // Aperture: FNumber, else derived from the APEX ApertureValue.
        if let Some(fnum) = rat(Tag(Context::Exif, 0x829D))
            .or_else(|| strat(Tag(Context::Exif, 0x9202)).map(|apex| 2f64.powf(apex / 2.0)))
        {
            let text = format::aperture(fnum);
            if !text.is_empty() {
                parts.push((Field::Aperture, text));
            }
        }

        // Shutter: ExposureTime, else derived from the APEX ShutterSpeedValue.
        if let Some(sec) = rat(Tag(Context::Exif, 0x8824))
            .or_else(|| strat(Tag(Context::Exif, 0x9201)).map(|apex| 2f64.powf(-apex)))
        {
            parts.push((Field::Shutter, format::shutter(sec)));
        }

        if let Some(iso) = short(Tag(Context::Exif, 0x8827)) {
            parts.push((Field::Iso, format::iso(iso)));
        }

        // Date: DateTimeOriginal (0x9003) preferred over DateTime (0x0132).
        let dt = ascii_tag(Tag(Context::Exif, 0x9003), &parsed)
            .or_else(|| ascii_tag(Tag(Context::Tiff, 0x0132), &parsed));
        if let Some(dt) = dt
            && let Some(d) = exif_date_to_iso(&dt)
        {
            parts.push((Field::Date, d));
        }

        // Keep the canonical display order.
        parts.sort_by_key(|(f, _)| ORDER.iter().position(|x| x == f).unwrap_or(99));

        PhotoInfo {
            brand: Brand::resolve(&make, &model),
            fields: Fields {
                parts,
                custom_line: None,
            },
            exif: crate::jpeg::find_app1(bytes).unwrap_or_default().to_vec(),
            make,
            model,
            orientation,
        }
    }
}

/// ASCII helper for tags not in the local `ascii` closure's scope.
fn ascii_tag(tag: Tag, parsed: &Option<exif::Exif>) -> Option<String> {
    parsed
        .as_ref()?
        .get_field(tag, In::PRIMARY)
        .and_then(|f| match &f.value {
            Value::Ascii(parts) => parts
                .first()
                .map(|p| String::from_utf8_lossy(p).trim_end().to_string()),
            _ => None,
        })
}

/// `YYYY:MM:DD HH:MM:SS` → `YYYY-MM-DD`, tolerating a shorter value.
fn exif_date_to_iso(raw: &str) -> Option<String> {
    if raw.len() < 10 {
        return None;
    }
    Some(format!("{}-{}-{}", &raw[0..4], &raw[5..7], &raw[8..10]))
}
