use std::fs;
use std::io::Cursor;
use std::path::Path;

use macroquad::prelude::*;

use crate::constants::{CHECKER_TILE_PX, JPEG_SAVE_QUALITY};

/// Reads the EXIF orientation (1-8) from raw image bytes; 1 when absent or unreadable.
fn exif_orientation(bytes: &[u8]) -> u32 {
    let exif = exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok();
    let Some(exif) = exif else { return 1 };
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
}

/// Bakes the EXIF orientation into pixels so the image displays upright.
fn apply_orientation(img: image::DynamicImage, o: u32) -> image::DynamicImage {
    match o {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        // transpose = rotate 90 CW, then flip horizontal
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        // transverse = rotate 90 CCW (270 CW), then flip horizontal
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

/// Reads and decodes an image file, applying EXIF orientation when present.
fn load_dynamic(path: &Path) -> Result<image::DynamicImage, String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let mut img =
        image::load_from_memory(&bytes).map_err(|e| format!("Cannot decode image: {e}"))?;
    let orientation = exif_orientation(&bytes);
    if orientation > 1 {
        img = apply_orientation(img, orientation);
    }
    Ok(img)
}

pub fn load_texture(path: &Path) -> Result<Texture2D, String> {
    let img = load_dynamic(path)?;
    let rgba = img.to_rgba8();
    let mq_image = Image {
        width: rgba.width() as u16,
        height: rgba.height() as u16,
        bytes: rgba.into_raw(),
    };
    Ok(Texture2D::from_image(&mq_image))
}

/// 32x32 pattern texture (2x2 squares of 16 px, white / light gray) for transparent areas.
pub fn make_checkerboard() -> Texture2D {
    let size = CHECKER_TILE_PX * 2;
    let tile = CHECKER_TILE_PX;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let v: u8 = if ((x / tile) + (y / tile)) % 2 == 0 {
                255
            } else {
                204
            };
            bytes.extend_from_slice(&[v, v, v, 255]);
        }
    }
    Texture2D::from_image(&Image {
        width: size,
        height: size,
        bytes,
    })
}

#[derive(Clone, Copy, PartialEq)]
pub enum Rot {
    None,
    Cw,
    Ccw,
}

/// Re-encodes the image at `path` (optionally rotated) and overwrites the original file.
pub fn save_transformed(path: &Path, rot: Rot) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let img =
        image::load_from_memory(&bytes).map_err(|e| format!("Cannot decode image: {e}"))?;

    let fmt = image::ImageFormat::from_path(path)
        .map_err(|e| format!("Unknown file format: {e}"))?;

    let out_img = match fmt {
        image::ImageFormat::Jpeg => {
            let rgb8 = img.to_rgb8();
            image::DynamicImage::ImageRgb8(match rot {
                Rot::None => rgb8,
                Rot::Cw => image::imageops::rotate90(&rgb8),
                Rot::Ccw => image::imageops::rotate270(&rgb8),
            })
        }
        _ => {
            let rgba8 = img.to_rgba8();
            image::DynamicImage::ImageRgba8(match rot {
                Rot::None => rgba8,
                Rot::Cw => image::imageops::rotate90(&rgba8),
                Rot::Ccw => image::imageops::rotate270(&rgba8),
            })
        }
    };

    let mut out_bytes: Vec<u8> = Vec::new();
    match fmt {
        image::ImageFormat::Jpeg => {
            let rgb = out_img.to_rgb8();
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                Cursor::new(&mut out_bytes),
                JPEG_SAVE_QUALITY,
            );
            use image::ImageEncoder;
            encoder
                .write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ColorType::Rgb8,
                )
                .map_err(|e| format!("Cannot encode JPEG: {e}"))?;
        }
        _ => out_img
            .write_to(&mut Cursor::new(&mut out_bytes), fmt)
            .map_err(|e| format!("Cannot encode image: {e}"))?,
    }

    fs::write(path, &out_bytes).map_err(|e| format!("Cannot write file: {e}"))
}
