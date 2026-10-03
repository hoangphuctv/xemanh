use std::collections::HashMap;
use std::path::{Path, PathBuf};

use macroquad::prelude::*;

use crate::gallery::{file_name_of, Gallery};

/// Max thumbnails decoded per frame while filling the visible area.
/// Keeps scrolling smooth on large folders (lazy load).
const DECODE_BUDGET_PER_FRAME: usize = 3;
/// Thumbnail longest edge in pixels (source resolution for the texture).
const THUMB_MAX_EDGE: u32 = 256;
/// Hard cap on cached thumbnails; evicts least-recently-used beyond this.
const CACHE_CAP: usize = 512;
/// Grid cell padding in logical pixels.
const CELL_PAD: f32 = 10.0;
/// Desired cell width; actual columns are derived from the window width.
const TARGET_CELL_W: f32 = 180.0;
const MIN_CELL_W: f32 = 96.0;
const MAX_CELL_W: f32 = 260.0;

struct Thumb {
texture: Texture2D,
last_used: u64,
}

/// Full-folder grid view with lazy thumbnail loading.
#[derive(Default)]
pub struct GridMode {
pub active: bool,
pub selected: usize,
pub scroll: f32,
pub scroll_target: f32,
cache: HashMap<PathBuf, Thumb>,
access_counter: u64,
pub cols: usize,
pub cell_w: f32,
pub cell_h: f32,
rects: Vec<Rect>,
}

