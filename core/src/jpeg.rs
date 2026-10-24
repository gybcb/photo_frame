//! JPEG decoding and framed export with byte-exact EXIF carry-over.
//!
//! Export deliberately does *not* re-serialize EXIF: rewriting the TIFF
//! block corrupts MakerNote internal offsets (a known failure class in
//! every Rust EXIF writer). Instead the original APP1 segment's payload
//! is spliced, unmodified, right after the SOI of the freshly encoded
//! JPEG. The known cost: `PixelXDimension`/`PixelYDimension` keep the
//! original photo size while the framed canvas is larger — harmless for
//! viewers, possibly flagged by strict validators.

use std::io::Cursor;

use exif::{In, Reader, Tag};
use image::{ImageFormat, ImageReader};

use crate::assets::LogoMode;
use crate::error::FrameError;
use crate::layout::FrameLayout;
use crate::render;

/// A decoded photo plus the framing-relevant EXIF facts.
pub struct Decoded {
    pub photo: image::RgbaImage,
    /// The original Exif APP1 payload (after `Exif\0\0`), empty if none.
    pub exif: Vec<u8>,
    /// EXIF orientation already applied to `photo`.
    pub orientation: u16,
}

/// Decode bytes into an RGBA photo (EXIF orientation applied) plus the
/// original Exif APP1 payload (empty when the file has none).
///
/// Missing EXIF is not an error: orientation defaults to 1.
pub fn load_bytes(bytes: &[u8]) -> Result<(image::RgbaImage, Vec<u8>), FrameError> {
    let d = decode(bytes)?;
    Ok((d.photo, d.exif))
}

/// Decode with all framing-relevant facts.
pub fn decode(bytes: &[u8]) -> Result<Decoded, FrameError> {
    let fmt = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| FrameError::Decode(e.to_string()))?
        .format()
        .ok_or(FrameError::UnsupportedFormat)?;
    if !matches!(
        fmt,
        ImageFormat::Jpeg | ImageFormat::Tiff | ImageFormat::Png
    ) {
        return Err(FrameError::UnsupportedFormat);
    }
    let img = ImageReader::with_format(Cursor::new(bytes), fmt)
        .decode()
        .map_err(|e| FrameError::Decode(e.to_string()))?;
    let orientation = exif_orientation(bytes);
    let photo = rotate_to_orientation(img.to_rgba8(), orientation);
    Ok(Decoded {
        photo,
        exif: find_app1(bytes).unwrap_or(&[]).to_vec(),
        orientation,
    })
}

/// EXIF orientation tag (0x0112), defaulting to 1 when absent/unreadable.
fn exif_orientation(bytes: &[u8]) -> u16 {
    let Ok(exif) = Reader::new().read_from_container(&mut Cursor::new(bytes)) else {
        return 1;
    };
    exif.get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|f| match &f.value {
            exif::Value::Short(vs) => vs.first().copied(),
            _ => None,
        })
        .unwrap_or(1)
}

/// Apply an EXIF orientation by rotating/transforming the pixel data,
/// so downstream framing sees the photo as it is meant to be displayed.
fn rotate_to_orientation(img: image::RgbaImage, orientation: u16) -> image::RgbaImage {
    use image::imageops;
    match orientation {
        2 => image::imageops::flip_horizontal(&img),
        3 => imageops::rotate180(&img),
        4 => imageops::flip_vertical(&img),
        5 => imageops::flip_horizontal(&imageops::rotate90(&img)),
        6 => imageops::rotate90(&img),
        7 => imageops::flip_horizontal(&imageops::rotate270(&img)),
        8 => imageops::rotate270(&img),
        _ => img,
    }
}

/// Locate the raw Exif APP1 payload (the bytes after `Exif\0\0`) in a JPEG
/// by scanning marker segments from SOI. Returns None for non-JPEG or
/// EXIF-less input.
pub fn find_app1(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 4 || bytes[..2] != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xFF {
            return None;
        }
        let marker = bytes[i + 1];
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if len < 2 || i + 2 + len > bytes.len() {
            return None;
        }
        if marker == 0xE1 {
            let seg = &bytes[i + 4..i + 2 + len];
            return seg.strip_prefix(b"Exif\0\0");
        }
        if marker == 0xDA {
            return None; // start of scan: no APP1 present
        }
        i += 2 + len;
    }
    None
}

