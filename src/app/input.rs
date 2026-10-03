use macroquad::prelude::*;

use crate::constants::{
DOUBLE_CLICK_SECS, DRAG_THRESHOLD_PX, WHEEL_DELTA_UNIT, ZOOM_MAX_NOTCHES_PER_EVENT,
ZOOM_PER_NOTCH,
};
use crate::image_io::Rot;
use crate::platform;
use crate::toolbar::{ToolbarAction, TOOLBAR_HEIGHT};

use super::CropInputFocus;
use super::{App, FULLSCREEN_UI_HIDE_SECS};

impl App {
pub(crate) fn toggle_fullscreen(&mut self) {
self.fullscreen = !self.fullscreen;
platform::set_fullscreen(self.fullscreen);
self.toolbar.visible = true;
self.toolbar.user_hidden = false;
self.last_mouse_move = get_time();

if !self.fullscreen {
self.request_window_for_texture();
self.set_toast("Windowed", false);
} else {
self.set_toast("Fullscreen", false);
}
}

fn update_fullscreen_ui(&mut self) {
if !self.fullscreen {
return;
}

// Do not auto-reveal on the same frame as a click. This lets the
// persistent show/hide control work normally even while the toolbar
// is hidden: clicking it reveals the toolbar instead of immediately
// revealing and hiding it again.
if mouse_delta_position().length_squared() > 0.0 && !is_mouse_button_down(MouseButton::Left) {
if !self.toolbar.user_hidden {
self.toolbar.visible = true;
self.last_mouse_move = get_time();
}
} else if self.toolbar.visible
&& get_time() - self.last_mouse_move >= FULLSCREEN_UI_HIDE_SECS
{
self.toolbar.visible = false;
}
}

pub(crate) fn open_grid(&mut self) {
self.grid.open(self.gallery.index);
self.set_toast("Grid mode - Enter to open", false);
}

pub(crate) fn handle_grid_input(&mut self) -> bool {
let _win_w = screen_width();
let win_h = screen_height();
let top_offset = TOOLBAR_HEIGHT;
let entry_count = self.gallery.len();
if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::GraveAccent) {
self.grid.close();
self.set_toast("Grid closed", false);
return true;
}
if is_key_pressed(KeyCode::Enter) {
let idx = self.grid.selected;
self.grid.close();
if !self.gallery.is_empty() {
self.load_index(idx);
}
return true;
}
if is_key_pressed(KeyCode::Right) {
self.grid.move_and_reveal(1, 0, entry_count, win_h, top_offset);
}
if is_key_pressed(KeyCode::Left) {
self.grid.move_and_reveal(-1, 0, entry_count, win_h, top_offset);
}
if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::PageDown) {
self.grid.move_and_reveal(0, 1, entry_count, win_h, top_offset);
}
if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::PageUp) {
self.grid.move_and_reveal(0, -1, entry_count, win_h, top_offset);
}
if is_key_pressed(KeyCode::Home) {
self.grid.selected = 0;
self.grid.ensure_selected_visible(entry_count, win_h, top_offset);
}
if is_key_pressed(KeyCode::End) && !self.gallery.is_empty() {
self.grid.selected = entry_count - 1;
self.grid.ensure_selected_visible(entry_count, win_h, top_offset);
}
let wheel = mouse_wheel();
if wheel.1 != 0.0 {
self.grid.handle_wheel(wheel.1, entry_count, win_h, top_offset);
}
if is_mouse_button_pressed(MouseButton::Left) {
let mouse = vec2(mouse_position().0, mouse_position().1);
if let Some(idx) = self.grid.handle_click(mouse, entry_count) {
if idx == self.grid.selected {
self.grid.close();
self.load_index(idx);
} else {
self.grid.selected = idx;
}
}
}
true
}

