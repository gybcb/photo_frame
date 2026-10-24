# Framely

[English](README.md) | [简体中文](README.zh-CN.md)

![CI](https://github.com/gybcb/photo_frame/actions/workflows/ci.yml/badge.svg)

A desktop app that frames photos with their camera brand — Rust, eframe +
egui. Reads EXIF, detects the brand, draws its vector logo and fills in
lens / focal length / aperture / shutter / ISO; missing fields can be
typed in by hand. Live preview for one photo, parallel export for many.

## Build and run

```sh
cargo run --release -p framely-app  # run the GUI (this is the daily command)
cargo test --workspace              # all tests (core logic + batch export)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Toolchain: Rust 1.98+ (edition 2024). The code targets macOS, Windows and
Linux — GUI, file dialogs, font probing and config paths are all
platform-aware and unit-tested per platform; CI builds and tests on all
three. Interactive use has only been verified on macOS so far. No
packaging artifacts (dmg/msi/AppImage) yet.

> Always run with `--release`. The compositor is per-pixel CPU work and
> debug builds are an order of magnitude slower (~145 ms vs ~1.4 ms per
> 3000 px frame); previews will stutter otherwise. Debug is for
> development only.

## Usage

- **Open photos** — drag into the window, or use the toolbar button
  (`rfd` dialog); multiple selection feeds the batch list.
- **Theme panel (left)** — 4 templates (White Studio / Black Bar /
  Minimal / Magazine). Border width, strip height, font size,
  foreground/background colors, field toggles and order, mono vs.
  official-color logo — all adjustable, and save/load-able as a JSON
  preset (dialogs default to your home folder).
- **Fields panel (right)** — each field shows its EXIF value; tick
  *manual* to replace it with custom text. A whole-line custom text
  field covers bodies like Alpa that write no lens name. Manual values
  affect rendering only; exported EXIF is untouched.
- **Batch export (bottom)** — pick a folder → the same pipeline runs
  over every file in parallel → progress bar plus a per-file error
  list. One broken file is recorded, never fatal to the batch.
- **Output naming** — `<original>_framed.jpg`, with `_2`, `_3`… suffixes
  on collision.

Preview and export share one `render::compose` pipeline and differ only
in canvas scale — that is what makes WYSIWYG real here.

## Supported brands

EXIF `Make` prefix match: Sony, Nikon, Canon, Leica, Hasselblad,
Phase One, Alpa. An unmatched `Unknown` is not an error: the photo gets
the frame without a logo, text only.

Logos are vector SVGs rasterized at the exact target pixel size (not
upscaled bitmaps), so they stay sharp at any export size. Each brand
ships two variants: `.color.svg` (official colors) and `.mono.svg`
(the theme's foreground color is injected at render time).

> On the dark Black Bar theme, "official color" makes most wordmarks
> near-invisible (they are black type). The theme defaults to mono
> (white) on purpose.

## Huge images (stitched panoramas)

Multi-frame stitches can reach hundreds of megapixels. The resident
preview photo is downscaled to ≤ 2048 px on the long edge before
rendering (borders are proportional, the logo is vector, text is
rasterized at target size — a small canvas looks identical and text is
actually sharper); export re-decodes at full resolution on demand and
frees the photo buffer before encoding. Batch export runs 2 files at a
time. Measured on a ~286 MP two-frame Phase One stitch: ~2.3 GB peak to
open and preview, ~3.8 GB transient during export (previously: an outright
"Memory limit exceeded", or 5.4 GB).

## EXIF policy (important)

Exported JPEGs carry the original file's Exif APP1 segment **byte for
byte**, spliced in right after SOI — nothing is re-written. Rationale:
every Rust EXIF *writer* has a known failure mode of breaking absolute
offsets inside MakerNotes (the kamadak-exif author documents this risk
himself). Two deliberate exceptions/costs:

1. **Orientation is rewritten to 1** — pixels are already rotated during
   decode, so keeping e.g. `6` would make viewers rotate the output
   again. Only the inline SHORT value of tag `0x0112` changes — those
   2 bytes; MakerNotes, all offsets and every other tag are untouched.
   Files with no orientation tag, or with `1`, come out byte-identical.
2. **PixelXDimension / PixelYDimension still report the unframed size**
   and now disagree with the larger canvas. Mainstream viewers don't
   care; `exiftool -validate` may warn.

Inputs without EXIF export as plain JPEGs — not an error.

Logo sources and licensing (Wikimedia Commons, PD-textlogo + a
Trademarked notice; the purification steps) are documented in
`assets/brand/LICENSES.md`. The Inter font (OFL) is embedded in the
binary — outputs never depend on system fonts.

## Language

- UI in English / Chinese, toggled from a toolbar button. The choice is
  persisted per platform: macOS `~/.framely/config.json` (historical
  location, kept so upgrades don't lose it), Linux
  `$XDG_CONFIG_HOME/framely/config.json` (falls back to
  `~/.config/framely/`), Windows `%APPDATA%\framely\config.json`.
- First launch follows the system `LANG` (`zh*` → Chinese). Windows has
  no `LANG`; it starts in English, toggle freely.
- No CJK font is embedded. A system Chinese font is probed at runtime —
  macOS: PingFang / Hiragino Sans GB / STHeiti; Windows: Microsoft YaHei
  (`%WINDIR%\Fonts\msyh.ttc`) → SimHei → SimSun; Linux: Noto Sans CJK
  (common Debian/Arch/Fedora locations) → WenQuanYi Micro Hei / Zen Hei.
  If nothing is found the UI stays English-only and the toggle hides
  itself.
- The CJK face is appended last (Latin keeps the default fonts and
  metrics) and scaled to 0.84: Han glyphs fill the em box while Latin
  reaches only cap height, so equal sizes read unevenly — and Hiragino
  class fonts carry `hhea.lineGap = 0.5em`, whose per-glyph vertical
  centering in egui would push Chinese out of its row and clip it. The
  scale fixes both.
- Text *inside the frame* (template names, parameters, logo) is always
  English, independent of UI language.

## Project layout

```
core/   framely-core — all logic, zero GUI deps
  exif.rs      EXIF extraction → display fields
  brand.rs     Make/Model → brand matching
  format.rs    shutter/aperture/ISO/focal/lens-name formatting
               (pure functions, fully unit-tested)
  template.rs  templates + tunable theme (serde; presets are JSON)
  layout.rs    proportional layout → Vec<DrawOp>
  render.rs    SVG rasterization + glyphs + compositing
               (shared by preview and export)
  jpeg.rs      decode (orientation-corrected) / export with original APP1
  assets.rs    embedded logos and fonts
app/    framely-app — the eframe shell (panels, preview, batch)
```

## Out of scope (v1)

RAW decoding, PNG/TIFF output, external template description files,
installers/packaging, any image editing (crop, color adjustments).

## Development tools

`cargo run -p framely-app --example e2e` exports test images through the
real pipeline into `/tmp/framely-e2e/` for eyeballing and EXIF diffs
(needs `exiftool`, or Python Pillow; a throwaway debug driver, not a
product feature).
