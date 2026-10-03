use std::path::{Path, PathBuf};

use macroquad::prelude::*;

use crate::toolbar::TOOLBAR_HEIGHT;

use super::{App, CropInputFocus, CropState};

impl CropState {
pub fn reset(&mut self) {
self.active = false;
self.start_pos = None;
self.current_pos = Vec2::ZERO;
self.dragging = false;
self.fixed_w_str.clear();
self.fixed_h_str.clear();
self.active_input = None;
}

pub fn get_fixed_size(&self) -> Option<(u32, u32)> {
let w = self.fixed_w_str.trim().parse::<u32>().ok()?;
let h = self.fixed_h_str.trim().parse::<u32>().ok()?;
if w > 0 && h > 0 {
Some((w, h))
} else {
None
}
}

pub fn get_selection_rect(&self) -> Option<Rect> {
let start = self.start_pos?;
let min_x = start.x.min(self.current_pos.x);
let min_y = start.y.min(self.current_pos.y);
let max_x = start.x.max(self.current_pos.x);
let max_y = start.y.max(self.current_pos.y);
let w = max_x - min_x;
let h = max_y - min_y;

if w > 2.0 && h > 2.0 {
Some(Rect::new(min_x, min_y, w, h))
} else {
None
}
}
}

impl App {
pub(crate) fn generate_crop_filename(original_path: &Path) -> PathBuf {
let parent = original_path.parent().unwrap_or_else(|| Path::new(""));
let stem = original_path
.file_stem()
.and_then(|s| s.to_str())
.unwrap_or("image");

// Strip existing _cropN suffix if present to prevent file_crop1_crop2.jpg
let base_stem = if let Some(idx) = stem.rfind("_crop") {
if stem[idx + 5..].chars().all(|c| c.is_ascii_digit()) && !stem[idx + 5..].is_empty() {
&stem[..idx]
} else {
stem
}
} else {
stem
};
let ext = original_path
.extension()
.and_then(|e| e.to_str())
.unwrap_or("png");

let mut count = 1;
loop {
let candidate_name = format!("{}_crop{}.{}", base_stem, count, ext);
let candidate_path = parent.join(candidate_name);
if !candidate_path.exists() {
return candidate_path;
}
count += 1;
}
}

pub(crate) fn apply_crop(&mut self) {
let Some(crop_rect) = self.crop_state.get_selection_rect() else {
self.set_toast("Vùng chọn cắt quá nhỏ", true);
return;
};

let win_w = screen_width();
let win_h = screen_height();
let tex_w = self.texture.width();
let tex_h = self.texture.height();
// Reserve the toolbar strip at the top whenever windowed; the toolbar
// never overlaps the image, and toggling it does not move the image.
let (top_offset, available_h) = if self.fullscreen {
(0.0, win_h)
} else {
(TOOLBAR_HEIGHT, (win_h - TOOLBAR_HEIGHT).max(1.0))
};
let img_rect = self.view.view_rect(tex_w, tex_h, win_w, available_h, top_offset);

let fixed_size = self.crop_state.get_fixed_size();
let (x, y, w, h) = if let Some((fixed_w, fixed_h)) = fixed_size {
let image_w = tex_w.round() as u32;
let image_h = tex_h.round() as u32;
if fixed_w > image_w || fixed_h > image_h {
self.set_toast("Kích thước crop lớn hơn ảnh gốc", true);
return;
}

let rel_x = (crop_rect.x - img_rect.x) / img_rect.w;
let rel_y = (crop_rect.y - img_rect.y) / img_rect.h;
let x = (rel_x * tex_w)
.round()
.clamp(0.0, (image_w - fixed_w) as f32) as u32;
let y = (rel_y * tex_h)
.round()
.clamp(0.0, (image_h - fixed_h) as f32) as u32;
(x, y, fixed_w, fixed_h)
} else {
// Convert screen crop rect to texture pixel space
let rel_x = (crop_rect.x - img_rect.x) / img_rect.w;
let rel_y = (crop_rect.y - img_rect.y) / img_rect.h;
let rel_w = crop_rect.w / img_rect.w;
let rel_h = crop_rect.h / img_rect.h;

let x = (rel_x * tex_w).round().max(0.0) as u32;
let y = (rel_y * tex_h).round().max(0.0) as u32;
let w = (rel_w * tex_w).round() as u32;
let h = (rel_h * tex_h).round() as u32;
(x, y, w, h)
};

if w == 0 || h == 0 {
self.set_toast("Vùng chọn cắt không hợp lệ", true);
return;
}

self.image.crop(x, y, w, h);
match self.image.upload_texture() {
Ok(new_tex) => {
self.texture = new_tex;
let current_path = self.gallery.current_path();
let new_path = Self::generate_crop_filename(&current_path);

if let Err(e) = self.image.save_to_path(&new_path) {
self.set_toast(format!("Lỗi lưu ảnh cắt: {}", e), true);
return;
}

// Insert saved crop image to gallery right after current image
let insert_idx = self.gallery.index + 1;
self.gallery.entries.insert(insert_idx, new_path);
self.load_index(insert_idx);

self.crop_state.reset();
self.set_toast("Đã cắt & lưu ảnh thành công!", false);
}
Err(e) => {
self.set_toast(e, true);
}
}
}

pub(crate) fn crop_input_rects(&self, img_rect: Rect) -> (Rect, Rect) {
let x = img_rect.x + 12.0;
let y = img_rect.y + if self.fullscreen {
TOOLBAR_HEIGHT + 12.0
} else {
12.0
};
let w = 120.0;
let h = 34.0;
(
Rect::new(x, y, w, h),
Rect::new(x + w + 12.0, y, w, h),
)
}

pub(crate) fn draw_crop_overlay(&self, img_rect: Rect) {
if !self.crop_state.active {
return;
}

let dim_color = Color::new(0.0, 0.0, 0.0, 0.5);

if let Some(selection) = self.crop_state.get_selection_rect() {
// Draw dim overlay in 4 areas around selection box
// Top
draw_rectangle(img_rect.x, img_rect.y, img_rect.w, selection.y - img_rect.y, dim_color);
// Bottom
let bottom_y = selection.y + selection.h;
draw_rectangle(img_rect.x, bottom_y, img_rect.w, (img_rect.y + img_rect.h) - bottom_y, dim_color);
// Left
draw_rectangle(img_rect.x, selection.y, selection.x - img_rect.x, selection.h, dim_color);
// Right
let right_x = selection.x + selection.w;
draw_rectangle(right_x, selection.y, (img_rect.x + img_rect.w) - right_x, selection.h, dim_color);

// Draw selection box border & corners
draw_rectangle_lines(selection.x, selection.y, selection.w, selection.h, 2.0, WHITE);

// Draw corner handles
let handle_sz = 6.0;
let corners = [
(selection.x, selection.y),
(selection.x + selection.w, selection.y),
(selection.x, selection.y + selection.h),
(selection.x + selection.w, selection.y + selection.h),
];
for (cx, cy) in corners {
draw_rectangle(cx - handle_sz / 2.0, cy - handle_sz / 2.0, handle_sz, handle_sz, WHITE);
}
} else {
// Dim whole image if no selection yet
draw_rectangle(img_rect.x, img_rect.y, img_rect.w, img_rect.h, dim_color);
}

let (width_rect, height_rect) = self.crop_input_rects(img_rect);
let fields = [
(
width_rect,
"W",
&self.crop_state.fixed_w_str,
self.crop_state.active_input == Some(CropInputFocus::Width),
),
(
height_rect,
"H",
&self.crop_state.fixed_h_str,
self.crop_state.active_input == Some(CropInputFocus::Height),
),
];
for (rect, label, value, active) in fields {
draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.0, 0.0, 0.0, 0.8));
draw_rectangle_lines(
rect.x,
rect.y,
rect.w,
rect.h,
2.0,
if active { WHITE } else { GRAY },
);
let text = if value.is_empty() {
format!("{label}:")
} else {
format!("{label}: {value}")
};
draw_text_ex(
&text,
rect.x + 8.0,
rect.y + 23.0,
TextParams {
font: Some(&self.font),
font_size: 20,
color: WHITE,
..Default::default()
},
);
}
}
}
