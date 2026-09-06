use macroquad::prelude::*;

pub const TOOLBAR_HEIGHT: f32 = 44.0;

pub struct Toolbar {
    pub visible: bool,
    pub buttons: Vec<ToolbarButton>,
    pub toggle_rect: Rect,
    pub hovered: Option<usize>,
    pub toggle_hovered: bool,
}

#[derive(Clone, Copy)]
pub struct ToolbarButton {
    pub label: &'static str,
    pub action: ToolbarAction,
    pub rect: Rect,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ToolbarAction {
    Prev,
    Next,
    ZoomIn,
    ZoomOut,
    ResetView,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            visible: true,
            buttons: Vec::new(),
            toggle_rect: Rect::default(),
            hovered: None,
            toggle_hovered: false,
        }
    }

    pub fn update_buttons(&mut self, win_w: f32, _win_h: f32) {
        let btn_h = 28.0;
        let toggle_w = 32.0;
        let toggle_margin = 8.0;
        let y = (TOOLBAR_HEIGHT - btn_h) / 2.0;

        self.toggle_rect = Rect::new(
            win_w - toggle_w - toggle_margin,
            y,
            toggle_w,
            btn_h,
        );

        let count = 5;
        let spacing = 6.0;
        let button_w = 56.0;
        let total_w = count as f32 * button_w + (count - 1) as f32 * spacing;
        let start_x = (win_w - total_w) / 2.0;

        self.buttons.clear();

        if !self.visible {
            return;
        }

        let actions = [
            ToolbarAction::Prev,
            ToolbarAction::Next,
            ToolbarAction::ZoomOut,
            ToolbarAction::ResetView,
            ToolbarAction::ZoomIn,
        ];
        let labels = ["<", ">", "-", "100%", "+"];

        for (i, (&action, &label)) in actions.iter().zip(labels.iter()).enumerate() {
            let x = start_x + i as f32 * (button_w + spacing);
            self.buttons.push(ToolbarButton {
                label,
                action,
                rect: Rect::new(x, y, button_w, btn_h),
            });
        }
    }

    pub fn draw(&self, win_w: f32, _win_h: f32) {
        let btn_color = Color::new(0.2, 0.2, 0.2, 0.85);
        let btn_hover_color = Color::new(0.4, 0.4, 0.4, 0.95);
        let border_color = Color::new(1.0, 1.0, 1.0, 0.2);
        let text_color = WHITE;

        if !self.visible {
            let color = if self.toggle_hovered {
                btn_hover_color
            } else {
                btn_color
            };
            draw_rectangle(
                self.toggle_rect.x,
                self.toggle_rect.y,
                self.toggle_rect.w,
                self.toggle_rect.h,
                color,
            );
            draw_rectangle_lines(
                self.toggle_rect.x,
                self.toggle_rect.y,
                self.toggle_rect.w,
                self.toggle_rect.h,
                1.0,
                border_color,
            );
            let font_size = 14.0;
            let label = "v";
            let dims = measure_text(label, None, font_size as u16, 1.0);
            let tx = self.toggle_rect.x + (self.toggle_rect.w - dims.width) / 2.0;
            let ty = self.toggle_rect.y + (self.toggle_rect.h - dims.height) / 2.0 + font_size * 0.35;
            draw_text(label, tx, ty, font_size, text_color);
            return;
        }

        let bg_color = Color::new(0.1, 0.1, 0.1, 0.92);
        draw_rectangle(0.0, 0.0, win_w, TOOLBAR_HEIGHT, bg_color);
        draw_line(0.0, TOOLBAR_HEIGHT, win_w, TOOLBAR_HEIGHT, 1.0, Color::new(1.0, 1.0, 1.0, 0.1));

        for (i, btn) in self.buttons.iter().enumerate() {
            let color = if Some(i) == self.hovered {
                btn_hover_color
            } else {
                btn_color
            };
            draw_rectangle(btn.rect.x, btn.rect.y, btn.rect.w, btn.rect.h, color);
            draw_rectangle_lines(
                btn.rect.x,
                btn.rect.y,
                btn.rect.w,
                btn.rect.h,
                1.0,
                border_color,
            );

            let font_size = 18.0;
            let text_dims = measure_text(btn.label, None, font_size as u16, 1.0);
            let tx = btn.rect.x + (btn.rect.w - text_dims.width) / 2.0;
            let ty = btn.rect.y + (btn.rect.h - text_dims.height) / 2.0 + font_size * 0.35;
            draw_text(btn.label, tx, ty, font_size, text_color);
        }

        // Draw toggle button (^ to hide)
        let t_color = if self.toggle_hovered {
            btn_hover_color
        } else {
            btn_color
        };
        draw_rectangle(
            self.toggle_rect.x,
            self.toggle_rect.y,
            self.toggle_rect.w,
            self.toggle_rect.h,
            t_color,
        );
        draw_rectangle_lines(
            self.toggle_rect.x,
            self.toggle_rect.y,
            self.toggle_rect.w,
            self.toggle_rect.h,
            1.0,
            border_color,
        );
        let font_size = 14.0;
        let label = "^";
        let dims = measure_text(label, None, font_size as u16, 1.0);
        let tx = self.toggle_rect.x + (self.toggle_rect.w - dims.width) / 2.0;
        let ty = self.toggle_rect.y + (self.toggle_rect.h - dims.height) / 2.0 + font_size * 0.35;
        draw_text(label, tx, ty, font_size, text_color);
    }

    pub fn handle_toggle_click(&mut self, mouse: Vec2) -> bool {
        if self.toggle_rect.contains(mouse) {
            self.visible = !self.visible;
            true
        } else {
            false
        }
    }

    pub fn handle_click(&mut self, mouse: Vec2) -> Option<ToolbarAction> {
        if !self.visible {
            return None;
        }
        for btn in &self.buttons {
            if btn.rect.contains(mouse) {
                return Some(btn.action);
            }
        }
        None
    }

    pub fn update_hover(&mut self, mouse: Vec2) {
        self.toggle_hovered = self.toggle_rect.contains(mouse);
        if self.visible {
            self.hovered = self.buttons.iter().position(|btn| btn.rect.contains(mouse));
        } else {
            self.hovered = None;
        }
    }
}
