use macroquad::prelude::*;

use crate::constants::ZOOM_LERP_SPEED;

/// Zoom / pan state for displaying an image in the window.
pub struct ViewState {
    pub zoom: f32,
    pub zoom_target: f32,
    pub pan: Vec2,
    pub pan_target: Vec2,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            zoom_target: 1.0,
            pan: Vec2::ZERO,
            pan_target: Vec2::ZERO,
        }
    }
}

impl ViewState {
    pub fn reset(&mut self) {
        self.zoom = 1.0;
        self.zoom_target = 1.0;
        self.pan = Vec2::ZERO;
        self.pan_target = Vec2::ZERO;
    }

    /// Smoothly approaches `zoom_target` and `pan_target`.
    pub fn tick_zoom(&mut self, dt: f32) {
        let t = 1.0 - (-ZOOM_LERP_SPEED * dt).exp();
        self.zoom += (self.zoom_target - self.zoom) * t;
        if (self.zoom - self.zoom_target).abs() < 0.0001 {
            self.zoom = self.zoom_target;
        }
        self.pan += (self.pan_target - self.pan) * t;
        if (self.pan - self.pan_target).length_squared() < 0.0001 {
            self.pan = self.pan_target;
        }
    }

    pub fn fit_scale(tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> f32 {
        (win_w / tex_w).min(win_h / tex_h)
    }

    /// Scale at zoom = 1.0: one image pixel maps to one physical screen pixel
    /// when the image fits; only shrinks when the image is larger than the window.
    pub fn base_scale(tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> f32 {
        let native = 1.0 / screen_dpi_scale().max(1.0);
        Self::fit_scale(tex_w, tex_h, win_w, win_h).min(native)
    }

    fn displayed_size(tex_w: f32, tex_h: f32, win_w: f32, win_h: f32, zoom: f32) -> (f32, f32) {
        let scale = Self::base_scale(tex_w, tex_h, win_w, win_h) * zoom;
        (tex_w * scale, tex_h * scale)
    }

    /// Max pan offset so the image still covers the window (0 when fully fitted).
    pub fn pan_limits(tex_w: f32, tex_h: f32, win_w: f32, win_h: f32, zoom: f32) -> (f32, f32) {
        let (disp_w, disp_h) = Self::displayed_size(tex_w, tex_h, win_w, win_h, zoom);
        let max_x = ((disp_w - win_w) / 2.0).max(0.0);
        let max_y = ((disp_h - win_h) / 2.0).max(0.0);
        (max_x, max_y)
    }

    pub fn can_pan(&self, tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> bool {
        let (max_x, max_y) = Self::pan_limits(tex_w, tex_h, win_w, win_h, self.zoom);
        max_x > 0.0 || max_y > 0.0
    }

    /// Keeps `pan` and `pan_target` within the visible bounds.
    pub fn clamp_pan(&mut self, tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) {
        let (max_x, max_y) = Self::pan_limits(tex_w, tex_h, win_w, win_h, self.zoom);
        self.pan.x = if max_x <= 0.0 {
            0.0
        } else {
            self.pan.x.clamp(-max_x, max_x)
        };
        self.pan.y = if max_y <= 0.0 {
            0.0
        } else {
            self.pan.y.clamp(-max_y, max_y)
        };

        let (max_xt, max_yt) = Self::pan_limits(tex_w, tex_h, win_w, win_h, self.zoom_target);
        self.pan_target.x = if max_xt <= 0.0 {
            0.0
        } else {
            self.pan_target.x.clamp(-max_xt, max_xt)
        };
        self.pan_target.y = if max_yt <= 0.0 {
            0.0
        } else {
            self.pan_target.y.clamp(-max_yt, max_yt)
        };
    }

    /// Displayed rectangle of the texture (after pan-clamping).
    pub fn view_rect(&self, tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> Rect {
        let (disp_w, disp_h) = Self::displayed_size(tex_w, tex_h, win_w, win_h, self.zoom);
        let (max_x, max_y) = Self::pan_limits(tex_w, tex_h, win_w, win_h, self.zoom);
        let pan_x = if max_x <= 0.0 {
            0.0
        } else {
            self.pan.x.clamp(-max_x, max_x)
        };
        let pan_y = if max_y <= 0.0 {
            0.0
        } else {
            self.pan.y.clamp(-max_y, max_y)
        };

        Rect {
            x: (win_w / 2.0 + pan_x - disp_w / 2.0).round(),
            y: (win_h / 2.0 + pan_y - disp_h / 2.0).round(),
            w: disp_w.round().max(1.0),
            h: disp_h.round().max(1.0),
        }
    }

    fn view_rect_at_zoom(
        &self,
        zoom: f32,
        tex_w: f32,
        tex_h: f32,
        win_w: f32,
        win_h: f32,
    ) -> Rect {
        let (disp_w, disp_h) = Self::displayed_size(tex_w, tex_h, win_w, win_h, zoom);
        Rect {
            x: win_w / 2.0 + self.pan_target.x - disp_w / 2.0,
            y: win_h / 2.0 + self.pan_target.y - disp_h / 2.0,
            w: disp_w,
            h: disp_h,
        }
    }

    /// Zooms by `factor`, keeping the point under the mouse cursor fixed.
    pub fn zoom_at_mouse(
        &mut self,
        factor: f32,
        mouse: Vec2,
        tex_w: f32,
        tex_h: f32,
        win_w: f32,
        win_h: f32,
    ) {
        let before = self.view_rect_at_zoom(self.zoom_target, tex_w, tex_h, win_w, win_h);
        let base = Self::base_scale(tex_w, tex_h, win_w, win_h);

        let min_zoom = 0.25;
        let max_zoom = (8.0 / base).max(1.0);
        self.zoom_target = (self.zoom_target * factor).clamp(min_zoom, max_zoom);

        let after = self.view_rect_at_zoom(self.zoom_target, tex_w, tex_h, win_w, win_h);
        // pan' = m - center - (m - center - pan_before) * (size_after / size_before)
        let center = vec2(win_w / 2.0, win_h / 2.0);
        let pan_before = self.pan_target;
        let ratio = if before.w > 0.0 {
            after.w / before.w
        } else {
            1.0
        };
        self.pan_target = mouse - center - (mouse - center - pan_before) * ratio;
        self.clamp_pan(tex_w, tex_h, win_w, win_h);
    }
}
