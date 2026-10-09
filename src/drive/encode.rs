//! Re-encoding a downloaded photo into the sizes the pages use: AVIF and progressive JPEG per
//! width, like tools/images.py. Nothing of the original file but the pixels is kept (no EXIF, so
//! no GPS position), after applying its EXIF orientation and converting an embedded colour
//! profile (Display P3, Adobe RGB) to sRGB.

use std::io::Cursor;

use image::{imageops::FilterType, DynamicImage, ImageDecoder, ImageReader, Limits};
use jpeg_encoder::{ColorType, Encoder as JpegEncoder};
use ravif::{Encoder as AvifEncoder, Img, RGB8};

/// Bump when the output changes (settings, widths): every photo is encoded again, under new
/// file names.
pub const PIPELINE_VERSION: u32 = 3;
const JPEG_QUALITY: u8 = 80;
const AVIF_QUALITY: f32 = 62.0;
/// rav1e preset: 8 is about 1.3x faster than 6 for ~1% larger files (encode_speed below).
/// For a much faster encoder build with `--features fast-avif` (needs nasm).
const AVIF_SPEED: u8 = 8;
/// Larger originals are refused: the decoded image must fit in this many bytes (a 24 MP 8-bit RGB
/// photo is 72 MB; 16-bit PNGs count double).
const MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;
/// The page background: transparent pixels (PNG) are flattened onto it.
const PAGE_BG: [u8; 3] = [10, 12, 15];

pub struct Encoded {
    pub width: u32,
    pub height: u32,
    pub widths: Vec<u32>,
    /// (file name suffix "-<w>.avif" / "-<w>.jpg", bytes)
    pub files: Vec<(String, Vec<u8>)>,
}

/// Decodes `bytes` (JPEG, PNG or WebP) and encodes it at every width of `ladder` below the
/// original's, plus the original width capped at the largest ladder step.
pub fn encode(bytes: &[u8], ladder: &[u32], threads: usize) -> Result<Encoded, String> {
    let (image, icc) = decode(bytes)?;
    let mut rgb = flatten(image);
    if let Some(icc) = icc {
        to_srgb(&mut rgb, &icc);
    }
    let (width, height) = rgb.dimensions();
    let widths = widths_for(width, ladder);
    let mut files = Vec::with_capacity(widths.len() * 2);
    for &w in &widths {
        let h = ((height as f64 * w as f64 / width as f64).round() as u32).max(1);
        let resized = if w == width { rgb.clone() } else { image::imageops::resize(&rgb, w, h, FilterType::Lanczos3) };
        files.push((format!("-{w}.avif"), avif(&resized, threads)?));
        files.push((format!("-{w}.jpg"), jpeg(&resized)?));
    }
    let largest = *widths.last().ok_or("no widths")?;
    let height_at_largest = ((height as f64 * largest as f64 / width as f64).round() as u32).max(1);
    Ok(Encoded { width: largest, height: height_at_largest, widths, files })
}

/// Size and widths `encode` would produce, from the file header only (no pixel decoding): lets
/// a sync that was interrupted reuse the files it already wrote.
pub fn probe(bytes: &[u8], ladder: &[u32]) -> Result<(u32, u32, Vec<u32>), String> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut decoder = reader.into_decoder().map_err(|e| format!("unreadable image: {e}"))?;
    let (w, h) = decoder.dimensions();
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let (width, height) = match orientation {
        image::metadata::Orientation::Rotate90
        | image::metadata::Orientation::Rotate270
        | image::metadata::Orientation::Rotate90FlipH
        | image::metadata::Orientation::Rotate270FlipH => (h, w),
        _ => (w, h),
    };
    let widths = widths_for(width, ladder);
    let largest = *widths.last().ok_or("no widths")?;
    let height_at_largest = ((height as f64 * largest as f64 / width as f64).round() as u32).max(1);
    Ok((largest, height_at_largest, widths))
}

/// The oriented image and its embedded ICC profile, if any.
fn decode(bytes: &[u8]) -> Result<(DynamicImage, Option<Vec<u8>>), String> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| format!("unreadable image: {e}"))?;
    // Checked before decoding: the decoder's own allocation limit does not cover the output.
    if decoder.total_bytes() > MAX_DECODED_BYTES {
        let (w, h) = decoder.dimensions();
        return Err(format!("{w}x{h} is too large"));
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let icc = decoder.icc_profile().ok().flatten();
    let mut image = DynamicImage::from_decoder(decoder).map_err(|e| format!("unreadable image: {e}"))?;
    image.apply_orientation(orientation);
    Ok((image, icc))
}

/// Converts pixels in an embedded RGB colour profile to sRGB, which is what browsers assume for
/// files without a profile. Profiles that cannot be used (gray, CMYK, broken) leave the pixels
/// as they are.
fn to_srgb(rgb: &mut image::RgbImage, icc: &[u8]) {
    use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};
    let Ok(source) = ColorProfile::new_from_slice(icc) else { return };
    if source.color_space != DataColorSpace::Rgb {
        return;
    }
    let srgb = ColorProfile::new_srgb();
    let Ok(transform) = source.create_transform_8bit(Layout::Rgb, &srgb, Layout::Rgb, TransformOptions::default())
    else {
        return;
    };
    let mut out = vec![0; rgb.as_raw().len()];
    if transform.transform(rgb.as_raw(), &mut out).is_ok() {
        let (w, h) = rgb.dimensions();
        if let Some(converted) = image::RgbImage::from_raw(w, h, out) {
            *rgb = converted;
        }
    }
}

