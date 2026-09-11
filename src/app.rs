use std::path::{Path, PathBuf};
use std::time::Duration;

use macroquad::prelude::*;

use crate::constants::{
    DOUBLE_CLICK_SECS, DRAG_THRESHOLD_PX, ICON_DATA, TOAST_DURATION, WHEEL_DELTA_UNIT,
    ZOOM_MAX_NOTCHES_PER_EVENT, ZOOM_PER_NOTCH,
};
use crate::gallery::{file_name_of, Gallery};
use crate::image_io::{make_checkerboard, LoadedImage, Rot};
use crate::platform;
use crate::toolbar::{Toolbar, ToolbarAction, TOOLBAR_HEIGHT};
use crate::view::ViewState;

struct Toast {
    message: String,
    is_error: bool,
    deadline: f64,
}

const FULLSCREEN_UI_HIDE_SECS: f64 = 2.0;

#[derive(Default)]
pub struct CropState {
    pub active: bool,
    pub start_pos: Option<Vec2>,
    pub current_pos: Vec2,
    pub dragging: bool,
}

impl CropState {
    pub fn reset(&mut self) {
        self.active = false;
        self.start_pos = None;
        self.current_pos = Vec2::ZERO;
        self.dragging = false;
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

pub struct App {
    gallery: Gallery,
    image: LoadedImage,
    texture: Texture2D,
    checker: Texture2D,
    font: Font,
    view: ViewState,
    fullscreen: bool,
    toast: Option<Toast>,
    show_image_info: bool,
    last_click_time: f64,
    hwnd: usize,
    was_maximized: bool,
    scroll_acc: f32,
    /// Last mouse position while left button is held (pixel space).
    drag_last: Option<Vec2>,
    /// True once the current press moved past the drag threshold.
    dragging: bool,
    toolbar: Toolbar,
    last_mouse_move: f64,
    crop_state: CropState,
}

impl App {
    pub async fn new(gallery: Gallery) -> Result<Self, String> {
        let path = gallery.current().ok_or_else(|| "Gallery is empty".to_string())?;
        let image = LoadedImage::load(path)?;
        let texture = image.upload_texture()?;
        #[cfg(target_os = "windows")]
        let font_path = "C:\\Windows\\Fonts\\arial.ttf";
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
            last_click_time: 0.0,
            hwnd: 0,
            was_maximized: false,
            scroll_acc: 0.0,
            drag_last: None,
            dragging: false,
            toolbar: Toolbar::new(),
            last_mouse_move: 0.0,
            crop_state: CropState::default(),
        })
    }

    pub fn texture_size(&self) -> (f32, f32) {
        (self.texture.width(), self.texture.height())
    }

