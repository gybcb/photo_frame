#!/usr/bin/env python3 -I
"""Generate JPEG fixtures with known EXIF for framely-core tests.

Run:  python3 -I core/tests/fixtures/gen.py
Produces (in this directory):
  sony.jpg          120x80, Make=SONY Model=ILCE-7RM4 + lens/fnum/iso/focal
  nikon_rot6.jpg    120x80 with Orientation=6 (portrait shot, rotated storage)
  plain.jpg         120x80, no EXIF
"""

import io

from PIL import Image
from PIL.ExifTags import IFD
from PIL.TiffImagePlugin import IFDRational

HERE = __file__.rsplit("/", 1)[0]


def base_exif(make, model, dt):
    im = Image.new("RGB", (8, 8))
    ex = im.getexif()
    ex[0x010F] = make      # Make
    ex[0x0110] = model     # Model
    ex[0x0132] = dt        # DateTime
    return ex


def exif_sub(ex):
    """Write Exif-sub-IFD tags: DateTimeOriginal, lens, fnum, shutter, iso, focal."""
    e = ex.get_ifd(IFD.Exif)
    return e


def save(name, im, exif=None):
    kwargs = {"quality": 95}
    if exif is not None:
        kwargs["exif"] = exif.tobytes()
    im.save(f"{HERE}/{name}", "JPEG", **kwargs)
    print("wrote", name)


def main():
    ex = base_exif("SONY", "ILCE-7RM4", "2026:10:24 12:00:00")
    e = exif_sub(ex)
    e[0x9003] = "2026:10:24 12:00:00"   # DateTimeOriginal
    e[0xA434] = "FE 35mm F1.8"          # LensModel
    e[0x829D] = IFDRational(1.8)        # FNumber
    e[0x8833] = IFDRational(1 / 250)    # ExposureTime
    e[0x8827] = 100                     # ISOSpeedRatings
    e[0x920A] = IFDRational(35)         # FocalLength
    save("sony.jpg", Image.new("RGB", (120, 80), (200, 100, 50)), ex)

    ex6 = base_exif("NIKON CORPORATION", "NIKON Z 6", "2026:10:24 13:00:00")
    e6 = exif_sub(ex6)
    e6[0x9003] = "2026:10:24 13:00:00"
    e6[0xA434] = "24.0-70.0 mm f/4.0"
    e6[0x829D] = IFDRational(4.0)
    e6[0x8833] = IFDRational(1 / 125)
    e6[0x8827] = 64
    e6[0x920A] = IFDRational(50)
    ex6[0x0112] = 6                     # Orientation (primary IFD)
    save("nikon_rot6.jpg", Image.new("RGB", (120, 80), (20, 60, 200)), ex6)

    save("plain.jpg", Image.new("RGB", (120, 80), (90, 90, 90)))


if __name__ == "__main__":
    main()
