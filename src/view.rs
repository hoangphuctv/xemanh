use macroquad::prelude::*;

/// Zoom / pan state for displaying an image in the window.
pub struct ViewState {
    pub zoom: f32,
    pub zoom_target: f32,
    pub pan: Vec2,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            zoom_target: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl ViewState {
    pub fn reset(&mut self) {
        self.zoom_target = 1.0;
        self.pan = Vec2::ZERO;
    }

    /// Smoothly approaches `zoom_target`.
    pub fn tick_zoom(&mut self, dt: f32) {
        let lerp_speed = 12.0;
        let t = 1.0 - (-lerp_speed * dt).exp();
        self.zoom += (self.zoom_target - self.zoom) * t;
        if (self.zoom - self.zoom_target).abs() < 0.0001 {
            self.zoom = self.zoom_target;
        }
    }

    pub fn fit_scale(tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> f32 {
        (win_w / tex_w).min(win_h / tex_h)
    }

    /// Scale at zoom = 1.0: native size when the image fits the window,
    /// scaled down only when the image is larger than the window.
    pub fn base_scale(tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> f32 {
        Self::fit_scale(tex_w, tex_h, win_w, win_h).min(1.0)
    }

    /// Displayed rectangle of the texture (after pan-clamping).
    pub fn view_rect(&self, tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> Rect {
        let scale = Self::base_scale(tex_w, tex_h, win_w, win_h) * self.zoom;
        let disp_w = tex_w * scale;
        let disp_h = tex_h * scale;

        let mut pan = self.pan;
        let max_x = (disp_w - win_w) / 2.0;
        let max_y = (disp_h - win_h) / 2.0;
        pan.x = if max_x <= 0.0 {
            0.0
        } else {
            pan.x.clamp(-max_x, max_x)
        };
        pan.y = if max_y <= 0.0 {
            0.0
        } else {
            pan.y.clamp(-max_y, max_y)
        };

        Rect {
            x: win_w / 2.0 + pan.x - disp_w / 2.0,
            y: win_h / 2.0 + pan.y - disp_h / 2.0,
            w: disp_w,
            h: disp_h,
        }
    }

    fn view_rect_unclamped(&self, tex_w: f32, tex_h: f32, win_w: f32, win_h: f32) -> Rect {
        let scale = Self::base_scale(tex_w, tex_h, win_w, win_h) * self.zoom;
        let disp_w = tex_w * scale;
        let disp_h = tex_h * scale;
        Rect {
            x: win_w / 2.0 + self.pan.x - disp_w / 2.0,
            y: win_h / 2.0 + self.pan.y - disp_h / 2.0,
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
        let before = self.view_rect(tex_w, tex_h, win_w, win_h);
        let base = Self::base_scale(tex_w, tex_h, win_w, win_h);

        let min_zoom = 0.25;
        let max_zoom = (8.0 / base).max(1.0);
        self.zoom_target = (self.zoom_target * factor).clamp(min_zoom, max_zoom);

        let after = self.view_rect_unclamped(tex_w, tex_h, win_w, win_h);
        // pan' = m - center - (m - center - pan_before) * (size_after / size_before)
        let center = vec2(win_w / 2.0, win_h / 2.0);
        let pan_before = self.pan;
        let ratio = after.w / before.w;
        self.pan = mouse - center - (mouse - center - pan_before) * ratio;
    }
}
