use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::RgbaImage;
use macroquad::prelude::*;

use crate::constants::{CHECKER_TILE_PX, JPEG_SAVE_QUALITY};

/// Decoded image kept at full resolution so zoom/rotate never recompress pixels.
pub struct LoadedImage {
    rgba: RgbaImage,
    path: PathBuf,
    /// Original JPEG bytes (DCT data). Used for lossless orientation updates.
    jpeg_bytes: Option<Vec<u8>>,
    /// EXIF orientation that matches `rgba` when applied to `jpeg_bytes`.
    orientation: u32,
}

impl LoadedImage {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
        let orientation = exif_orientation(&bytes);
        let mut img =
            image::load_from_memory(&bytes).map_err(|e| format!("Cannot decode image: {e}"))?;
        if orientation > 1 {
            img = apply_orientation(img, orientation);
        }
        let jpeg_bytes = is_jpeg_path(path).then_some(bytes);
        Ok(Self {
            rgba: img.to_rgba8(),
            path: path.to_path_buf(),
            jpeg_bytes,
            orientation: orientation.clamp(1, 8),
        })
    }

    pub fn rgba(&self) -> &RgbaImage {
        &self.rgba
    }

    pub fn png_bytes(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut out);
        use image::ImageEncoder;
        encoder
            .write_image(
                self.rgba.as_raw(),
                self.rgba.width(),
                self.rgba.height(),
                image::ColorType::Rgba8,
            )
            .map_err(|e| format!("Cannot encode PNG: {e}"))?;
        Ok(out)
    }

    pub fn rotate(&mut self, rot: Rot) {
        self.rgba = match rot {
            Rot::None => return,
            Rot::Cw => image::imageops::rotate90(&self.rgba),
            Rot::Ccw => image::imageops::rotate270(&self.rgba),
        };
        self.orientation = match rot {
            Rot::None => self.orientation,
            Rot::Cw => rotate_orientation_cw(self.orientation),
            Rot::Ccw => rotate_orientation_ccw(self.orientation),
        };
    }

    pub fn upload_texture(&self) -> Result<Texture2D, String> {
        let (w, h) = self.rgba.dimensions();
        if w == 0 || h == 0 || w > u16::MAX as u32 || h > u16::MAX as u32 {
            return Err(format!("Image size {w}x{h} is not supported"));
        }
        let texture = Texture2D::from_rgba8(w as u16, h as u16, self.rgba.as_raw());
        configure_photo_texture(&texture);
        Ok(texture)
    }

    /// Writes the current pixels/orientation to disk without extra decode cycles.
    pub fn save(&self) -> Result<(), String> {
        if let Some(jpeg) = &self.jpeg_bytes {
            if let Some(out) = jpeg_with_orientation(jpeg, self.orientation as u16) {
                return fs::write(&self.path, out).map_err(|e| format!("Cannot write file: {e}"));
            }
            return encode_jpeg(&self.rgba, &self.path);
        }
        encode_lossless(&self.rgba, &self.path)
    }
}

fn is_jpeg_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "jpe" | "jfif"))
}

fn encode_lossless(rgba: &RgbaImage, path: &Path) -> Result<(), String> {
    let fmt = image::ImageFormat::from_path(path)
        .map_err(|e| format!("Unknown file format: {e}"))?;
    let img = image::DynamicImage::ImageRgba8(rgba.clone());
    let mut out_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut out_bytes), fmt)
        .map_err(|e| format!("Cannot encode image: {e}"))?;
    fs::write(path, out_bytes).map_err(|e| format!("Cannot write file: {e}"))
}

fn encode_jpeg(rgba: &RgbaImage, path: &Path) -> Result<(), String> {
    let rgb = image::DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
    let mut out_bytes = Vec::new();
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
    fs::write(path, out_bytes).map_err(|e| format!("Cannot write file: {e}"))
}

/// Linear + mipmaps when shrinking; sharp texels at 1:1 physical pixels.
pub fn configure_photo_texture(texture: &Texture2D) {
    let id = texture.raw_miniquad_id();
    let gl = unsafe { get_internal_gl() };
    gl.quad_context.texture_generate_mipmaps(id);
    gl.quad_context.texture_set_min_filter(
        id,
        FilterMode::Linear,
        miniquad::MipmapFilterMode::Linear,
    );
    gl.quad_context.texture_set_mag_filter(id, FilterMode::Linear);
}

/// Picks nearest sampling when the image is shown 1 image pixel = 1 screen pixel.
pub fn apply_display_filter(texture: &Texture2D, dest_w: f32, dest_h: f32) {
    let tw = texture.width();
    let th = texture.height();
    if tw <= 0.0 || th <= 0.0 {
        return;
    }
    let dpi = screen_dpi_scale().max(1.0);
    let sx = dest_w / tw * dpi;
    let sy = dest_h / th * dpi;
    let pixel_perfect = (sx - 1.0).abs() < 0.004 && (sy - 1.0).abs() < 0.004;
    let id = texture.raw_miniquad_id();
    let gl = unsafe { get_internal_gl() };
    if pixel_perfect {
        gl.quad_context.texture_set_min_filter(
            id,
            FilterMode::Nearest,
            miniquad::MipmapFilterMode::None,
        );
        gl.quad_context.texture_set_mag_filter(id, FilterMode::Nearest);
    } else {
        gl.quad_context.texture_set_min_filter(
            id,
            FilterMode::Linear,
            miniquad::MipmapFilterMode::Linear,
        );
        gl.quad_context.texture_set_mag_filter(id, FilterMode::Linear);
    }
}

fn exif_orientation(bytes: &[u8]) -> u32 {
    let exif = exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok();
    let Some(exif) = exif else {
        return 1;
    };
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
}