/// Frame an already-decoded photo and encode it as JPEG, carrying the
/// original EXIF payload over byte-for-byte when `original_exif` has one.
pub fn frame_and_export(
    photo: &image::RgbaImage,
    layout: &FrameLayout,
    logo_mode: LogoMode,
    original_exif: Option<&[u8]>,
    quality: u8,
) -> Result<Vec<u8>, FrameError> {
    let canvas = render::compose(photo, layout, logo_mode)?;
    let mut buf = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    // JPEG has no alpha; the canvas is opaque anyway (bg + opaque photo).
    let rgb = image::DynamicImage::ImageRgba8(canvas).into_rgb8();
    encoder
        .encode(
            &rgb,
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| FrameError::Encode(e.to_string()))?;
    Ok(splice_app1(buf, original_exif))
}

/// One-shot pipeline: decode `bytes` (applying orientation), frame with
/// `layout`, export as JPEG with the original EXIF carried over.
pub fn frame_bytes(bytes: &[u8], layout: FrameLayout, quality: u8) -> Result<Vec<u8>, FrameError> {
    let (photo, _) = load_bytes(bytes)?;
    frame_and_export(&photo, &layout, LogoMode::Mono, find_app1(bytes), quality)
}

/// Insert the Exif APP1 segment right after SOI, unless the encoder
/// already emitted one (it never does today, but check rather than assume).
fn splice_app1(encoded: Vec<u8>, exif_payload: Option<&[u8]>) -> Vec<u8> {
    // JPEG marker length is a u16 counting itself, so a segment body
    // (2 length bytes + 6 header + payload) must fit in 65535 - 2 bytes.
    const MAX_PAYLOAD: usize = 65535 - 2 - 6;
    let Some(payload) = exif_payload.filter(|p| !p.is_empty() && p.len() <= MAX_PAYLOAD) else {
        return encoded;
    };
    if find_app1(&encoded).is_some() {
        return encoded;
    }
    let payload = normalize_orientation(payload);
    // segment: FFE1 <len> "Exif\0\0" <payload>; len covers itself + body.
    let body_len = 6 + payload.len() + 2;
    let mut seg = Vec::with_capacity(body_len + 2);
    seg.extend_from_slice(&[0xFF, 0xE1]);
    seg.extend_from_slice(&(body_len as u16).to_be_bytes());
    seg.extend_from_slice(b"Exif\0\0");
    seg.extend_from_slice(&payload);
    let mut out = Vec::with_capacity(encoded.len() + seg.len());
    out.extend_from_slice(&encoded[..2]); // SOI
    out.extend_from_slice(&seg);
    out.extend_from_slice(&encoded[2..]);
    out
}

/// Rewrite *only* the Orientation (0x0112) tag's value to 1 (Normal).
///
/// We always bake the orientation into pixels at decode, so a carried
/// tag of 6 would make viewers rotate the finished frame a second time.
/// Re-serializing EXIF is off the table (MakerNote offsets), so instead
/// we locate the inline SHORT value inside IFD0 and patch those 2 bytes;
/// every other byte — MakerNote, all offsets, all other tags — stays
/// untouched. Best-effort: anything malformed returns the input as-is
/// (a stale orientation tag is cosmetic; corrupting the block is not).
fn normalize_orientation(payload: &[u8]) -> Vec<u8> {
    fn u16_at(b: &[u8], at: usize, le: bool) -> Option<usize> {
        let v = b.get(at..at + 2)?.try_into().ok()?;
        Some(if le {
            u16::from_le_bytes(v)
        } else {
            u16::from_be_bytes(v)
        } as usize)
    }
    fn u32_at(b: &[u8], at: usize, le: bool) -> Option<usize> {
        let v = b.get(at..at + 4)?.try_into().ok()?;
        Some(if le {
            u32::from_le_bytes(v)
        } else {
            u32::from_be_bytes(v)
        } as usize)
    }

    let le = match payload.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return payload.to_vec(),
    };
    let Some(ifd0) = u32_at(payload, 4, le) else {
        return payload.to_vec();
    };
    let Some(count) = u16_at(payload, ifd0, le) else {
        return payload.to_vec();
    };
    for k in 0..count {
        let entry = ifd0 + 2 + k * 12;
        if u16_at(payload, entry, le) != Some(0x0112) {
            continue; // reads are bounds-checked, so a truncated
            // table simply never matches
        }
        // Patch only the canonical shape: 1 SHORT stored inline in the
        // 4-byte value field (type 3, count 1). Anything else — absent,
        // another type — we leave alone rather than restructure.
        let typ = u16_at(payload, entry + 2, le);
        let n = u32_at(payload, entry + 4, le);
        if typ != Some(3) || n != Some(1) {
            return payload.to_vec();
        }
        let mut out = payload.to_vec();
        let vs = if le {
            1u16.to_le_bytes()
        } else {
            1u16.to_be_bytes()
        };
        out[entry + 8..entry + 10].copy_from_slice(&vs);
        return out;
    }
    payload.to_vec() // no orientation tag: nothing to do
}
