use macroquad::prelude::*;

use crate::constants::{DOUBLE_CLICK_SECS, ICON_DATA, TOAST_DURATION};
use crate::gallery::{file_name_of, Gallery};
use crate::image_io::{load_texture, make_checkerboard, save_transformed, Rot};
use crate::platform;
use crate::view::ViewState;

struct Toast {
    message: String,
    is_error: bool,
    deadline: f64,
}

pub struct App {
    gallery: Gallery,
    texture: Texture2D,
    checker: Texture2D,
    view: ViewState,
    fullscreen: bool,
    toast: Option<Toast>,
    last_click_time: f64,
    hwnd: usize,
    was_maximized: bool,
    scroll_acc: f32,
}

impl App {
    pub fn new(gallery: Gallery) -> Result<Self, String> {
        let path = gallery.current().ok_or_else(|| "Gallery is empty".to_string())?;
        let texture = load_texture(path)?;
        Ok(Self {
            gallery,
            texture,
            checker: make_checkerboard(),
            view: ViewState::default(),
            fullscreen: false,
            toast: None,
            last_click_time: 0.0,
            hwnd: 0,
            was_maximized: false,
            scroll_acc: 0.0,
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
        let title = format!("{} - XemAnh", self.gallery.title_label());
        platform::set_title(self.hwnd, &title);
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
            let (w, h) = platform::clamp_window_target(self.texture.width(), self.texture.height());
            request_new_screen_size(w, h);
        }
        self.was_maximized = maximized;
    }

    /// Loads the image at `index`, resizes the window accordingly and resets the view.
    fn load_index(&mut self, index: usize) {
        let path = self.gallery.entries[index].clone();
        match load_texture(&path) {
            Ok(texture) => {
                self.gallery.index = index;
                self.texture = texture;
                self.reset_view();
                // Never resize while fullscreen or maximized: it breaks the window
                // state and desynchronizes the GL viewport from the window.
                if !self.fullscreen && !platform::is_zoomed(self.hwnd) {
                    let (w, h) =
                        platform::clamp_window_target(self.texture.width(), self.texture.height());
                    request_new_screen_size(w, h);
                }
                self.update_title();
            }
            Err(err) => self.set_toast(format!("[{}] {}", index + 1, err), true),
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
        macroquad::miniquad::window::set_fullscreen(self.fullscreen);
        if !self.fullscreen {
            // Restore a sane window size for the current image after exiting fullscreen.
            let (w, h) = platform::clamp_window_target(self.texture.width(), self.texture.height());
            request_new_screen_size(w, h);
        }
    }

    fn rotate_and_save(&mut self, rot: Rot) {
        let path = self.gallery.current_path();
        match save_transformed(&path, rot) {
            Ok(()) => match load_texture(&path) {
                Ok(texture) => {
                    self.texture = texture;
                    self.reset_view();
                    self.set_toast(format!("Saved {}", file_name_of(&path)), false);
                }
                Err(err) => self.set_toast(format!("Saved but reload failed: {err}"), true),
            },
            Err(err) => self.set_toast(err, true),
        }
    }

    /// Deletes the current image to the Recycle Bin and shows the next one.
    fn delete_current(&mut self) {
        if self.gallery.is_empty() {
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
        if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::PageDown) {
            self.next_image();
        }
        if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::PageUp) {
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

        // Delete current image (to Recycle Bin)
        if is_key_pressed(KeyCode::Delete) {
            self.delete_current();
        }

        // Reset view: 0 or double-click
        if is_key_pressed(KeyCode::Key0) {
            self.reset_view();
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let now = get_time();
            if now - self.last_click_time < DOUBLE_CLICK_SECS {
                self.reset_view();
            }
            self.last_click_time = now;
        }

        // Zoom with scroll wheel (accumulate until threshold)
        let (_, wheel_y) = mouse_wheel();
        self.scroll_acc += wheel_y;
        let threshold = 1.0;
        if self.scroll_acc.abs() >= threshold {
            let steps = self.scroll_acc.trunc() as i32;
            self.scroll_acc -= steps as f32;
            let factor = 1.08f32.powf(steps as f32);
            let (mx, my) = mouse_position();
            self.view.zoom_at_mouse(
                factor,
                vec2(mx, my),
                self.texture.width(),
                self.texture.height(),
                screen_width(),
                screen_height(),
            );
        }

        // Pan with left mouse drag
        if is_mouse_button_down(MouseButton::Left) {
            self.view.pan += mouse_delta_position();
        }

        true
    }

    fn draw_overlay(&self) {
        let font_size = 18.0;
        let pad = 6.0;
        let margin = 10.0;

        // Toast (errors / save confirmations) at the bottom-left corner
        if let Some(toast) = &self.toast {
            if get_time() < toast.deadline {
                let dims = measure_text(&toast.message, None, font_size as u16, 1.0);
                let w = dims.width + pad * 2.0;
                let h = font_size + pad * 2.0;
                let y = screen_height() - margin - h;
                draw_rectangle(margin, y, w, h, Color::new(0.0, 0.0, 0.0, 0.65));
                draw_text(
                    &toast.message,
                    margin + pad,
                    y + pad + font_size * 0.8,
                    font_size,
                    if toast.is_error {
                        Color::new(1.0, 0.45, 0.45, 1.0)
                    } else {
                        Color::new(0.55, 1.0, 0.55, 1.0)
                    },
                );
            }
        }
    }

    /// Tiles the checkerboard pattern across `region`, clipped exactly to it.
    fn draw_checkerboard(&self, region: Rect) {
        let ts = self.checker.width();
        if ts <= 0.0 || region.w <= 0.0 || region.h <= 0.0 {
            return;
        }
        let end_x = region.x + region.w;
        let end_y = region.y + region.h;
        let mut ty = (region.y / ts).floor() * ts;
        while ty < end_y {
            let mut tx = (region.x / ts).floor() * ts;
            while tx < end_x {
                let vx = tx.max(region.x);
                let vy = ty.max(region.y);
                let vr = (tx + ts).min(end_x);
                let vb = (ty + ts).min(end_y);
                draw_texture_ex(
                    &self.checker,
                    vx,
                    vy,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect {
                            x: vx - tx,
                            y: vy - ty,
                            w: vr - vx,
                            h: vb - vy,
                        }),
                        ..Default::default()
                    },
                );
                tx += ts;
            }
            ty += ts;
        }
    }

    pub fn update(&mut self) -> bool {
        self.sync_window_state();
        if !self.handle_input() {
            return false;
        }

        self.view.tick_zoom(get_frame_time());

        clear_background(BLACK);

        let rect = self.view.view_rect(
            self.texture.width(),
            self.texture.height(),
            screen_width(),
            screen_height(),
        );
        self.draw_checkerboard(rect);
        draw_texture_ex(
            &self.texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                ..Default::default()
            },
        );

        self.draw_overlay();
        true
    }
}
