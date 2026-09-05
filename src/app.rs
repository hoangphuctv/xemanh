use macroquad::prelude::*;

use crate::constants::{
    DOUBLE_CLICK_SECS, DRAG_THRESHOLD_PX, ICON_DATA, TOAST_DURATION, WHEEL_DELTA_UNIT,
    ZOOM_MAX_NOTCHES_PER_EVENT, ZOOM_PER_NOTCH, TOOLBAR_AUTO_HIDE_DELAY,
};
use crate::gallery::{file_name_of, Gallery};
use crate::image_io::{make_checkerboard, LoadedImage, Rot};
use crate::platform;
use crate::toolbar::{Toolbar, ToolbarAction};
use crate::view::ViewState;

struct Toast {
    message: String,
    is_error: bool,
    deadline: f64,
}

pub struct App {
    gallery: Gallery,
    image: LoadedImage,
    texture: Texture2D,
    checker: Texture2D,
    view: ViewState,
    fullscreen: bool,
    toast: Option<Toast>,
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
}

impl App {
    pub fn new(gallery: Gallery) -> Result<Self, String> {
        let path = gallery.current().ok_or_else(|| "Gallery is empty".to_string())?;
        let image = LoadedImage::load(path)?;
        let texture = image.upload_texture()?;
        Ok(Self {
            gallery,
            image,
            texture,
            checker: make_checkerboard(),
            view: ViewState::default(),
            fullscreen: false,
            toast: None,
            last_click_time: 0.0,
            hwnd: 0,
            was_maximized: false,
            scroll_acc: 0.0,
            drag_last: None,
            dragging: false,
            toolbar: Toolbar::new(),
            last_mouse_move: 0.0,
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
                self.gallery.index = index;
                self.image = image;
                self.texture = texture;
                self.reset_view();
                // Never resize while fullscreen or maximized: it breaks the window
                // state and desynchronizes the GL viewport from the window.
                if !self.fullscreen && !platform::is_zoomed(self.hwnd) {
                    let current_w = screen_width();
                    let current_h = screen_height();
                    let dpi = screen_dpi_scale().max(1.0);
                    let (w_img, h_img) = platform::clamp_window_target(
                        self.texture.width() / dpi,
                        self.texture.height() / dpi,
                        dpi,
                    );

                    // If the current window is already larger than or equal to the target size for the new image,
                    // we don't need to shrink or change the window size at all.
                    if current_w < w_img || current_h < h_img {
                        // The new image target is larger in at least one dimension.
                        // We grow the window to fit it, but we clamp the target to the screen limits.
                        let mut w_target = w_img.max(current_w);
                        let mut h_target = h_img.max(current_h);

                        // Clamp to screen limits
                        let (clamped_w, clamped_h) =
                            platform::clamp_window_target(w_target, h_target, dpi);

                        // If the user had already manually stretched the window to be larger than clamped limits,
                        // we preserve their manually set size instead of forcing it to shrink.
                        w_target = clamped_w.max(current_w);
                        h_target = clamped_h.max(current_h);

                        platform::request_window_size(w_target, h_target);
                    }
                }
                self.update_title();
                if self.fullscreen {
                    self.set_toast(self.gallery.title_label(), false);
                }
                // Show toolbar briefly when image changes
                self.toolbar.visible = true;
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
        if !self.fullscreen {
            self.request_window_for_texture();
            self.set_toast("Windowed", false);
        } else {
            self.set_toast("Fullscreen", false);
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
                    // Stay at the same position: it now points at the next image.
                    let next = self.gallery.index;
                    self.load_index(next);
                    self.set_toast(format!("Deleted {name}"), false);
                }
            }
            Err(err) => self.set_toast(err, true),
        }
    }

    /// Handles all input. Returns false when the app should quit.
    fn handle_input(&mut self) -> bool {
        // Esc exits fullscreen first; a second Esc quits the app.
        if is_key_pressed(KeyCode::Escape) {
            if self.fullscreen {
                self.toggle_fullscreen();
            } else {
                return false;
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

        // Track mouse movement for toolbar auto-hide
        let mouse_moved = is_mouse_button_down(MouseButton::Left) || 
                          (mouse != vec2(mx, my));
        if mouse_moved {
            self.last_mouse_move = get_time();
            self.toolbar.visible = true;
        }

        // Update toolbar hover
        self.toolbar.update_hover(mouse);

        // Check toolbar clicks
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(action) = self.toolbar.handle_click(mouse) {
                match action {
                    ToolbarAction::Prev => self.prev_image(),
                    ToolbarAction::Next => self.next_image(),
                }
                // Don't process as image click
                return true;
            }
        }

        if is_mouse_button_pressed(MouseButton::Middle) {
            self.reset_view();
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            self.drag_last = Some(mouse);
            self.dragging = false;
        }

        // Drag to pan when the image is larger than the window (typically after zoom).
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

        // Auto-hide toolbar after inactivity
        if self.toolbar.visible && get_time() - self.last_mouse_move > TOOLBAR_AUTO_HIDE_DELAY {
            self.toolbar.visible = false;
        }

        // Show toolbar on any mouse movement
        if get_time() - self.last_mouse_move < 0.5 {
            self.toolbar.visible = true;
        }

        true
    }

    fn draw_overlay(&self) {
        if let Some(toast) = &self.toast {
            if get_time() < toast.deadline {
                let msg = &toast.message;
                let dims = measure_text(msg, None, 24, 1.0);
                let padding = 20.0;
                let margin = 20.0;
                let w = dims.width + padding * 2.0;
                let h = dims.height + padding * 2.0;
                let x = (screen_width() - w) / 2.0;
                let y = screen_height() - h - margin;
                draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.65));
                draw_text(
                    msg,
                    x + padding,
                    y + padding + dims.height * 0.8,
                    24.0,
                    if toast.is_error { RED } else { WHITE },
                );
            }
        }
    }

    fn draw_checkerboard(&self, region: Rect) {
        // Only draw checkerboard if the image has transparency
        if !self.image.has_transparency() {
            return;
        }

        let mut x = region.x;
        let mut y = region.y;
        let tile = 16.0;
        let mut odd = false;
        while y < region.y + region.h {
            while x < region.x + region.w {
                let tile_rect = Rect::new(x, y, tile, tile);
                draw_texture_ex(
                    &self.checker,
                    tile_rect.x,
                    tile_rect.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(tile_rect.w, tile_rect.h)),
                        ..Default::default()
                    },
                );
                x += tile;
            }
            odd = !odd;
            x = region.x - if odd { 0.0 } else { tile / 2.0 };
            y += tile;
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

        // Draw checkerboard, texture, overlay
        let rect = self.view.view_rect(tex_w, tex_h, win_w, win_h);
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

        // Draw toolbar on top of everything
        self.toolbar.draw(win_w, win_h);
        self.draw_overlay();

        true
    }
}