impl GridMode {
pub fn open(&mut self, selected: usize) {
self.active = true;
self.selected = selected;
self.scroll = 0.0;
self.scroll_target = 0.0;
self.center_on_selected();
}

pub fn close(&mut self) {
self.active = false;
}

fn update_layout(&mut self, entry_count: usize, win_w: f32, top_offset: f32) {
let usable_w = (win_w - CELL_PAD).max(MIN_CELL_W);
let cols = ((usable_w / (TARGET_CELL_W + CELL_PAD)).floor() as usize).max(1);
let cell_w = ((usable_w - CELL_PAD * (cols as f32 + 1.0)) / cols as f32)
.clamp(MIN_CELL_W, MAX_CELL_W);
let cell_h = cell_w;

self.cols = cols;
self.cell_w = cell_w;
self.cell_h = cell_h;

self.rects.clear();
self.rects.reserve(entry_count);
for i in 0..entry_count {
let row = i / cols;
let col = i % cols;
let x = CELL_PAD + col as f32 * (cell_w + CELL_PAD);
let y = top_offset + CELL_PAD + row as f32 * (cell_h + CELL_PAD);
self.rects.push(Rect::new(x, y, cell_w, cell_h));
}
}

fn content_height(&self, entry_count: usize) -> f32 {
if entry_count == 0 {
return 0.0;
}
let rows = entry_count.div_ceil(self.cols.max(1));
CELL_PAD + rows as f32 * (self.cell_h + CELL_PAD)
}

fn max_scroll(&self, entry_count: usize, win_h: f32, top_offset: f32) -> f32 {
let visible = (win_h - top_offset).max(1.0);
(self.content_height(entry_count) - visible).max(0.0)
}

fn scrolled_rect(&self, i: usize) -> Option<Rect> {
self.rects
.get(i)
.map(|r| Rect::new(r.x, r.y - self.scroll, r.w, r.h))
}

fn center_on_selected(&mut self) {
let row = self.selected / self.cols.max(1);
let row_top = row as f32 * (self.cell_h + CELL_PAD);
self.scroll_target = (row_top - (self.cell_h + CELL_PAD)).max(0.0);
self.scroll = self.scroll_target;
}

pub fn tick(&mut self, entry_count: usize, win_h: f32, top_offset: f32, dt: f32) {
let max = self.max_scroll(entry_count, win_h, top_offset);
self.scroll_target = self.scroll_target.clamp(0.0, max);
let t = 1.0 - (-12.0 * dt).exp();
self.scroll += (self.scroll_target - self.scroll) * t;
if (self.scroll - self.scroll_target).abs() < 0.5 {
self.scroll = self.scroll_target;
}
}

pub fn ensure_selected_visible(&mut self, entry_count: usize, win_h: f32, top_offset: f32) {
let Some(rect) = self.rects.get(self.selected).copied() else {
return;
};
let row_top = rect.y - top_offset;
let row_bottom = row_top + rect.h;
let visible = (win_h - top_offset).max(1.0);
let margin = 8.0;

if row_top - self.scroll_target < margin {
self.scroll_target = (row_top - margin).max(0.0);
} else if row_bottom - self.scroll_target > visible - margin {
self.scroll_target = row_bottom - visible + margin;
}
let max = self.max_scroll(entry_count, win_h, top_offset);
self.scroll_target = self.scroll_target.clamp(0.0, max);
}

pub fn move_selection(&mut self, dx: i32, dy: i32, entry_count: usize) {
if entry_count == 0 {
return;
}
let cols = self.cols.max(1) as i32;
let idx = self.selected as i32 + dx + dy * cols;
self.selected = idx.clamp(0, entry_count as i32 - 1) as usize;
}

pub fn handle_click(&self, mouse: Vec2, entry_count: usize) -> Option<usize> {
for i in (0..entry_count).rev() {
if let Some(rect) = self.scrolled_rect(i) && rect.contains(mouse) {
return Some(i);
}
}
None
}

pub fn hovered_at(&self, mouse: Vec2, entry_count: usize) -> Option<usize> {
self.handle_click(mouse, entry_count)
}

pub fn handle_wheel(&mut self, delta: f32, entry_count: usize, win_h: f32, top_offset: f32) {
let max = self.max_scroll(entry_count, win_h, top_offset);
let step = (self.cell_h + CELL_PAD) * 0.5;
self.scroll_target = (self.scroll_target - delta * step).clamp(0.0, max);
self.snap_selection_to_view(entry_count);
}

/// Moves the selection to the top-left visible cell so keyboard nav
/// continues from what the user is looking at after scrolling.
pub fn snap_selection_to_view(&mut self, entry_count: usize) {
if entry_count == 0 {
return;
}
let cols = self.cols.max(1);
let row = (self.scroll_target / (self.cell_h + CELL_PAD)).floor().max(0.0) as usize;
let idx = (row * cols).min(entry_count - 1);
self.selected = idx;
}

/// Moves selection by a delta and scrolls just enough to reveal it.
pub fn move_and_reveal(&mut self, dx: i32, dy: i32, entry_count: usize, win_h: f32, top_offset: f32) {
self.move_selection(dx, dy, entry_count);
self.ensure_selected_visible(entry_count, win_h, top_offset);
}

fn fill_visible(
&mut self,
gallery: &Gallery,
win_w: f32,
win_h: f32,
top_offset: f32,
) -> bool {
self.update_layout(gallery.len(), win_w, top_offset);
if gallery.is_empty() {
return false;
}

let visible_top = self.scroll - top_offset;
let visible_bottom = self.scroll + (win_h - top_offset).max(1.0);
let row_start = ((visible_top / (self.cell_h + CELL_PAD)).floor().max(0.0)) as usize;
let row_end = (visible_bottom / (self.cell_h + CELL_PAD)).ceil() as usize + 1;
let first = row_start.saturating_mul(self.cols);
let last = ((row_end + 1) * self.cols).min(gallery.len());

let mut produced = false;
let mut budget = DECODE_BUDGET_PER_FRAME;
for i in first..last {
if budget == 0 {
break;
}
let path = &gallery.entries[i];
if self.cache.contains_key(path) {
continue;
}
self.access_counter += 1;
let last_used = self.access_counter;
let thumb = match load_thumb(path) {
Some(tex) => tex,
None => placeholder_texture(),
};
self.cache.insert(path.clone(), Thumb { texture: thumb, last_used });
produced = true;
budget -= 1;
}

if self.cache.len() > CACHE_CAP {
self.evict_lru();
}
produced
}

fn evict_lru(&mut self) {
let mut entries: Vec<(PathBuf, u64)> = self
.cache
.iter()
.map(|(k, v)| (k.clone(), v.last_used))
.collect();
entries.sort_by_key(|(_, used)| *used);
let remove_count = entries.len().saturating_sub(CACHE_CAP);
for (path, _) in entries.into_iter().take(remove_count) {
self.cache.remove(&path);
}
}

pub fn update(
&mut self,
gallery: &Gallery,
win_w: f32,
win_h: f32,
top_offset: f32,
dt: f32,
) {
self.update_layout(gallery.len(), win_w, top_offset);
self.tick(gallery.len(), win_h, top_offset, dt);
self.fill_visible(gallery, win_w, win_h, top_offset);
}

pub fn draw(
&mut self,
gallery: &Gallery,
win_w: f32,
win_h: f32,
top_offset: f32,
font: &Font,
) {
draw_rectangle(0.0, 0.0, win_w, win_h, Color::new(0.06, 0.07, 0.09, 1.0));

if gallery.is_empty() {
let msg = "Không có ảnh trong thư mục";
let dims = measure_text(msg, Some(font), 28, 1.0);
draw_text_ex(
msg,
(win_w - dims.width) / 2.0,
(win_h + dims.height) / 2.0,
TextParams {
font: Some(font),
font_size: 28,
color: Color::new(0.8, 0.83, 0.88, 1.0),
..Default::default()
},
);
return;
}

let mouse = vec2(mouse_position().0, mouse_position().1);
let hovered = self.hovered_at(mouse, gallery.len());
let entry_count = gallery.len();

let visible_top = self.scroll - top_offset;
let visible_bottom = self.scroll + (win_h - top_offset).max(1.0);
let row_start = ((visible_top / (self.cell_h + CELL_PAD)).floor().max(0.0)) as usize;
let row_end = (visible_bottom / (self.cell_h + CELL_PAD)).ceil() as usize + 1;
let first = row_start.saturating_mul(self.cols);
let last = ((row_end + 1) * self.cols).min(entry_count);

for i in first..last {
let Some(rect) = self.scrolled_rect(i) else {
continue;
};
if rect.y + rect.h < top_offset || rect.y > win_h {
continue;
}
let path = &gallery.entries[i];
let is_selected = i == self.selected;
let is_hovered = hovered == Some(i);

let bg = if is_selected {
Color::new(0.14, 0.42, 0.72, 1.0)
} else if is_hovered {
Color::new(0.16, 0.19, 0.24, 1.0)
} else {
Color::new(0.11, 0.13, 0.17, 1.0)
};
draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);

let inner = Rect::new(rect.x + 4.0, rect.y + 4.0, rect.w - 8.0, rect.h - 8.0);
if let Some(thumb) = self.cache.get(path) {
let tex_w = thumb.texture.width();
let tex_h = thumb.texture.height();
if tex_w > 1.0 && tex_h > 1.0 {
let scale = (inner.w / tex_w).min(inner.h / tex_h);
let dw = tex_w * scale;
let dh = tex_h * scale;
let dx = inner.x + (inner.w - dw) / 2.0;
let dy = inner.y + (inner.h - dh) / 2.0;
draw_texture_ex(
&thumb.texture,
dx,
dy,
WHITE,
DrawTextureParams {
dest_size: Some(vec2(dw, dh)),
..Default::default()
},
);
}
} else {
draw_rectangle(inner.x, inner.y, inner.w, inner.h, Color::new(0.08, 0.09, 0.12, 1.0));
let dots = "…";
let dims = measure_text(dots, Some(font), 24, 1.0);
draw_text_ex(
dots,
rect.x + (rect.w - dims.width) / 2.0,
rect.y + rect.h / 2.0 + dims.height * 0.35,
TextParams {
font: Some(font),
font_size: 24,
color: Color::new(0.5, 0.55, 0.62, 1.0),
..Default::default()
},
);
}

if is_selected {
draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.5, WHITE);
}

let name = file_name_of(path);
let label = truncate_label(&name, rect.w - 12.0, font, 13);
let dims = measure_text(&label, Some(font), 13, 1.0);
let label_h = dims.height + 6.0;
draw_rectangle(
rect.x + 3.0,
rect.y + rect.h - label_h - 3.0,
rect.w - 6.0,
label_h,
Color::new(0.0, 0.0, 0.0, 0.55),
);
draw_text_ex(
&label,
rect.x + (rect.w - dims.width) / 2.0,
rect.y + rect.h - 7.0,
TextParams {
font: Some(font),
font_size: 13,
color: Color::new(0.94, 0.96, 0.99, 1.0),
..Default::default()
},
);
}

