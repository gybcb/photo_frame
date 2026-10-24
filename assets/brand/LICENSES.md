# Brand logo assets

All SVGs in this directory were downloaded from Wikimedia Commons on
2026-10-24 and purified for use in this application. Purification steps:

1. Removed XML doctype, editor metadata, ids, and unused attributes.
2. Replaced fixed `width`/`height` with a normalized `viewBox` only.
3. Produced two variants per brand:
   - `{brand}.color.svg` — original colors preserved.
   - `{brand}.mono.svg` — a single placeholder fill `%FG%`, substituted at
     render time with the active theme foreground color.

Brand-specific notes:

- **nikon.mono.svg** — the Commons source (`File:Nikon Logo.svg`) is the
  corporate emblem: a yellow square with the NIKON wordmark band at the
  bottom. The color variant keeps the official emblem intact; the mono
  variant extracts only the wordmark path and crops the viewBox to its
  painted bounds (a full-canvas solid fill would be unusable as a logo).
- **phaseone.svg** — the "1" interlocked between "N" and "E" is part of the
  source vector and preserved.
- **alpa** — a Commons source exists (`File:Alpa logo.svg`); no redrawing
  was needed. The source's `viewBox` clipped the artwork to roughly the
  left 60% (an Inkscape leftover), so it was corrected to the true content
  bounds (`-8 -8 961 293`), which reveals the full "ALPA® / OF SWITZERLAND"
  lockup.
- **leica.mono.svg** — a two-layer variant generated from the color source:
  the disc takes `%FG%` and the wordmark takes `%BG%`, so on any theme the
  logo reads as a solid disc with knocked-out letters.

## Sources and license

| File stem | Source | Commons license tag |
|---|---|---|
| sony | https://commons.wikimedia.org/wiki/File:Sony_logo.svg | Public domain (PD-textlogo) |
| nikon | https://commons.wikimedia.org/wiki/File:Nikon_Logo.svg | Public domain (below threshold of originality, US Copyright Office ruling) |
| canon | https://commons.wikimedia.org/wiki/File:Canon_logo.svg | Public domain (PD-textlogo / Japan PD-textlogo) |
| leica | https://commons.wikimedia.org/wiki/File:Leica_Camera.svg | Public domain (PD-textlogo) |
| hasselblad | https://commons.wikimedia.org/wiki/File:Hasselblad_logo.svg | Public domain (PD-textlogo) |
| phaseone | https://commons.wikimedia.org/wiki/File:Phase_One_logo.svg | Public domain (PD-textlogo) |
| alpa | https://commons.wikimedia.org/wiki/File:Alpa_logo.svg | Public domain |

Copyright status per Wikimedia Commons: all of the above are text wordmarks
judged below the threshold of copyright originality.

**Trademark notice:** the marks remain registered trademarks of Sony Group
Corporation, Nikon Corporation, Canon Inc., Leica Camera AG, Victor Hasselblad
AB, Phase One A/S, and Alpa SA respectively. They are displayed here solely to
identify the camera brand recorded in a photograph's EXIF data (nominative /
editorial use).
