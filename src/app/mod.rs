mod clipboard;
mod crop;
mod input;
mod navigation;
mod render;
mod slideshow;

use std::time::Duration;

use macroquad::prelude::*;

use crate::constants::{ICON_DATA, TOAST_DURATION};
use crate::gallery::Gallery;
use crate::grid::GridMode;
use crate::image_io::{make_checkerboard, LoadedImage};
use crate::platform;
use crate::toolbar::{Toolbar, TOOLBAR_HEIGHT};
use crate::updater::{UpdatePoll, Updater};
use crate::view::ViewState;

pub(crate) struct Toast {
pub(crate) message: String,
pub(crate) is_error: bool,
pub(crate) deadline: f64,
}

pub(crate) const FULLSCREEN_UI_HIDE_SECS: f64 = 2.0;

#[derive(Default)]
pub struct CropState {
pub active: bool,
pub start_pos: Option<Vec2>,
pub current_pos: Vec2,
pub dragging: bool,
pub fixed_w_str: String,
pub fixed_h_str: String,
pub active_input: Option<CropInputFocus>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CropInputFocus {
Width,
Height,
}

pub struct App {
pub(crate) gallery: Gallery,
pub(crate) image: LoadedImage,
pub(crate) texture: Texture2D,
pub(crate) checker: Texture2D,
pub(crate) font: Font,
pub(crate) view: ViewState,
pub(crate) fullscreen: bool,
pub(crate) toast: Option<Toast>,
pub(crate) show_image_info: bool,
pub(crate) show_help: bool,
pub(crate) last_click_time: f64,
pub(crate) hwnd: usize,
pub(crate) was_maximized: bool,
pub(crate) scroll_acc: f32,
/// Last mouse position while left button is held (pixel space).
pub(crate) drag_last: Option<Vec2>,
/// True once the current press moved past the drag threshold.
pub(crate) dragging: bool,
pub(crate) toolbar: Toolbar,
pub(crate) last_mouse_move: f64,
pub(crate) slideshow_active: bool,
pub(crate) slideshow_elapsed: f32,
pub(crate) slideshow_interval: f32,
pub(crate) crop_state: CropState,
pub(crate) updater: Updater,
pub(crate) grid: GridMode,
}

impl App {
pub async fn new(gallery: Gallery) -> Result<Self, String> {
let path = gallery.current().ok_or_else(|| "Gallery is empty".to_string())?;
let image = LoadedImage::load(path)?;
let texture = image.upload_texture()?;

// Request the final window size (image + toolbar strip) immediately, then
// paint one frame with the image before loading the (large) Unicode font.
// This removes the black-window flash on startup and the wrong-sized
// first frame that would otherwise precede the window resize.
let dpi = screen_dpi_scale().max(1.0);
// Window = image + reserved toolbar strip at the top.
let (target_w, target_h) = platform::clamp_window_target(
texture.width() / dpi,
texture.height() / dpi + TOOLBAR_HEIGHT,
dpi,
);
platform::request_window_size(target_w, target_h);

// Wait (short timeout) until the framebuffer matches the requested size.
// Painting into a stale framebuffer before the OS applies the resize makes
// the OS stretch that frame to fill the new window — a visible vertical
// squash on the first frame.
let wait_deadline = get_time() + 0.5;
while (screen_width() - target_w).abs() > 1.0
|| (screen_height() - target_h).abs() > 1.0
{
if get_time() > wait_deadline {
break;
}
next_frame().await;
}

{
// Paint one frame with the image before loading the (large) Unicode
// font. Reserve the same top strip the steady-state viewport uses.
let view = ViewState::default();
let (win_w, win_h) = (screen_width(), screen_height());
let top_offset = TOOLBAR_HEIGHT;
let available_h = (win_h - TOOLBAR_HEIGHT).max(1.0);
let rect =
view.view_rect(texture.width(), texture.height(), win_w, available_h, top_offset);
clear_background(BLACK);
draw_texture_ex(
&texture,
rect.x,
rect.y,
WHITE,
DrawTextureParams {
dest_size: Some(vec2(rect.w, rect.h)),
..Default::default()
},
);
next_frame().await;
}

#[cfg(target_os = "windows")]
let font_path = r"C:\Windows\Fonts\arial.ttf";
#[cfg(target_os = "macos")]
let font_path = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
let font_path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

let font = load_ttf_font(font_path)
.await
.map_err(|e| format!("Failed to load Unicode font '{font_path}': {e}"))?;
Ok(Self {
gallery,
image,
texture,
checker: make_checkerboard(),
font,
view: ViewState::default(),
fullscreen: false,
toast: None,
show_image_info: false,
show_help: false,
last_click_time: 0.0,
hwnd: 0,
was_maximized: false,
scroll_acc: 0.0,
drag_last: None,
dragging: false,
toolbar: Toolbar::new(),
last_mouse_move: 0.0,
slideshow_active: false,
slideshow_elapsed: 0.0,
slideshow_interval: 3.0,
crop_state: CropState::default(),
updater: Updater::new(),
grid: GridMode::default(),
})
}

pub fn texture_size(&self) -> (f32, f32) {
(self.texture.width(), self.texture.height())
}

pub(crate) fn set_toast(&mut self, message: impl Into<String>, is_error: bool) {
self.toast = Some(Toast {
message: message.into(),
is_error,
deadline: get_time() + TOAST_DURATION,
});
}

pub(crate) fn reset_view(&mut self) {
self.view.reset();
}

/// Shows filename [i/N] (WxH) — XemAnh in the window title bar.
/// Appends [zoom%] only when zoom differs from 100%.
pub fn update_title(&mut self) {
if self.hwnd == 0 {
self.hwnd = platform::find_hwnd();
if self.hwnd != 0 {
platform::set_icon(self.hwnd, ICON_DATA);
}
}
let (w, h) = self.image.dimensions();
let zoom = self.view.zoom_percent();
let zoom_label = if zoom != 100 {
format!(" [{}%]", zoom)
} else {
String::new()
};
let title = format!(
"XemAnh — {} ({}×{}){}",
self.gallery.title_label(),
w,
h,
zoom_label
);
platform::set_title(self.hwnd, &title);
}

pub(crate) fn request_window_for_texture(&self) {
let dpi = screen_dpi_scale().max(1.0);
// Window = image + reserved toolbar strip at the top.
let (w, h) = platform::clamp_window_target(
self.texture.width() / dpi,
self.texture.height() / dpi + TOOLBAR_HEIGHT,
dpi,
);
platform::request_window_size(w, h);
}

/// Tracks maximize state; when the user un-maximizes, restore a window size
/// matching the current image.
pub(crate) fn sync_window_state(&mut self) {
if self.hwnd == 0 {
self.hwnd = platform::find_hwnd();
if self.hwnd == 0 {
return;
}
}
let maximized = platform::is_zoomed(self.hwnd);
if self.was_maximized && !maximized && !self.fullscreen {
self.request_window_for_texture();
}
self.was_maximized = maximized;
}

pub fn update(&mut self) -> bool {
match self.updater.poll() {
UpdatePoll::None | UpdatePoll::Downloading => {}
UpdatePoll::Error(message) => self.set_toast(message, true),
UpdatePoll::LaunchInstaller => return false,
}

self.sync_window_state();
self.handle_dropped_files();

if !self.handle_input() {
return false;
}

let dt = get_frame_time();
self.view.tick_zoom(dt);

if self.slideshow_active {
self.slideshow_elapsed += dt;
if self.slideshow_elapsed >= self.slideshow_interval {
self.next_image();
}
}

// Tick animation if current image is animated
if let Some(anim) = self.image.animation_mut() {
let d = Duration::from_secs_f32(dt);
anim.update(d);
let current_tex = anim.current_texture();
self.texture = current_tex.clone();
}

// Update toolbar button positions
let win_w = screen_width();
let win_h = screen_height();
self.toolbar.update_buttons(win_w, win_h);

// Grid mode draws its own full-screen view and skips the single-image path.
if self.grid.active {
self.grid.update(&self.gallery, win_w, win_h, TOOLBAR_HEIGHT, dt);
self.grid.draw(&self.gallery, win_w, win_h, TOOLBAR_HEIGHT, &self.font);
return true;
}

// Clear and draw
clear_background(BLACK);

let win_w = screen_width();
let win_h = screen_height();
let tex_w = self.texture.width();
let tex_h = self.texture.height();

// Fullscreen color background
clear_background(BLACK);

// Reserve the toolbar strip whenever windowed; toggling the toolbar
// never moves the image. In fullscreen the toolbar overlays the image.
let (top_offset, available_h) = if self.fullscreen {
(0.0, win_h)
} else {
(TOOLBAR_HEIGHT, (win_h - TOOLBAR_HEIGHT).max(1.0))
};
let rect = self.view.view_rect(tex_w, tex_h, win_w, available_h, top_offset);

self.draw_checkerboard(rect);
draw_texture_ex(
&self.texture,
rect.x,
rect.y,
WHITE,
DrawTextureParams {
dest_size: Some(vec2(rect.w, rect.h)),
source: None,
rotation: 0.0,
flip_x: false,
flip_y: false,
pivot: None,
},
);

// Draw crop overlay if in crop mode
if self.crop_state.active {
self.draw_crop_overlay(rect);
}

// Draw toolbar on top of everything
self.toolbar.slideshow_active = self.slideshow_active;
self.toolbar.draw(win_w, win_h, &self.font);
self.draw_overlay();

true
}
}