let hint = format!(
"{} ảnh · [{}] · Enter mở · Esc/` đóng",
entry_count,
self.selected + 1
);
let dims = measure_text(&hint, Some(font), 16, 1.0);
let hx = (win_w - dims.width) / 2.0;
let hy = win_h - 14.0;
draw_rectangle(
hx - 10.0,
hy - dims.height - 4.0,
dims.width + 20.0,
dims.height + 10.0,
Color::new(0.0, 0.0, 0.0, 0.5),
);
draw_text_ex(
&hint,
hx,
hy,
TextParams {
font: Some(font),
font_size: 16,
color: Color::new(0.85, 0.88, 0.93, 1.0),
..Default::default()
},
);

let max = self.max_scroll(entry_count, win_h, top_offset);
if max > 0.0 {
let bar_x = win_w - 8.0;
let track_y = top_offset + 4.0;
let track_h = (win_h - top_offset - 8.0).max(1.0);
let content_h = self.content_height(entry_count);
let thumb_h = (track_h * (track_h / content_h)).clamp(30.0, track_h);
let thumb_y = track_y + (track_h - thumb_h) * (self.scroll / max);
draw_rectangle(bar_x, track_y, 4.0, track_h, Color::new(1.0, 1.0, 1.0, 0.06));
draw_rectangle(bar_x, thumb_y, 4.0, thumb_h, Color::new(1.0, 1.0, 1.0, 0.30));
}
}
}

fn load_thumb(path: &Path) -> Option<Texture2D> {
let img = image::open(path).ok()?;
let (w, h) = (img.width(), img.height());
if w == 0 || h == 0 {
return None;
}
let thumb = if w.max(h) > THUMB_MAX_EDGE {
img.thumbnail(THUMB_MAX_EDGE, THUMB_MAX_EDGE)
} else {
img
};
let rgba = thumb.to_rgba8();
Some(Texture2D::from_rgba8(
rgba.width() as u16,
rgba.height() as u16,
rgba.as_raw(),
))
}

fn placeholder_texture() -> Texture2D {
Texture2D::from_rgba8(1, 1, &[0, 0, 0, 0])
}

fn truncate_label(text: &str, max_w: f32, font: &Font, font_size: u16) -> String {
if measure_text(text, Some(font), font_size, 1.0).width <= max_w {
return text.to_string();
}
let ellipsis = "…";
let ell_w = measure_text(ellipsis, Some(font), font_size, 1.0).width;
let mut out = String::new();
for c in text.chars() {
let mut candidate = out.clone();
candidate.push(c);
candidate.push_str(ellipsis);
if measure_text(&candidate, Some(font), font_size, 1.0).width > max_w - ell_w {
break;
}
out.push(c);
}
out.push_str(ellipsis);
out
}