    fn set_toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some(Toast {
            message: message.into(),
            is_error,
            deadline: get_time() + TOAST_DURATION,
        });
    }

    fn reset_view(&mut self) {
        self.view.reset();
    }

    /// Shows `filename [i/N] - XemAnh` in the window title bar.
    pub fn update_title(&mut self) {
        if self.hwnd == 0 {
            self.hwnd = platform::find_hwnd();
            if self.hwnd != 0 {
                platform::set_icon(self.hwnd, ICON_DATA);
            }
        }
        let title = format!("XemAnh — {}", self.gallery.title_label());
        platform::set_title(self.hwnd, &title);
    }

    fn request_window_for_texture(&self) {
        let dpi = screen_dpi_scale().max(1.0);
        let (w, h) = platform::clamp_window_target(
            self.texture.width() / dpi,
            self.texture.height() / dpi,
            dpi,
        );
        platform::request_window_size(w, h);
    }

    /// Tracks maximize state; when the user un-maximizes, restore a window size
    /// matching the current image.
    fn sync_window_state(&mut self) {
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

    /// Loads the image at `index`, resizes the window accordingly and resets the view.
    fn load_index(&mut self, index: usize) {
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
                        self.texture.height() / dpi,
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
    fn handle_dropped_files(&mut self) {
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

    fn next_image(&mut self) {
        if let Some(next) = self.gallery.next_index() {
            self.load_index(next);
        }
    }

    fn prev_image(&mut self) {
        if let Some(prev) = self.gallery.prev_index() {
            self.load_index(prev);
        }
    }

    fn toggle_fullscreen(&mut self) {
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

    fn paste_from_clipboard(&mut self) {
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

    fn copy_current(&mut self) {
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

    fn rotate_and_save(&mut self, rot: Rot) {
        if rot != Rot::None {
            self.image.rotate(rot);
            match self.image.upload_texture() {
                Ok(texture) => {
                    self.texture = texture;
                    self.reset_view();
                    if !self.fullscreen && !platform::is_zoomed(self.hwnd) {
                        self.request_window_for_texture();
                    }
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

    fn generate_crop_filename(original_path: &Path) -> PathBuf {
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

    fn apply_crop(&mut self) {
        let Some(crop_rect) = self.crop_state.get_selection_rect() else {
            self.set_toast("Vùng chọn cắt quá nhỏ", true);
            return;
        };

        let win_w = screen_width();
        let win_h = screen_height();
        let tex_w = self.texture.width();
        let tex_h = self.texture.height();
        let (top_offset, available_h) = if self.fullscreen {
            (0.0, win_h)
        } else if self.toolbar.visible {
            (TOOLBAR_HEIGHT, (win_h - TOOLBAR_HEIGHT).max(1.0))
        } else {
            (0.0, win_h)
        };
        let img_rect = self.view.view_rect(tex_w, tex_h, win_w, available_h, top_offset);

        // Convert screen crop rect to texture pixel space
        let rel_x = (crop_rect.x - img_rect.x) / img_rect.w;
        let rel_y = (crop_rect.y - img_rect.y) / img_rect.h;
        let rel_w = crop_rect.w / img_rect.w;
        let rel_h = crop_rect.h / img_rect.h;

        let x = (rel_x * tex_w).round().max(0.0) as u32;
        let y = (rel_y * tex_h).round().max(0.0) as u32;
        let w = (rel_w * tex_w).round() as u32;
        let h = (rel_h * tex_h).round() as u32;

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

    /// Deletes the current image to the Recycle Bin and shows the next one.
    fn delete_current(&mut self) {
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

    /// Handles all input. Returns false when the app should quit.
    fn handle_input(&mut self) -> bool {
        self.update_fullscreen_ui();

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
        if is_key_pressed(KeyCode::Home) && !self.gallery.is_empty() {
            self.load_index(0);
        }
        if is_key_pressed(KeyCode::End) && !self.gallery.is_empty() {
            self.load_index(self.gallery.len() - 1);
        }

        // Rotate & save
        if is_key_pressed(KeyCode::I) {
            self.show_image_info = !self.show_image_info;
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

        // Reset view: 0 / Numpad 0 or double-click (click = no drag)
        if is_key_pressed(KeyCode::Key0) || is_key_pressed(KeyCode::Kp0) {
            self.reset_view();
            self.set_toast("View reset", false);
        }

        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        let win_w = screen_width();
        let win_h = screen_height();
        let tex_w = self.texture.width();
        let tex_h = self.texture.height();

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
                    ToolbarAction::ZoomIn => {
                        let factor = ZOOM_PER_NOTCH;
                        self.view.zoom_at_mouse(factor, mouse, tex_w, tex_h, win_w, win_h);
                        let percent = (self.view.zoom_target * 100.0).round() as i32;
                        self.set_toast(format!("{percent}%"), false);
                    }
                    ToolbarAction::ZoomOut => {
                        let factor = 1.0 / ZOOM_PER_NOTCH;
                        self.view.zoom_at_mouse(factor, mouse, tex_w, tex_h, win_w, win_h);
                        let percent = (self.view.zoom_target * 100.0).round() as i32;
                        self.set_toast(format!("{percent}%"), false);
                    }
                    ToolbarAction::Crop => {
                        self.crop_state.active = !self.crop_state.active;
                        if self.crop_state.active {
                            self.set_toast("Chế độ cắt: Kéo chuột để chọn, Enter để cắt, Esc để hủy", false);
                        } else {
                            self.crop_state.reset();
                        }
                    }
                    ToolbarAction::ResetView => {
                        self.reset_view();
                        self.set_toast("View reset", false);
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
            let (top_offset, available_h) = if self.fullscreen {
                (0.0, win_h)
            } else if self.toolbar.visible {
                (TOOLBAR_HEIGHT, (win_h - TOOLBAR_HEIGHT).max(1.0))
            } else {
                (0.0, win_h)
            };
            let img_rect = self.view.view_rect(tex_w, tex_h, win_w, available_h, top_offset);

            if is_mouse_button_pressed(MouseButton::Left) {
                let clamped_x = mouse.x.clamp(img_rect.x, img_rect.x + img_rect.w);
                let clamped_y = mouse.y.clamp(img_rect.y, img_rect.y + img_rect.h);
                self.crop_state.start_pos = Some(vec2(clamped_x, clamped_y));
                self.crop_state.current_pos = vec2(clamped_x, clamped_y);
                self.crop_state.dragging = true;
            }

            if is_mouse_button_down(MouseButton::Left) && self.crop_state.dragging {
                let clamped_x = mouse.x.clamp(img_rect.x, img_rect.x + img_rect.w);
                let clamped_y = mouse.y.clamp(img_rect.y, img_rect.y + img_rect.h);
                self.crop_state.current_pos = vec2(clamped_x, clamped_y);
            }

            if is_mouse_button_released(MouseButton::Left) {
                self.crop_state.dragging = false;
            }
        } else {
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
                        if self.view.can_pan(tex_w, tex_h, win_w, win_h) {
                            self.view.pan += delta;
                            self.view.pan_target += delta;
                            self.view.clamp_pan(tex_w, tex_h, win_w, win_h);
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
                    self.last_click_time = 0.0;
                } else {
                    self.last_click_time = now;
                }
            }
        }

        // Mouse wheel: zoom at cursor position
        let wheel = mouse_wheel();
        if wheel.1 != 0.0 {
            self.scroll_acc += wheel.1 * WHEEL_DELTA_UNIT;
            let notches = (self.scroll_acc / WHEEL_DELTA_UNIT).round();
            if notches != 0.0 {
                let clamped = notches.clamp(-ZOOM_MAX_NOTCHES_PER_EVENT, ZOOM_MAX_NOTCHES_PER_EVENT);
                let factor = ZOOM_PER_NOTCH.powf(clamped);
                self.view
                    .zoom_at_mouse(factor, mouse, tex_w, tex_h, win_w, win_h);
                self.scroll_acc -= clamped * WHEEL_DELTA_UNIT;
            }
        } else {
            self.scroll_acc = 0.0;
        }

        true
    }

    fn draw_crop_overlay(&self, img_rect: Rect) {
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
    }

    fn draw_overlay(&self) {
        if let Some(toast) = &self.toast {
            if get_time() < toast.deadline {
                let msg = &toast.message;
                let dims = measure_text(msg, Some(&self.font), 24, 1.0);
                let padding = 20.0;
                let margin = 20.0;
                let w = dims.width + padding * 2.0;
                let h = dims.height + padding * 2.0;
                let x = (screen_width() - w) / 2.0;
                let y = screen_height() - h - margin;
                draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.65));
                draw_text_ex(
                    msg,
                    x + padding,
                    y + padding + dims.height * 0.8,
                    TextParams {
                        font: Some(&self.font),
                        font_size: 24,
                        color: if toast.is_error { RED } else { WHITE },
                        ..Default::default()
                    },
                );
            }
        }

        if self.show_image_info {
            let path = self.gallery.current_path();
            let name = file_name_of(&path);
            let width = self.texture.width();
            let height = self.texture.height();

            let lines = [
                name,
                format!("Kích thước: {:.0} × {:.0} px", width, height),
            ];
            let font_size = 24u16;
            let line_height = 36.0;
            let padding = 24.0;

            let max_width = lines
                .iter()
                .map(|line| measure_text(line, Some(&self.font), font_size, 1.0).width)
                .fold(0.0, f32::max);

            let panel_w = max_width + padding * 2.0;
            let panel_h = line_height * lines.len() as f32 + padding * 2.0;
            let panel_x = (screen_width() - panel_w) / 2.0;
            let panel_y = (screen_height() - panel_h) / 2.0;

            draw_rectangle(
                panel_x,
                panel_y,
                panel_w,
                panel_h,
                Color::new(0.0, 0.0, 0.0, 0.78),
            );

            for (index, line) in lines.iter().enumerate() {
                draw_text_ex(
                    line,
                    panel_x + padding,
                    panel_y + padding + line_height * (index as f32 + 0.78),
                    TextParams {
                        font: Some(&self.font),
                        font_size,
                        color: WHITE,
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn draw_checkerboard(&self, region: Rect) {
        if region.w <= 0.0 || region.h <= 0.0 {
            return;
        }

        let scale_x = region.w / self.texture.width().max(1.0);
        let tile = (16.0 * scale_x).max(2.0);
        let cell = tile / 2.0;

        let mut y = region.y;
        let max_y = region.y + region.h;
        let max_x = region.x + region.w;

        while y < max_y {
            let cur_h = (max_y - y).min(cell);
            let mut x = region.x;
            while x < max_x {
                let cur_w = (max_x - x).min(cell);
                draw_texture_ex(
                    &self.checker,
                    x,
                    y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(cur_w.max(0.1), cur_h.max(0.1))),
                        source: Some(Rect::new(0.0, 0.0, 16.0, 16.0)),
                        ..Default::default()
                    },
                );
                x += cell;
            }
            y += cell;
        }
    }

    pub fn update(&mut self) -> bool {
        self.sync_window_state();
        self.handle_dropped_files();

        if !self.handle_input() {
            return false;
        }

        let dt = get_frame_time();
        self.view.tick_zoom(dt);

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

        // Clear and draw
        clear_background(BLACK);

        let win_w = screen_width();
        let win_h = screen_height();
        let tex_w = self.texture.width();
        let tex_h = self.texture.height();

        // Fullscreen color background
        clear_background(BLACK);

        // In fullscreen the image always owns the entire screen; the toolbar is
        // drawn on top instead of reducing the image viewport.
        let (top_offset, available_h) = if self.fullscreen {
            (0.0, win_h)
        } else if self.toolbar.visible {
            (TOOLBAR_HEIGHT, (win_h - TOOLBAR_HEIGHT).max(1.0))
        } else {
            (0.0, win_h)
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
        self.toolbar.draw(win_w, win_h);
        self.draw_overlay();

        true
    }
}
