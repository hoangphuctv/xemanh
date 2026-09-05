use macroquad::prelude::*;

pub struct Toolbar {
    pub visible: bool,
    pub buttons: Vec<ToolbarButton>,
    pub hovered: Option<usize>,
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
            hovered: None,
        }
    }

    pub fn update_buttons(&mut self, win_w: f32, _win_h: f32) {
        let count = 5;
        let spacing = 6.0;
        let button_w = 56.0;
        let button_h = 36.0;
        let total_w = count as f32 * button_w + (count - 1) as f32 * spacing;
        let start_x = (win_w - total_w) / 2.0;
        let y = 12.0;

        self.buttons.clear();

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
                rect: Rect::new(x, y, button_w, button_h),
            });
        }
    }

    pub fn draw(&self, win_w: f32, _win_h: f32) {
        if !self.visible {
            return;
        }

        let bg_color = Color::new(0.0, 0.0, 0.0, 0.6);
        let btn_color = Color::new(0.3, 0.3, 0.3, 0.8);
        let btn_hover_color = Color::new(0.5, 0.5, 0.5, 0.9);
        let text_color = WHITE;

        let bar_h = 56.0;
        draw_rectangle(0.0, 0.0, win_w, bar_h, bg_color);

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
                Color::new(1.0, 1.0, 1.0, 0.3),
            );

            let font_size = 20.0;
            let text_dims = measure_text(btn.label, None, font_size as u16, 1.0);
            let tx = btn.rect.x + (btn.rect.w - text_dims.width) / 2.0;
            let ty = btn.rect.y + (btn.rect.h - text_dims.height) / 2.0 + font_size * 0.3;
            draw_text(btn.label, tx, ty, font_size, text_color);
        }
    }

    pub fn handle_click(&mut self, mouse: Vec2) -> Option<ToolbarAction> {
        for btn in &self.buttons {
            if btn.rect.contains(mouse) {
                return Some(btn.action);
            }
        }
        None
    }

    pub fn update_hover(&mut self, mouse: Vec2) {
        self.hovered = self.buttons.iter().position(|btn| btn.rect.contains(mouse));
    }
}
