#!/usr/bin/env python3
"""Responsive image pipeline for the Kozmosz site.

Reads the CONFIG list below, and for every key writes
    public/img/<key>-<w>.avif   (AVIF, quality 55, speed 6)
    public/img/<key>-<w>.jpg    (JPEG, quality 80, progressive, optimized, 4:2:0)
for each target width that fits the source, plus public/img/og.jpg (1200x630),
then generates src/images.rs.

Every output is metadata-free (no EXIF, GPS, ICC, XMP, comments): pixels are
EXIF-rotated, converted to sRGB, copied into a fresh image and re-encoded, and
each file is re-opened and its container structure checked afterwards.

Deterministic and re-runnable: files whose bytes did not change are not
rewritten, stale variants in the managed folders are removed.

Usage:  python -I tools/images.py [--src D:/Claude/kozmosz-wp-export]
Needs Pillow >= 11.3 (built-in AVIF support).
"""

from __future__ import annotations

import argparse
import io
import re
import struct
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageCms, ImageOps

# --------------------------------------------------------------------------
# Configuration
# --------------------------------------------------------------------------

# Steps close enough that a phone at DPR ~1.75-3 never gets a file far bigger than it draws.
HERO = [480, 720, 960, 1280, 1600, 1920]  # full-bleed / large images (1600: DPR-1 desktops)
MOON = [400, 560, 800, 1120]  # the Moon beside the projects intro (max ~520 CSS px)
PHOTO = [480, 720, 960, 1280]  # other community and astro photos
LECTURER = [480, 720, 945]  # lecturer portraits
SSTV = [320, 480, 640]  # SSTV reception images (sources are 640 wide)

# (key, source path relative to the export root, target widths).
# A source ending in "/*" means: every file in that folder, key = prefix + slug(stem).
CONFIG: list[tuple[str, str, list[int]]] = [
    # Community photos
    ("kozosseg/egbolt", "media-extra/IMG_5052-scaled.jpg", HERO),
    ("kozosseg/egbolt-allo", "media-extra/IMG_5052-scaled.jpg", PHOTO),  # see TRANSFORMS
    ("kozosseg/csoportkep", "media/ede2127c-f25b-40ec-a5bc-de8790124068-scaled.jpg", HERO),
    ("kozosseg/eloadas-szinpad", "media/IMG_1185-scaled.jpg", PHOTO),
    ("kozosseg/eloadas-vetites", "media-extra/IMG_1192-scaled.jpg", PHOTO),
    ("kozosseg/csillagvizsgalo", "media-extra/IMG_5080-scaled.jpg", PHOTO),
    ("kozosseg/csapat-terasz", "media-extra/IMG_5078-scaled.jpg", PHOTO),
    ("kozosseg/csapat-terem", "media-extra/DSC00915-scaled.jpg", PHOTO),
    ("kozosseg/muhely", "media-extra/DSC01261-scaled.jpg", PHOTO),
    ("kozosseg/irodaavato", "media-extra/irodaavato.png", PHOTO),
    ("kozosseg/eloadas-terem", "media-extra/PXL_20241212_171635845-scaled.jpg", PHOTO),
    # Astrophotography
    ("asztro/hold", "drive/asztro/Hold1 (1).jpg", HERO),
    ("asztro/hold-lap", "drive/asztro/Hold1 (1).jpg", MOON),  # see TRANSFORMS
    ("asztro/orion-kod", "media-extra/Orion_Nebula-scaled.jpg", PHOTO),
    ("asztro/orion-kod-szeles", "media-extra/Orion_Nebula-scaled.jpg", HERO),  # see TRANSFORMS
    ("asztro/laguna-kod", "media-extra/asztrokurzus_result.png", PHOTO),
    # Lecturers
    ("eloadok/szabo-jozsef", "media/image.png", LECTURER),
    ("eloadok/stepan-gabor", "media/image-1.png", LECTURER),
    ("eloadok/bacsardi-laszlo", "media/image-3.png", LECTURER),
    ("eloadok/horvath-gyula", "media/image-4.png", LECTURER),
    ("eloadok/kovacs-kalman", "media/image-5.png", LECTURER),
    ("eloadok/gschwindt-andras", "media/image-6.png", LECTURER),
    ("eloadok/szabo-nimrod", "media/image-7.png", LECTURER),
    ("eloadok/somodi-mate", "media/image-8.png", LECTURER),
    ("eloadok/medvegy-anna", "media/image-9.png", LECTURER),
    ("eloadok/detre-ors-hunor", "media/image-10.png", LECTURER),
    ("eloadok/pal-andras", "media/image-11.png", LECTURER),
    ("eloadok/szabo-robert", "media/image-12.png", LECTURER),
    ("eloadok/nagy-balazs-vince", "media/image-13.png", LECTURER),
    ("eloadok/pacher-tibor", "media/image-14.png", LECTURER),
    ("eloadok/sarneczky-krisztian", "media/image-16.png", LECTURER),
    # SSTV images: one key per file
    ("sstv/", "drive/sstv/*", SSTV),
]