/// Handles all input. Returns false when the app should quit.
pub(crate) fn handle_input(&mut self) -> bool {
self.update_fullscreen_ui();

// Grid mode owns all input while active.
if self.grid.active {
return self.handle_grid_input();
}

// Toggle grid view with the backtick key.
if is_key_pressed(KeyCode::GraveAccent) {
self.open_grid();
return true;
}

// Esc exits Crop mode first, then fullscreen, then quits app.
if is_key_pressed(KeyCode::Escape) {
if self.crop_state.active {
self.crop_state.reset();
self.set_toast("Đã hủy cắt ảnh", false);
return true;
}
if self.fullscreen {
self.toggle_fullscreen();
} else {
return false;
}
}

// Enter applies crop if crop mode is active
if self.crop_state.active && is_key_pressed(KeyCode::Enter) {
self.apply_crop();
return true;
}

// Toggle crop mode with 'C' (when Ctrl is not held)
if is_key_pressed(KeyCode::C)
&& !is_key_down(KeyCode::LeftControl)
&& !is_key_down(KeyCode::RightControl)
{
self.crop_state.active = !self.crop_state.active;
if self.crop_state.active {
self.set_toast("Chế độ cắt: Kéo chuột để chọn, Enter để cắt, Esc để hủy", false);
} else {
self.crop_state.reset();
}
}

// Fullscreen toggle (Space)
if is_key_pressed(KeyCode::Space) {
self.toggle_fullscreen();
}

// Navigation
if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::PageDown) || is_key_pressed(KeyCode::Down) {
self.next_image();
}
if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::PageUp) || is_key_pressed(KeyCode::Up) {
self.prev_image();
}
if (is_key_pressed(KeyCode::Home) || is_key_pressed(KeyCode::Key1)) && !self.gallery.is_empty() {
self.load_index(0);
}
if is_key_pressed(KeyCode::End) && !self.gallery.is_empty() {
self.load_index(self.gallery.len() - 1);
}

if self.crop_state.active {
if is_key_pressed(KeyCode::Tab) {
self.crop_state.active_input = Some(match self.crop_state.active_input {
Some(CropInputFocus::Width) => CropInputFocus::Height,
_ => CropInputFocus::Width,
});
return true;
}

if is_key_pressed(KeyCode::Backspace) {
match self.crop_state.active_input {
Some(CropInputFocus::Width) => { self.crop_state.fixed_w_str.pop(); }
Some(CropInputFocus::Height) => { self.crop_state.fixed_h_str.pop(); }
None => {}
}
return true;
}

while let Some(ch) = get_char_pressed() {
if ch.is_ascii_digit() {
match self.crop_state.active_input {
Some(CropInputFocus::Width) => self.crop_state.fixed_w_str.push(ch),
Some(CropInputFocus::Height) => self.crop_state.fixed_h_str.push(ch),
None => {}
}
}
}
}

// Open containing folder (F)
if is_key_pressed(KeyCode::F) {
let path = self.gallery.current_path();
if let Err(err) = platform::reveal_in_file_manager(&path) {
self.set_toast(err, true);
}
}

// Toggle image info overlay (I)
if is_key_pressed(KeyCode::I) {
self.show_image_info = !self.show_image_info;
}

// Cycle sort order (S) - không dùng khi Ctrl đang giữ
if is_key_pressed(KeyCode::S)
&& !is_key_down(KeyCode::LeftControl)
&& !is_key_down(KeyCode::RightControl)
{
let mode = self.gallery.sort_mode.next();
self.gallery.apply_sort(mode);
let n = self.gallery.index;
self.load_index(n);
self.set_toast(format!("Sắp xếp: {}", mode.label()), false);
}