fn apply_orientation(img: image::DynamicImage, o: u32) -> image::DynamicImage {
    match o {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn rotate_orientation_cw(o: u32) -> u32 {
    match o {
        1 => 6,
        2 => 7,
        3 => 8,
        4 => 5,
        5 => 2,
        6 => 3,
        7 => 4,
        8 => 1,
        _ => 6,
    }
}

fn rotate_orientation_ccw(o: u32) -> u32 {
    match o {
        1 => 8,
        2 => 5,
        3 => 6,
        4 => 7,
        5 => 4,
        6 => 1,
        7 => 2,
        8 => 3,
        _ => 8,
    }
}

/// Lossless JPEG orientation: keep DCT data, only change/add EXIF Orientation.
fn jpeg_with_orientation(jpeg: &[u8], orientation: u16) -> Option<Vec<u8>> {
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return None;
    }
    let mut out = jpeg.to_vec();
    if patch_existing_orientation(&mut out, orientation) {
        return Some(out);
    }
    Some(insert_orientation_app1(jpeg, orientation))
}

fn patch_existing_orientation(jpeg: &mut [u8], orientation: u16) -> bool {
    let Some(app1) = find_exif_app1(jpeg) else {
        return false;
    };
    let tiff = &mut jpeg[app1.tiff_start..app1.seg_end];
    patch_tiff_orientation(tiff, orientation)
}

struct ExifApp1 {
    tiff_start: usize,
    seg_end: usize,
}

fn find_exif_app1(jpeg: &[u8]) -> Option<ExifApp1> {
    let mut i = 2usize;
    while i + 4 <= jpeg.len() {
        if jpeg[i] != 0xFF {
            return None;
        }
        let marker = jpeg[i + 1];
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            i += 2;
            continue;
        }
        let len = u16::from_be_bytes([jpeg[i + 2], jpeg[i + 3]]) as usize;
        if len < 2 || i + 2 + len > jpeg.len() {
            return None;
        }
        let payload = i + 4;
        let seg_end = i + 2 + len;
        if marker == 0xE1 && seg_end.saturating_sub(payload) >= 8 {
            let id = &jpeg[payload..payload + 6];
            if id == b"Exif\0\0" {
                return Some(ExifApp1 {
                    tiff_start: payload + 6,
                    seg_end,
                });
            }
        }
        i = seg_end;
    }
    None
}

fn patch_tiff_orientation(tiff: &mut [u8], orientation: u16) -> bool {
    if tiff.len() < 8 {
        return false;
    }
    let le = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return false,
    };
    let read_u16 = |b: &[u8], off: usize| -> Option<u16> {
        let s = b.get(off..off + 2)?;
        Some(if le {
            u16::from_le_bytes([s[0], s[1]])
        } else {
            u16::from_be_bytes([s[0], s[1]])
        })
    };
    let read_u32 = |b: &[u8], off: usize| -> Option<u32> {
        let s = b.get(off..off + 4)?;
        Some(if le {
            u32::from_le_bytes([s[0], s[1], s[2], s[3]])
        } else {
            u32::from_be_bytes([s[0], s[1], s[2], s[3]])
        })
    };
    let Some(ifd0) = read_u32(tiff, 4).map(|v| v as usize) else {
        return false;
    };
    let Some(count) = read_u16(tiff, ifd0).map(|v| v as usize) else {
        return false;
    };
    for n in 0..count {
        let e = ifd0 + 2 + n * 12;
        let Some(tag) = read_u16(tiff, e) else {
            return false;
        };
        if tag != 0x0112 {
            continue;
        }
        let Some(typ) = read_u16(tiff, e + 2) else {
            return false;
        };
        let Some(cnt) = read_u32(tiff, e + 4) else {
            return false;
        };
        if cnt != 1 {
            return false;
        }
        let val_off = e + 8;
        match typ {
            3 => {
                // SHORT, inlined in first 2 bytes of the value field
                if le {
                    tiff[val_off..val_off + 2].copy_from_slice(&orientation.to_le_bytes());
                } else {
                    tiff[val_off..val_off + 2].copy_from_slice(&orientation.to_be_bytes());
                }
                return true;
            }
            4 => {
                if le {
                    tiff[val_off..val_off + 4].copy_from_slice(&(orientation as u32).to_le_bytes());
                } else {
                    tiff[val_off..val_off + 4].copy_from_slice(&(orientation as u32).to_be_bytes());
                }
                return true;
            }
            _ => return false,
        }
    }
    false
}

fn insert_orientation_app1(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    let mut app1 = Vec::with_capacity(36);
    app1.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x22]);
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&[0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08]);
    app1.extend_from_slice(&[0x00, 0x01]);
    app1.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01]);
    app1.extend_from_slice(&orientation.to_be_bytes());
    app1.extend_from_slice(&[0x00, 0x00]);
    app1.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    let mut out = Vec::with_capacity(jpeg.len() + app1.len());
    out.extend_from_slice(&jpeg[..2]);
    out.extend_from_slice(&app1);
    out.extend_from_slice(&jpeg[2..]);
    out
}

pub fn make_checkerboard() -> Texture2D {
    let size = CHECKER_TILE_PX * 2;
    let tile = CHECKER_TILE_PX;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let v: u8 = if ((x / tile) + (y / tile)).is_multiple_of(2) {
                255
            } else {
                204
            };
            bytes.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let texture = Texture2D::from_rgba8(size, size, &bytes);
    texture.set_filter(FilterMode::Nearest);
    texture
}

#[derive(Clone, Copy, PartialEq)]
pub enum Rot {
    None,
    Cw,
    Ccw,
}
