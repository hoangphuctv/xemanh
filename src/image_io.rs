use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, ImageOutputFormat, RgbaImage};
use macroquad::prelude::*;

/// Rotate direction.
#[derive(Clone, Copy, PartialEq)]
pub enum Rot {
    None,
    Cw,
    Ccw,
}

pub struct LoadedImage {
    inner: DynamicImage,
    rgba: RgbaImage,
    path: String,
    has_transparency: bool,
}

impl LoadedImage {
    pub fn load(path: &Path) -> Result<Self, String> {
        let path_str = path.to_string_lossy().to_string();
        let img = image::open(path).map_err(|e| format!("Failed to open {}: {}", path_str, e))?;
        let rgba = img.to_rgba8();
        let has_transparency = Self::check_transparency(&rgba);
        Ok(Self {
            inner: img,
            rgba,
            path: path_str,
            has_transparency,
        })
    }

    fn check_transparency(rgba: &RgbaImage) -> bool {
        // Check if any pixel has alpha < 255
        rgba.pixels().any(|p| p.0[3] != 255)
    }

    pub fn upload_texture(&self) -> Result<Texture2D, String> {
        let data = self.rgba.as_raw();
        let texture = Texture2D::from_rgba8(self.rgba.width() as u16, self.rgba.height() as u16, data);
        if texture.width() == 0.0 || texture.height() == 0.0 {
            Err("Failed to upload texture".to_string())
        } else {
            Ok(texture)
        }
    }

    pub fn rgba(&self) -> &RgbaImage {
        &self.rgba
    }

    pub fn has_transparency(&self) -> bool {
        self.has_transparency
    }

    pub fn png_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        self.inner
            .write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;
        Ok(bytes)
    }

    pub fn save(&self) -> Result<(), String> {
        self.inner
            .save(&self.path)
            .map_err(|e| format!("Failed to save {}: {}", self.path, e))
    }

    pub fn rotate(&mut self, rot: Rot) {
        if let Rot::Cw = rot {
            self.inner = self.inner.rotate90();
        } else if let Rot::Ccw = rot {
            self.inner = self.inner.rotate270();
        }
        self.rgba = self.inner.to_rgba8();
        self.has_transparency = Self::check_transparency(&self.rgba);
    }
}

pub fn make_checkerboard() -> Texture2D {
    let tile = 16;
    let size = tile * 2;
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let gray = if (x / tile + y / tile) % 2 == 0 { 180 } else { 100 };
            pixels[idx] = gray;
            pixels[idx + 1] = gray;
            pixels[idx + 2] = gray;
            pixels[idx + 3] = 255;
        }
    }
    let texture = Texture2D::from_rgba8(size as u16, size as u16, &pixels);
    if texture.width() == 0.0 || texture.height() == 0.0 {
        Texture2D::from_rgba8(1, 1, &[255, 255, 255, 255])
    } else {
        texture
    }
}
