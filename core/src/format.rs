//! Text formatting for the shooting-parameters shown on the frame.

/// Formats an exposure time in seconds the way photographers write it:
/// `1/250` under a second, `2"` at or above it.
pub fn shutter(seconds: f64) -> String {
    if seconds <= 0.0 {
        return String::from("0\"");
    }
    if seconds < 1.0 {
        let denom = (1.0 / seconds).round().max(1.0) as i64;
        format!("1/{denom}")
    } else {
        // Keep at most one decimal place: 2.0 -> 2", 1.5 -> 1.5".
        let rounded = (seconds * 10.0).round() / 10.0;
        if (rounded - rounded.round()).abs() < f64::EPSILON {
            format!("{}\"", rounded.round() as i64)
        } else {
            format!("{:.1}\"", rounded)
        }
    }
}

/// Standard f-stops, used to snap the imprecise rationals EXIF stores
/// (e.g. 2.8125 back to f/2.8, 8.0000001 back to f/8).
const STANDARD_STOPS: [f64; 30] = [
    0.7, 0.8, 1.0, 1.1, 1.2, 1.4, 1.6, 1.8, 2.0, 2.2, 2.5, 2.8, 3.2, 3.5, 4.0, 4.5, 5.0, 5.6, 6.3,
    7.1, 8.0, 9.0, 10.0, 11.0, 13.0, 14.0, 16.0, 18.0, 20.0, 22.0,
];

/// Formats an f-number as `f/2.8` (whole stops drop the decimal).
pub fn aperture(fnum: f64) -> String {
    if !fnum.is_finite() || fnum <= 0.0 {
        return String::new();
    }
    let snapped = STANDARD_STOPS
        .iter()
        .filter(|stop| ((**stop - fnum).abs() / **stop) < 0.02)
        .min_by(|a, b| (**a - fnum).abs().total_cmp(&(**b - fnum).abs()))
        .copied()
        .unwrap_or((fnum * 10.0).round() / 10.0);
    let rounded = (snapped * 10.0).round() / 10.0;
    if (rounded - rounded.round()).abs() < 1e-6 {
        format!("f/{}", rounded.round() as i64)
    } else {
        format!("f/{rounded}")
    }
}

/// Formats an ISO speed as `ISO100`.
pub fn iso(speed: u32) -> String {
    format!("ISO{speed}")
}

/// Formats a focal length as `85mm` (whole numbers drop the decimal).
pub fn focal(mm: f64) -> String {
    format!("{}mm", focal_number(mm))
}

/// Formats a zoom range as `24-70mm`.
pub fn focal_range(min: f64, max: f64) -> String {
    format!("{}-{}mm", focal_number(min), focal_number(max))
}

/// Renders a number without a trailing `.0` and with at most one decimal.
fn focal_number(mm: f64) -> String {
    let rounded = (mm * 10.0).round() / 10.0;
    if (rounded - rounded.round()).abs() < f64::EPSILON {
        format!("{}", rounded.round() as i64)
    } else {
        format!("{rounded:.1}")
    }
}

/// Normalizes a lens name from EXIF for display: collapses whitespace,
/// drops trailing `(Model …)` annotations and wrapping quotes, and strips
/// `.0` artifacts from mm values (Sony writes `24.0-70.0 mm`).
pub fn clean_lens_name(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");

    if s.ends_with(')')
        && let Some(idx) = s.rfind("(Model ")
    {
        s = s[..idx].trim_end().to_string();
    }

    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s = s[1..s.len() - 1].trim().to_string();
    }

    strip_trailing_zeros(&s)
}

/// Removes `.0` from decimal numbers (only when the `0` ends the number,
/// so `f/2.8` and `100-400` are untouched).
fn strip_trailing_zeros(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let is_dot_zero = c == '.' && i + 1 < bytes.len() && bytes[i + 1] == b'0';
        let ends_number = is_dot_zero && i > 0 && bytes[i - 1].is_ascii_digit();
        let next_is_digit = is_dot_zero && i + 2 < bytes.len() && bytes[i + 2].is_ascii_digit();
        if ends_number && !next_is_digit {
            // Skip ".0" unless another '.' follows (defensive).
            if i + 2 >= bytes.len() || bytes[i + 2] != b'.' {
                i += 2;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}
