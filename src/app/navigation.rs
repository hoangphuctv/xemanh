use macroquad::prelude::*;

use crate::gallery::{file_name_of, Gallery};
use crate::image_io::LoadedImage;
use crate::platform;
use crate::toolbar::TOOLBAR_HEIGHT;

use super::App;

impl App {
/// Loads the image at index, resizes the window accordingly and resets the view.
pub(crate) fn load_index(&mut self, index: usize) {
let path = self.gallery.entries[index].clone();
match LoadedImage::load(&path).and_then(|img| {
let texture = img.upload_texture()?;
Ok((img, texture))
}) {
Ok((image, texture)) => {
self.crop_state.reset();
self.gallery.index = index;
self.image = image;
self.texture = texture;
self.reset_view();
if !self.fullscreen && !platform::is_zoomed(self.hwnd) {
let current_w = screen_width();
let current_h = screen_height();
let dpi = screen_dpi_scale().max(1.0);
let (w_img, h_img) = platform::clamp_window_target(
self.texture.width() / dpi,
self.texture.height() / dpi + TOOLBAR_HEIGHT,
dpi,
);

if current_w < w_img || current_h < h_img {
let mut w_target = w_img.max(current_w);
let mut h_target = h_img.max(current_h);

let (clamped_w, clamped_h) =
platform::clamp_window_target(w_target, h_target, dpi);

w_target = clamped_w.max(current_w);
h_target = clamped_h.max(current_h);

platform::request_window_size(w_target, h_target);
}
}
self.update_title();
if self.fullscreen {
self.set_toast(self.gallery.title_label(), false);
}
self.toolbar.visible = true;
self.toolbar.user_hidden = false;
self.last_mouse_move = get_time();
}
Err(err) => self.set_toast(format!("[{}] {}", index + 1, err), true),
}
}

/// Handles files dropped onto the window, reloading the gallery & image.
pub(crate) fn handle_dropped_files(&mut self) {
let dropped = macroquad::input::get_dropped_files();
if dropped.is_empty() {
return;
}
let Some(first_path) = dropped.into_iter().find_map(|f| f.path) else {
return;
};
match Gallery::from_path(first_path) {
Ok(gallery) => {
let Some(current_path) = gallery.current() else {
self.set_toast("No images found in dropped location", true);
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
self.set_toast(format!("Opened {name}"), false);
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

pub(crate) fn next_image(&mut self) {
if let Some(next) = self.gallery.next_index() {
self.load_index(next);
self.slideshow_elapsed = 0.0;
}
}

pub(crate) fn prev_image(&mut self) {
if let Some(prev) = self.gallery.prev_index() {
self.load_index(prev);
self.slideshow_elapsed = 0.0;
}
}

/// Deletes the current image to the Recycle Bin and shows the next one.
pub(crate) fn delete_current(&mut self) {
if self.gallery.is_empty() {
self.set_toast("No image to delete", true);
return;
}
let path = self.gallery.current_path();
match platform::recycle_delete(self.hwnd, &path) {
Ok(()) => {
if let Some((name, empty)) = self.gallery.remove_current() {
if empty {
self.set_toast(format!("Deleted {name}. No more images."), false);
self.update_title();
return;
}
let next = self.gallery.index;
self.load_index(next);
self.set_toast(format!("Deleted {name}"), false);
}
}
Err(err) => self.set_toast(err, true),
}
}
}