/// 8-bit RGB; any alpha is composited onto the page background.
fn flatten(image: DynamicImage) -> image::RgbImage {
    if !image.color().has_alpha() {
        return image.into_rgb8();
    }
    let rgba = image.into_rgba8();
    let (w, h) = rgba.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for (dst, src) in out.pixels_mut().zip(rgba.pixels()) {
        let a = u16::from(src[3]);
        for c in 0..3 {
            dst[c] = ((u16::from(src[c]) * a + u16::from(PAGE_BG[c]) * (255 - a) + 127) / 255) as u8;
        }
    }
    out
}

/// Ladder steps narrower than the original, then the original width capped at the largest step.
pub fn widths_for(width: u32, ladder: &[u32]) -> Vec<u32> {
    let max = ladder.last().copied().unwrap_or(width);
    let top = width.min(max);
    let mut widths: Vec<u32> = ladder.iter().copied().filter(|&w| w < top).collect();
    widths.push(top);
    widths
}

fn avif(image: &image::RgbImage, threads: usize) -> Result<Vec<u8>, String> {
    let pixels: Vec<RGB8> = image.pixels().map(|p| RGB8::new(p[0], p[1], p[2])).collect();
    let (w, h) = image.dimensions();
    AvifEncoder::new()
        .with_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED)
        .with_num_threads(Some(threads.max(1)))
        .encode_rgb(Img::new(&pixels[..], w as usize, h as usize))
        .map(|encoded| encoded.avif_file)
        .map_err(|e| format!("AVIF encoding failed: {e}"))
}

fn jpeg(image: &image::RgbImage) -> Result<Vec<u8>, String> {
    let (w, h) = image.dimensions();
    let (w16, h16) = (u16::try_from(w).map_err(|e| e.to_string())?, u16::try_from(h).map_err(|e| e.to_string())?);
    let mut out = Vec::new();
    let mut encoder = JpegEncoder::new(&mut out, JPEG_QUALITY);
    encoder.set_progressive(true);
    encoder.set_optimized_huffman_tables(true);
    encoder.encode(image.as_raw(), w16, h16, ColorType::Rgb).map_err(|e| format!("JPEG encoding failed: {e}"))?;
    Ok(out)
}

/// FNV-1a over the source bytes and the pipeline version, as 8 hex digits (file names only, not
/// security).
pub fn content_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in PIPELINE_VERSION.to_le_bytes().iter().chain(bytes) {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:08x}", (hash >> 32) as u32 ^ hash as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_ladder() {
        assert_eq!(widths_for(640, &[320, 480, 640]), [320, 480, 640]);
        assert_eq!(widths_for(600, &[320, 480, 640]), [320, 480, 600]);
        assert_eq!(widths_for(4000, &[480, 800, 1200, 1600, 2048]), [480, 800, 1200, 1600, 2048]);
        assert_eq!(widths_for(300, &[480, 800]), [300]);
    }

    #[test]
    fn encodes_and_strips_everything_but_pixels() {
        // A 64x40 PNG with an alpha channel.
        let mut png = Vec::new();
        let src = image::RgbaImage::from_fn(64, 40, |x, y| image::Rgba([x as u8 * 4, y as u8 * 6, 128, if x < 32 { 255 } else { 0 }]));
        DynamicImage::ImageRgba8(src).write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let out = encode(&png, &[32, 48, 64], 1).unwrap();
        assert_eq!((out.width, out.height), (64, 40));
        assert_eq!(out.widths, [32, 48, 64]);
        let names: Vec<&str> = out.files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["-32.avif", "-32.jpg", "-48.avif", "-48.jpg", "-64.avif", "-64.jpg"]);
        let jpg = &out.files[5].1;
        assert_eq!(&jpg[..2], [0xff, 0xd8]);
        assert!(!jpg.windows(4).any(|w| w == b"Exif"), "no EXIF segment");
        assert_eq!(&out.files[4].1[4..12], b"ftypavif");
        assert_eq!(probe(&png, &[32, 48, 64]).unwrap(), (64, 40, vec![32, 48, 64]));
        assert!(encode(b"not an image", &[32], 1).is_err());
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(content_hash(b"abc"), content_hash(b"abc"));
        assert_ne!(content_hash(b"abc"), content_hash(b"abd"));
        assert_eq!(content_hash(b"").len(), 8);
    }
}

/// Encoder timing on a real photo: `KOZMOSZ_BENCH_IMAGE=<file> cargo test --features ssr
/// encode_speed -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore]
fn encode_speed() {
    let path = std::env::var("KOZMOSZ_BENCH_IMAGE").expect("KOZMOSZ_BENCH_IMAGE");
    let bytes = std::fs::read(path).unwrap();
    let rgb = flatten(decode(&bytes).unwrap().0);
    let threads = std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).max(1));
    for &w in &[640u32, 1600] {
        let h = rgb.height() * w / rgb.width();
        let img = image::imageops::resize(&rgb, w, h, FilterType::Lanczos3);
        for speed in [6u8, 8, 10] {
            let pixels: Vec<RGB8> = img.pixels().map(|p| RGB8::new(p[0], p[1], p[2])).collect();
            let start = std::time::Instant::now();
            let out = AvifEncoder::new()
                .with_quality(AVIF_QUALITY)
                .with_speed(speed)
                .with_num_threads(Some(threads))
                .encode_rgb(Img::new(&pixels[..], w as usize, h as usize))
                .unwrap();
            println!("{w}px speed {speed}: {:?}, {} bytes ({threads} threads)", start.elapsed(), out.avif_file.len());
        }
    }
}