# Page background of the site (style/main.css --bg); see TRANSFORMS.
PAGE_BG = (10, 12, 15)


def moon_on_page(im: Image.Image) -> Image.Image:
    """The Moon for the home page, where it bleeds off the edge into the page: the burned-in
    signature in the bottom-right corner of the sky is painted out (the credit is shown as a
    caption instead) and the black sky is lifted to the page colour, so no box edge shows."""
    w, h = im.size
    im = im.copy()
    im.paste((0, 0, 0), (round(w * 0.70), round(h * 0.88), w, h))
    return ImageChops.lighter(im, Image.new("RGB", im.size, PAGE_BG))


def portrait(center_x: float):
    """3:4 portrait crop of a landscape source, for a full-bleed hero on phones."""
    def crop(im: Image.Image) -> Image.Image:
        w, h = im.size
        cw = round(h * 3 / 4)
        left = min(max(0, round(w * center_x - cw / 2)), w - cw)
        return im.crop((left, 0, left + cw, h))
    return crop


def band(top: float, bottom: float):
    """Horizontal band of a portrait source: the part a wide, short header shows."""
    return lambda im: im.crop((0, round(im.height * top), im.width, round(im.height * bottom)))


# Per-key pixel transforms, applied after loading the source.
TRANSFORMS = {
    "asztro/hold-lap": moon_on_page,
    "kozosseg/egbolt-allo": portrait(0.5),
    "asztro/orion-kod-szeles": band(0.22, 0.62),
}

# Open Graph image: center crop of this key's source.
OG_KEY = "kozosseg/csoportkep"
OG_SIZE = (1200, 630)
OG_QUALITY = 82

AVIF_OPTS = {"quality": 55, "speed": 6, "codec": "aom", "subsampling": "4:2:0"}
JPEG_OPTS = {"quality": 80, "progressive": True, "optimize": True, "subsampling": "4:2:0"}

# Background for flattening transparency (none of the current sources use it).
FLATTEN_BG = (0, 0, 0)

# Files in public/img/ that are no longer used and must not linger.
OBSOLETE_GLOBS = ["home_bg-*.avif", "home_bg-*.webp", "home_bg-*.jpg"]

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "public" / "img"
RS_FILE = ROOT / "src" / "images.rs"
DEFAULT_SRC = ROOT.parent / "kozmosz-wp-export"

# --------------------------------------------------------------------------


