use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage, ImageOutputFormat, GenericImageView, RgbaImage};
use macroquad::prelude::*;

pub struct ImageAnimation {
    pub frames: Vec<Texture2D>,
    pub frame_durations: Vec<Duration>,
    pub current_frame: usize,
    pub elapsed_in_frame: Duration,
}

impl ImageAnimation {
    pub fn update(&mut self, dt: Duration) {
        if self.frames.is_empty() {
            return;
        }
        if self.frames.len() > 1 {
            self.elapsed_in_frame += dt;
            let mut current_dur = self.frame_durations[self.current_frame];
            if current_dur.is_zero() {
                current_dur = Duration::from_millis(100);
            }
            while self.elapsed_in_frame >= current_dur {
                self.elapsed_in_frame -= current_dur;
                self.current_frame = (self.current_frame + 1) % self.frames.len();
                current_dur = self.frame_durations[self.current_frame];
            }
        }
    }

    pub fn current_texture(&self) -> &Texture2D {
        &self.frames[self.current_frame]
    }
}

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
    animation: Option<ImageAnimation>,
}

impl LoadedImage {
    pub fn load(path: &Path) -> Result<Self, String> {
        let path_str = path.to_string_lossy().to_string();
        let img = image::open(path).map_err(|e| format!("Failed to open {}: {}", path_str, e))?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let can_have_transparency = !matches!(ext.as_str(), "jpg" | "jpeg" | "jfif" | "jpe" | "bmp");
        let is_gif = ext == "gif";

        let mut animation = None;
        if is_gif {
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                if let Ok(decoder) = GifDecoder::new(reader) {
                    if let Ok(raw_frames) = decoder.into_frames().collect_frames() {
                        if raw_frames.len() > 1 {
                            let mut textures = Vec::with_capacity(raw_frames.len());
                            let mut durations = Vec::with_capacity(raw_frames.len());
                            for frame in &raw_frames {
                                let buffer = frame.buffer();
                                let tex = Texture2D::from_rgba8(buffer.width() as u16, buffer.height() as u16, buffer.as_raw());
                                textures.push(tex);
                                let (num, denom) = frame.delay().numer_denom_ms();
                                let ms = if denom == 0 || num == 0 { 100 } else { (num / denom).max(10) };
                                let dur = Duration::from_millis(ms as u64);
                                durations.push(dur);
                            }
                            animation = Some(ImageAnimation {
                                frames: textures,
                                frame_durations: durations,
                                current_frame: 0,
                                elapsed_in_frame: Duration::ZERO,
                            });
                        }
                    }
                }
            }
        }

        let rgba = img.to_rgba8();
        let has_transparency = can_have_transparency && Self::check_transparency(&rgba);
        Ok(Self {
            inner: img,
            rgba,
            path: path_str,
            has_transparency,
            animation,
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

    pub fn animation_mut(&mut self) -> Option<&mut ImageAnimation> {
        self.animation.as_mut()
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

    pub fn save_to_path(&self, target_path: &Path) -> Result<(), String> {
        self.inner
            .save(target_path)
            .map_err(|e| format!("Failed to save {}: {}", target_path.display(), e))
    }

    pub fn rotate(&mut self, rot: Rot) {
        if let Rot::Cw = rot {
            self.inner = self.inner.rotate90();
        } else if let Rot::Ccw = rot {
            self.inner = self.inner.rotate270();
        } else {
            return;
        }
        self.rgba = self.inner.to_rgba8();
        self.animation = None;
    }

    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        let (img_w, img_h) = self.inner.dimensions();
        let x = x.min(img_w.saturating_sub(1));
        let y = y.min(img_h.saturating_sub(1));
        let width = width.min(img_w - x);
        let height = height.min(img_h - y);

        if width == 0 || height == 0 {
            return;
        }

        self.inner = self.inner.crop_imm(x, y, width, height);
        self.rgba = self.inner.to_rgba8();
        self.animation = None;
    }
}

pub fn make_checkerboard() -> Texture2D {
    let size = 16;
    let mut pixels = vec![0u8; size * size * 4];
    let c1 = [204u8, 204u8, 204u8, 255u8];
    let c2 = [255u8, 255u8, 255u8, 255u8];

    for y in 0..size {
        for x in 0..size {
            let is_even = (x / 8 + y / 8) % 2 == 0;
            let color = if is_even { c1 } else { c2 };
            let idx = (y * size + x) * 4;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }

    Texture2D::from_rgba8(size as u16, size as u16, &pixels)
}