if is_key_pressed(KeyCode::R) {
let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
self.rotate_and_save(if shift { Rot::Ccw } else { Rot::Cw });
}
// Manual save
if is_key_pressed(KeyCode::S)
&& (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
{
self.rotate_and_save(Rot::None);
}

// Copy current image to clipboard
if is_key_pressed(KeyCode::C)
&& (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
{
self.copy_current();
}

// Paste image from clipboard (Ctrl+V)
if is_key_pressed(KeyCode::V)
&& (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
{
self.paste_from_clipboard();
}

// Delete current image (to Recycle Bin)
if is_key_pressed(KeyCode::Delete) {
self.delete_current();
}

// Toggle help overlay (H)
if is_key_pressed(KeyCode::H) {
self.show_help = !self.show_help;
}

// Reset view: 0 / Numpad 0 or double-click (click = no drag)
if is_key_pressed(KeyCode::Key0) || is_key_pressed(KeyCode::Kp0) {
self.reset_view();
self.set_toast("View reset", false);
self.update_title();
}

let (mx, my) = mouse_position();
let mouse = vec2(mx, my);
let win_w = screen_width();
let win_h = screen_height();
let tex_w = self.texture.width();
let tex_h = self.texture.height();
// Image viewport: reserve the toolbar strip when windowed. Used for
// zoom/pan math so it matches what update() actually draws.
let (top_offset, available_h) = if self.fullscreen {
(0.0, win_h)
} else {
(TOOLBAR_HEIGHT, (win_h - TOOLBAR_HEIGHT).max(1.0))
};

// Update toolbar hover
self.toolbar.update_hover(mouse);

// Check toolbar clicks
if is_mouse_button_pressed(MouseButton::Left) {
if self.toolbar.handle_toggle_click(mouse) {
if self.fullscreen {
self.last_mouse_move = get_time();
}
return true;
}

if let Some(action) = self.toolbar.handle_click(mouse) {
match action {
ToolbarAction::Prev => self.prev_image(),
ToolbarAction::Next => self.next_image(),
ToolbarAction::Play => self.toggle_slideshow(),
ToolbarAction::Interval => self.cycle_slideshow_interval(),
ToolbarAction::ZoomIn => {
let factor = ZOOM_PER_NOTCH;
self.view.zoom_at_mouse(
factor,
mouse,
tex_w,
tex_h,
win_w,
available_h,
top_offset,
);
let percent = (self.view.zoom_target * 100.0).round() as i32;
self.set_toast(format!("{percent}%"), false);
self.update_title();
}
ToolbarAction::ZoomOut => {
let factor = 1.0 / ZOOM_PER_NOTCH;
self.view.zoom_at_mouse(
factor,
mouse,
tex_w,
tex_h,
win_w,
available_h,
top_offset,
);
let percent = (self.view.zoom_target * 100.0).round() as i32;
self.set_toast(format!("{percent}%"), false);
self.update_title();
}
ToolbarAction::Crop => {
self.crop_state.active = !self.crop_state.active;
if self.crop_state.active {
self.crop_state.active_input = Some(CropInputFocus::Width);
self.set_toast("Crop: nhập W/H (px), click/drag để đặt khung, Enter để cắt, Esc để hủy", false);
} else {
self.crop_state.reset();
}
}
ToolbarAction::ResetView => {
self.reset_view();
self.set_toast("View reset", false);
self.update_title();
}
}
return true;
}
}

if is_mouse_button_pressed(MouseButton::Middle) {
self.reset_view();
}

// Handle Mouse Input for Crop Mode vs Pan Mode
if self.crop_state.active {
self.handle_crop_mouse(mouse, win_w, win_h, tex_w, tex_h, top_offset, available_h);
} else {
self.handle_pan_mouse(mouse, win_w, win_h, tex_w, tex_h, available_h);
}

// Mouse wheel: zoom at cursor position
let wheel = mouse_wheel();
if wheel.1 != 0.0 {
self.scroll_acc += wheel.1 * WHEEL_DELTA_UNIT;
let notches = (self.scroll_acc / WHEEL_DELTA_UNIT).round();
if notches != 0.0 {
let clamped = notches.clamp(-ZOOM_MAX_NOTCHES_PER_EVENT, ZOOM_MAX_NOTCHES_PER_EVENT);
let factor = ZOOM_PER_NOTCH.powf(clamped);
self.view.zoom_at_mouse(
factor,
mouse,
tex_w,
tex_h,
win_w,
available_h,
top_offset,
);
self.scroll_acc -= clamped * WHEEL_DELTA_UNIT;
self.update_title();
}
} else {
self.scroll_acc = 0.0;
}

true
}

fn handle_crop_mouse(
&mut self,
mouse: Vec2,
win_w: f32,
_win_h: f32,
tex_w: f32,
tex_h: f32,
top_offset: f32,
available_h: f32,
) {
// Match the update() viewport so crop coordinates line up.
let img_rect = self.view.view_rect(tex_w, tex_h, win_w, available_h, top_offset);

let (width_rect, height_rect) = self.crop_input_rects(img_rect);

if is_mouse_button_pressed(MouseButton::Left) {
if width_rect.contains(mouse) {
self.crop_state.active_input = Some(CropInputFocus::Width);
} else if height_rect.contains(mouse) {
self.crop_state.active_input = Some(CropInputFocus::Height);
} else {
let clamped_x = mouse.x.clamp(img_rect.x, img_rect.x + img_rect.w);
let clamped_y = mouse.y.clamp(img_rect.y, img_rect.y + img_rect.h);

if let Some((fixed_w, fixed_h)) = self.crop_state.get_fixed_size() {
let screen_w = fixed_w as f32 * img_rect.w / tex_w;
let screen_h = fixed_h as f32 * img_rect.h / tex_h;
if screen_w > img_rect.w || screen_h > img_rect.h {
self.set_toast("Kích thước crop lớn hơn ảnh gốc", true);
self.crop_state.dragging = false;
} else {
let left = (clamped_x - screen_w / 2.0)
.clamp(img_rect.x, img_rect.x + img_rect.w - screen_w);
let top = (clamped_y - screen_h / 2.0)
.clamp(img_rect.y, img_rect.y + img_rect.h - screen_h);
self.crop_state.start_pos = Some(vec2(left, top));
self.crop_state.current_pos = vec2(left + screen_w, top + screen_h);
self.crop_state.dragging = true;
}
} else {
self.crop_state.start_pos = Some(vec2(clamped_x, clamped_y));
self.crop_state.current_pos = vec2(clamped_x, clamped_y);
self.crop_state.dragging = true;
}
}
}

if is_mouse_button_down(MouseButton::Left) && self.crop_state.dragging {
let clamped_x = mouse.x.clamp(img_rect.x, img_rect.x + img_rect.w);
let clamped_y = mouse.y.clamp(img_rect.y, img_rect.y + img_rect.h);
if let Some((fixed_w, fixed_h)) = self.crop_state.get_fixed_size() {
let screen_w = fixed_w as f32 * img_rect.w / tex_w;
let screen_h = fixed_h as f32 * img_rect.h / tex_h;
let left = (clamped_x - screen_w / 2.0)
.clamp(img_rect.x, img_rect.x + img_rect.w - screen_w);
let top = (clamped_y - screen_h / 2.0)
.clamp(img_rect.y, img_rect.y + img_rect.h - screen_h);
self.crop_state.start_pos = Some(vec2(left, top));
self.crop_state.current_pos = vec2(left + screen_w, top + screen_h);
} else {
self.crop_state.current_pos = vec2(clamped_x, clamped_y);
}
}

if is_mouse_button_released(MouseButton::Left) {
self.crop_state.dragging = false;
}
}

fn handle_pan_mouse(
&mut self,
mouse: Vec2,
win_w: f32,
_win_h: f32,
tex_w: f32,
tex_h: f32,
available_h: f32,
) {
if is_mouse_button_pressed(MouseButton::Left) {
self.drag_last = Some(mouse);
self.dragging = false;
}

// Drag to pan when the image is larger than the window
if is_mouse_button_down(MouseButton::Left) {
if let Some(last) = self.drag_last {
let delta = mouse - last;
if delta.length_squared() > 0.0 {
if delta.length() >= DRAG_THRESHOLD_PX {
self.dragging = true;
}
if self.view.can_pan(tex_w, tex_h, win_w, available_h) {
self.view.pan += delta;
self.view.pan_target += delta;
self.view.clamp_pan(tex_w, tex_h, win_w, available_h);
}
self.drag_last = Some(mouse);
}
}
} else {
self.drag_last = None;
}

// Double-click to reset view
if is_mouse_button_pressed(MouseButton::Left) && !self.dragging {
let now = get_time();
if now - self.last_click_time < DOUBLE_CLICK_SECS {
self.reset_view();
self.set_toast("View reset", false);
self.update_title();
self.last_click_time = 0.0;
} else {
self.last_click_time = now;
}
}
}
}