def slug(stem: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", stem.lower()).strip("-")


def expand_config(src_root: Path) -> list[tuple[str, Path, list[int]]]:
    items: list[tuple[str, Path, list[int]]] = []
    for key, src, widths in CONFIG:
        if src.endswith("/*"):
            folder = src_root / src[:-2]
            files = sorted(p for p in folder.iterdir() if p.is_file())
            if not files:
                sys.exit(f"error: no files in {folder}")
            for p in files:
                items.append((key + slug(p.stem), p, widths))
        else:
            items.append((key, src_root / src, widths))
    keys = [k for k, _, _ in items]
    dupes = {k for k in keys if keys.count(k) > 1}
    if dupes:
        sys.exit(f"error: duplicate keys {sorted(dupes)}")
    return sorted(items, key=lambda t: t[0])


_SRGB = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB"))


def load_srgb(path: Path) -> Image.Image:
    """Open, apply EXIF orientation, convert to 8-bit sRGB RGB, drop all metadata."""
    with Image.open(path) as im:
        im.load()
        icc = im.info.get("icc_profile")
        im = ImageOps.exif_transpose(im)

    if icc:
        profile = ImageCms.ImageCmsProfile(io.BytesIO(icc))
        desc = (ImageCms.getProfileDescription(profile) or "").strip()
        if "srgb" not in desc.lower():  # e.g. Display P3, Adobe RGB, CMYK profiles
            if im.mode in ("RGBA", "LA", "PA") or "transparency" in im.info:
                im = _flatten(im)
            if im.mode not in ("RGB", "CMYK", "L"):
                im = im.convert("RGB")
            im = ImageCms.profileToProfile(
                im,
                profile,
                _SRGB,
                renderingIntent=ImageCms.Intent.RELATIVE_COLORIMETRIC,
                outputMode="RGB",
                flags=ImageCms.Flags.BLACKPOINTCOMPENSATION,
            )

    if im.mode != "RGB":
        im = _flatten(im) if (im.mode in ("RGBA", "LA", "PA") or "transparency" in im.info) else im.convert("RGB")

    # Fresh image: no info dict, so no ICC/EXIF/XMP/comment can leak into encoders.
    return Image.frombytes("RGB", im.size, im.tobytes())


def _flatten(im: Image.Image) -> Image.Image:
    rgba = im.convert("RGBA")
    bg = Image.new("RGBA", rgba.size, FLATTEN_BG + (255,))
    return Image.alpha_composite(bg, rgba).convert("RGB")


def plan_widths(src_w: int, widths: list[int]) -> tuple[list[int], list[int]]:
    """Target widths that fit the source (plus the source width when it is smaller
    than the largest request), and the requested widths that were skipped."""
    fit = sorted({w for w in widths if w <= src_w})
    skipped = sorted(w for w in widths if w > src_w)
    if src_w < max(widths) and src_w not in fit:
        fit.append(src_w)
    return sorted(fit), skipped


def encode(im: Image.Image, fmt: str, opts: dict) -> bytes:
    buf = io.BytesIO()
    im.save(buf, fmt, exif=b"", icc_profile=b"", **opts)
    return buf.getvalue()


def write_if_changed(path: Path, data: bytes, stats: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.read_bytes() == data:
        stats["unchanged"] += 1
        return
    path.write_bytes(data)
    stats["written"] += 1


# --------------------------------------------------------------------------
# Metadata verification (re-open every output)
# --------------------------------------------------------------------------

_INFO_FORBIDDEN = ("exif", "icc_profile", "xmp", "XML:com.adobe.xmp", "comment", "photoshop", "adobe")


def verify(path: Path, size: tuple[int, int]) -> list[str]:
    errors: list[str] = []
    data = path.read_bytes()
    with Image.open(path) as im:
        im.load()
        if im.size != size:
            errors.append(f"size {im.size} != {size}")
        if im.mode != "RGB":
            errors.append(f"mode {im.mode}")
        leaked = [k for k in _INFO_FORBIDDEN if im.info.get(k)]
        if leaked:
            errors.append(f"info keys {leaked}")
        if len(im.getexif()):
            errors.append("EXIF tags present")
    if path.suffix == ".jpg":
        errors += _jpeg_markers(data)
    elif path.suffix == ".avif":
        errors += _avif_boxes(data)
    return errors


def _jpeg_markers(data: bytes) -> list[str]:
    """Walk every JPEG segment (including between progressive scans)."""
    errors: list[str] = []
    if data[:2] != b"\xff\xd8":
        return ["not a JPEG"]
    i, n = 2, len(data)
    while i < n:
        if data[i] != 0xFF:
            i += 1  # entropy-coded data
            continue
        m = data[i + 1]
        if m == 0x00 or m == 0xFF or 0xD0 <= m <= 0xD7:
            i += 1 if m == 0xFF else 2  # stuffed byte, fill, RSTn
            continue
        if m == 0xD9:
            break
        seg_len = struct.unpack(">H", data[i + 2 : i + 4])[0]
        payload = data[i + 4 : i + 2 + seg_len]
        if m == 0xE0:
            if not payload.startswith(b"JFIF\x00"):
                errors.append("APP0 is not JFIF")
            elif payload[12:14] != b"\x00\x00":
                errors.append("JFIF thumbnail")
        elif 0xE1 <= m <= 0xEF:
            errors.append(f"APP{m - 0xE0} segment {payload[:12]!r}")
        elif m == 0xFE:
            errors.append("COM segment")
        i += 2 + seg_len
    return errors


def _boxes(data: bytes, start: int, end: int):
    i = start
    while i + 8 <= end:
        size, typ = struct.unpack(">I4s", data[i : i + 8])
        hdr = 8
        if size == 1:
            size = struct.unpack(">Q", data[i + 8 : i + 16])[0]
            hdr = 16
        elif size == 0:
            size = end - i
        yield typ.decode("latin-1"), i + hdr, i + size
        i += size


def _avif_boxes(data: bytes) -> list[str]:
    """Check the HEIF item list and properties: only an av01 image item, nclx colour."""
    errors: list[str] = []
    top = {t: (s, e) for t, s, e in _boxes(data, 0, len(data))}
    if "meta" not in top:
        return ["no meta box"]
    s, e = top["meta"]
    meta = {t: (cs, ce) for t, cs, ce in _boxes(data, s + 4, e)}  # FullBox header
    item_types = []
    if "iinf" in meta:
        s, e = meta["iinf"]
        version = data[s]
        off = s + 4 + (2 if version == 0 else 4)
        for t, cs, ce in _boxes(data, off, e):
            if t != "infe":
                continue
            v = data[cs]
            p = cs + 4 + (2 if v == 2 else 4) + 2
            item_types.append(data[p : p + 4].decode("latin-1"))
    extra = [t for t in item_types if t != "av01"]
    if extra:
        errors.append(f"extra items {extra}")  # 'Exif', 'mime' (XMP), ...
    if "iprp" in meta:
        s, e = meta["iprp"]
        for t, cs, ce in _boxes(data, s, e):
            if t != "ipco":
                continue
            for pt, ps, pe in _boxes(data, cs, ce):
                if pt == "colr" and data[ps : ps + 4] != b"nclx":
                    errors.append(f"colr {data[ps:ps + 4]!r} (ICC)")
    return errors


# --------------------------------------------------------------------------


def render_rust(entries: list[tuple[str, int, int, list[int]]]) -> str:
    out = [
        "//! Generated by tools/images.py. Do not edit by hand.",
        "",
        "pub struct Image {",
        "    pub key: &'static str,",
        "    pub width: u32,",
        "    pub height: u32,",
        "    pub widths: &'static [u32],",
        "}",
        "",
        "pub const IMAGES: &[Image] = &[",
    ]
    for key, w, h, widths in entries:
        out += [
            "    Image {",
            f'        key: "{key}",',
            f"        width: {w},",
            f"        height: {h},",
            f"        widths: &[{', '.join(map(str, widths))}],",
            "    },",
        ]
    out += [
        "];",
        "",
        "pub fn get(key: &str) -> Option<&'static Image> {",
        "    IMAGES.iter().find(|i| i.key == key)",
        "}",
        "",
    ]
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--src", type=Path, default=DEFAULT_SRC, help="WordPress export root")
    args = ap.parse_args()

    items = expand_config(args.src)
    stats = {"written": 0, "unchanged": 0, "removed": 0}
    expected: set[Path] = set()
    produced: list[tuple[Path, tuple[int, int]]] = []
    rust_entries: list[tuple[str, int, int, list[int]]] = []
    skipped_report: list[str] = []
    sources: dict[str, Image.Image] = {}

    for key, src, widths in items:
        im = load_srgb(src)
        if key in TRANSFORMS:
            im = TRANSFORMS[key](im)
        if key == OG_KEY:
            sources[key] = im
        src_w, src_h = im.size
        targets, skipped = plan_widths(src_w, widths)
        if skipped:
            skipped_report.append(f"{key}: source {src_w}px, skipped {skipped}, using {targets}")
        largest = (0, 0)
        for w in targets:
            h = max(1, round(src_h * w / src_w))
            variant = im if w == src_w else im.resize((w, h), Image.Resampling.LANCZOS)
            for ext, fmt, opts in (("avif", "AVIF", AVIF_OPTS), ("jpg", "JPEG", JPEG_OPTS)):
                path = OUT_DIR / f"{key}-{w}.{ext}"
                write_if_changed(path, encode(variant, fmt, opts), stats)
                expected.add(path)
                produced.append((path, (w, h)))
            largest = (w, h)
        rust_entries.append((key, largest[0], largest[1], targets))

    # Open Graph image
    og = ImageOps.fit(sources[OG_KEY], OG_SIZE, Image.Resampling.LANCZOS, centering=(0.5, 0.5))
    og_path = OUT_DIR / "og.jpg"
    write_if_changed(og_path, encode(og, "JPEG", {**JPEG_OPTS, "quality": OG_QUALITY}), stats)
    produced.append((og_path, OG_SIZE))

    # Remove stale variants in managed folders and obsolete files.
    for folder in sorted({OUT_DIR / k.split("/")[0] for k, _, _ in items if "/" in k}):
        for p in sorted(folder.rglob("*")):
            if p.is_file() and p.suffix in (".avif", ".jpg", ".webp") and p not in expected:
                p.unlink()
                stats["removed"] += 1
    for pattern in OBSOLETE_GLOBS:
        for p in sorted(OUT_DIR.glob(pattern)):
            p.unlink()
            stats["removed"] += 1

    # Rust table
    rs = render_rust(rust_entries)
    RS_FILE.parent.mkdir(parents=True, exist_ok=True)
    if not RS_FILE.exists() or RS_FILE.read_text(encoding="utf-8") != rs:
        RS_FILE.write_bytes(rs.encode("utf-8"))
        print(f"wrote {RS_FILE.relative_to(ROOT)}")

    # Verify every output
    failures = 0
    for path, size in produced:
        errs = verify(path, size)
        if errs:
            failures += 1
            print(f"FAIL {path.relative_to(ROOT)}: {'; '.join(errs)}")

    # Report
    groups: dict[str, list[int]] = {}
    for path, _ in produced:
        rel = path.relative_to(OUT_DIR)
        group = rel.parts[0] if len(rel.parts) > 1 else "(root)"
        fmt = path.suffix[1:]
        g = groups.setdefault(group, [0, 0, 0])
        g[0] += path.stat().st_size if fmt == "avif" else 0
        g[1] += path.stat().st_size if fmt == "jpg" else 0
        g[2] += 1
    total = sum(a + j for a, j, _ in groups.values())
    print(f"{len(items)} keys, {len(produced)} files: {stats['written']} written, "
          f"{stats['unchanged']} unchanged, {stats['removed']} removed")
    for group, (a, j, n) in sorted(groups.items()):
        print(f"  {group:10s} {n:3d} files  avif {a / 1024:8.1f} KiB  jpg {j / 1024:8.1f} KiB")
    print(f"  {'total':10s} {len(produced):3d} files  {total / 1024:.1f} KiB ({total / 1048576:.2f} MiB)")
    for line in skipped_report:
        print(f"  skipped: {line}")
    if failures:
        print(f"{failures} files FAILED metadata/size verification")
        return 1
    print(f"verified {len(produced)} files: no EXIF/GPS/ICC/XMP/comments, sizes match")
    return 0


if __name__ == "__main__":
    sys.exit(main())
