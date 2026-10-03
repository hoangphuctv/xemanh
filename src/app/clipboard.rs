use std::path::PathBuf;

use macroquad::prelude::*;

use crate::gallery::{file_name_of, Gallery};
use crate::image_io::{LoadedImage, Rot};
use crate::platform;

use super::App;

impl App {
pub(crate) fn paste_from_clipboard(&mut self) {
if self.hwnd == 0 {
self.hwnd = platform::find_hwnd();
}
let dynamic_img = match platform::read_image_from_clipboard(self.hwnd) {
Ok(img) => img,
Err(err) => {
self.set_toast(err, true);
return;
}
};

// 1. Target directory ~/Pictures/XemAnh
let dir = crate::gallery::resolve_path(PathBuf::from("~/Pictures/XemAnh"));
if let Err(e) = std::fs::create_dir_all(&dir) {
self.set_toast(format!("Failed to create folder: {}", e), true);
return;
}

// 2. Filename: xemanh-<timestamp>.jpg
let timestamp = std::time::SystemTime::now()
.duration_since(std::time::UNIX_EPOCH)
.map(|d| d.as_millis())
.unwrap_or(0);
let filename = format!("xemanh-{}.jpg", timestamp);
let saved_path = dir.join(filename);

// Save as JPEG
if let Err(e) = dynamic_img.save(&saved_path) {
self.set_toast(format!("Failed to save clipboard image: {}", e), true);
return;
}

// 3. Open created image
match Gallery::from_path(saved_path) {
Ok(gallery) => {
let Some(current_path) = gallery.current() else {
self.set_toast("Failed to locate pasted image", true);
return;
};
match LoadedImage::load(current_path).and_then(|img| {
let tex = img.upload_texture()?;
Ok((img, tex))
}) {
Ok((image, texture)) => {
self.crop_state.reset();
self.gallery = gallery;
self.image = image;
self.texture = texture;
self.reset_view();
if !self.fullscreen && !platform::is_zoomed(self.hwnd) {
self.request_window_for_texture();
}
self.update_title();
let name = file_name_of(&self.gallery.current_path());
self.set_toast(format!("Pasted {name}"), false);
self.toolbar.visible = true;
self.toolbar.user_hidden = false;
self.last_mouse_move = get_time();
}
Err(err) => self.set_toast(err, true),
}
}
Err(err) => self.set_toast(err, true),
}
}

pub(crate) fn copy_current(&mut self) {
if self.gallery.is_empty() {
self.set_toast("No image to copy", true);
return;
}
if self.hwnd == 0 {
self.hwnd = platform::find_hwnd();
}
let rgba = self.image.rgba();
let png = match self.image.png_bytes() {
Ok(bytes) => bytes,
Err(err) => {
self.set_toast(err, true);
return;
}
};
match platform::copy_image_to_clipboard(
self.hwnd,
rgba.width(),
rgba.height(),
rgba.as_raw(),
&png,
) {
Ok(()) => {
let name = file_name_of(&self.gallery.current_path());
self.set_toast(format!("Copied {name}"), false);
}
Err(err) => self.set_toast(err, true),
}
}

pub(crate) fn rotate_and_save(&mut self, rot: Rot) {
if rot != Rot::None {
self.image.rotate(rot);
match self.image.upload_texture() {
Ok(texture) => {
self.texture = texture;
self.reset_view();
if !self.fullscreen && !platform::is_zoomed(self.hwnd) {
self.request_window_for_texture();
}
self.update_title();
}
Err(err) => {
self.set_toast(err, true);
return;
}
}
}
let path = self.gallery.current_path();
match self.image.save() {
Ok(()) => self.set_toast(format!("Saved {}", file_name_of(&path)), false),
Err(err) => self.set_toast(err, true),
}
}
}
