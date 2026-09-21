use macroquad::prelude::*;

pub const TOOLBAR_HEIGHT: f32 = 44.0;

pub struct Toolbar {
    pub visible: bool,
    pub user_hidden: bool,
    pub buttons: Vec<ToolbarButton>,
    pub toggle_rect: Rect,
    pub hovered: Option<usize>,
    pub toggle_hovered: bool,
    pub slideshow_active: bool,
}

#[derive(Clone, Copy)]
pub struct ToolbarButton {
    pub action: ToolbarAction,
    pub rect: Rect,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ToolbarAction {
    Prev,
    Next,
    Play,
    Interval,
    Crop,
    ZoomIn,
    ZoomOut,
    ResetView,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            visible: true,
            user_hidden: false,
            buttons: Vec::new(),
            toggle_rect: Rect::default(),
            hovered: None,
            toggle_hovered: false,
            slideshow_active: false,
        }
    }

    pub fn update_buttons(&mut self, win_w: f32, _win_h: f32) {
        let btn_h = 30.0;
        let toggle_w = 34.0;
        let toggle_margin = 8.0;
        let y = (TOOLBAR_HEIGHT - btn_h) / 2.0;

        self.toggle_rect = Rect::new(win_w - toggle_w - toggle_margin, y, toggle_w, btn_h);

        let count = 8;
        let spacing = 7.0;
        let button_w = 38.0;
        let total_w = count as f32 * button_w + (count - 1) as f32 * spacing;
        let start_x = (win_w - total_w) / 2.0;

        self.buttons.clear();

        if !self.visible {
            return;
        }

        let actions = [
            ToolbarAction::Crop,
            ToolbarAction::Prev,
            ToolbarAction::Next,
            ToolbarAction::Play,
            ToolbarAction::Interval,
            ToolbarAction::ZoomOut,
            ToolbarAction::ResetView,
            ToolbarAction::ZoomIn,
        ];

        for (i, &action) in actions.iter().enumerate() {
            let x = start_x + i as f32 * (button_w + spacing);
            self.buttons.push(ToolbarButton {
                action,
                rect: Rect::new(x, y, button_w, btn_h),
            });
        }
    }

    pub fn update_hover(&mut self, mouse_pos: Vec2) {
        self.toggle_hovered = self.toggle_rect.contains(mouse_pos);
        self.hovered = None;

        if !self.visible {
            return;
        }

        for (i, btn) in self.buttons.iter().enumerate() {
            if btn.rect.contains(mouse_pos) {
                self.hovered = Some(i);
                break;
            }
        }
    }

    pub fn handle_toggle_click(&mut self, mouse_pos: Vec2) -> bool {
        if self.toggle_rect.contains(mouse_pos) {
            self.visible = !self.visible;
            self.user_hidden = !self.visible;
            true
        } else {
            false
        }
    }

    pub fn handle_click(&self, mouse_pos: Vec2) -> Option<ToolbarAction> {
        if !self.visible {
            return None;
        }

        for btn in &self.buttons {
            if btn.rect.contains(mouse_pos) {
                return Some(btn.action);
            }
        }
        None
    }

    pub fn draw(&self, win_w: f32, _win_h: f32, font: &Font) {
        let btn_color = Color::new(0.14, 0.16, 0.20, 0.96);
        let btn_hover_color = Color::new(0.10, 0.45, 0.67, 1.0);
        let border_color = Color::new(0.72, 0.82, 0.92, 0.18);
        let icon_color = Color::new(0.91, 0.95, 1.0, 1.0);

        if !self.visible {
            let color = if self.toggle_hovered {
                btn_hover_color
            } else {
                btn_color
            };
            draw_button(self.toggle_rect, color, border_color);
            draw_vertical_chevron(self.toggle_rect.center(), false, icon_color);
            return;
        }

        let bg_color = Color::new(0.055, 0.065, 0.09, 0.94);
        draw_rectangle(0.0, 0.0, win_w, TOOLBAR_HEIGHT, bg_color);
        draw_line(
            0.0,
            TOOLBAR_HEIGHT,
            win_w,
            TOOLBAR_HEIGHT,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.1),
        );

        for (i, btn) in self.buttons.iter().enumerate() {
            let color = if Some(i) == self.hovered {
                btn_hover_color
            } else {
                btn_color
            };
            draw_button(btn.rect, color, border_color);
            if btn.action == ToolbarAction::Play && self.slideshow_active {
                draw_pause_icon(btn.rect.center(), icon_color);
            } else {
                draw_action_icon(btn.action, btn.rect.center(), icon_color);
            }

            if Some(i) == self.hovered {
                draw_tooltip(action_name(btn.action), btn.rect, font);
            }
        }

        // Draw toggle button (^ to hide)
        let t_color = if self.toggle_hovered {
            btn_hover_color
        } else {
            btn_color
        };
        draw_button(self.toggle_rect, t_color, border_color);
        draw_vertical_chevron(self.toggle_rect.center(), true, icon_color);
    }
}

fn draw_button(rect: Rect, fill: Color, border: Color) {
    draw_rounded_rect(rect, 7.0, border);
    draw_rounded_rect(
        Rect::new(rect.x + 1.0, rect.y + 1.0, rect.w - 2.0, rect.h - 2.0),
        6.0,
        fill,
    );
}

fn draw_rounded_rect(rect: Rect, radius: f32, color: Color) {
    draw_rectangle(
        rect.x + radius,
        rect.y,
        rect.w - radius * 2.0,
        rect.h,
        color,
    );
    draw_rectangle(
        rect.x,
        rect.y + radius,
        rect.w,
        rect.h - radius * 2.0,
        color,
    );
    for (x, y) in [
        (rect.x + radius, rect.y + radius),
        (rect.x + rect.w - radius, rect.y + radius),
        (rect.x + radius, rect.y + rect.h - radius),
        (rect.x + rect.w - radius, rect.y + rect.h - radius),
    ] {
        draw_circle(x, y, radius, color);
    }
}

fn draw_action_icon(action: ToolbarAction, center: Vec2, color: Color) {
    match action {
        ToolbarAction::Prev => draw_chevron(center, true, color),
        ToolbarAction::Next => draw_chevron(center, false, color),
        ToolbarAction::Play => draw_play_icon(center, color),
        ToolbarAction::Interval => draw_interval_icon(center, color),
        ToolbarAction::Crop => draw_crop_icon(center, color),
        ToolbarAction::ZoomIn => draw_zoom_icon(center, true, color),
        ToolbarAction::ZoomOut => draw_zoom_icon(center, false, color),
        ToolbarAction::ResetView => draw_reset_label(center, color),
    }
}

fn draw_pause_icon(center: Vec2, color: Color) {
    draw_rectangle(center.x - 6.0, center.y - 7.0, 4.0, 14.0, color);
    draw_rectangle(center.x + 2.0, center.y - 7.0, 4.0, 14.0, color);
}

fn draw_play_icon(center: Vec2, color: Color) {
    draw_triangle(
        Vec2::new(center.x - 5.0, center.y - 7.0),
        Vec2::new(center.x - 5.0, center.y + 7.0),
        Vec2::new(center.x + 7.0, center.y),
        color,
    );
}

fn draw_interval_icon(center: Vec2, color: Color) {
    draw_circle_lines(center.x, center.y, 8.0, 2.0, color);
    draw_line(center.x, center.y, center.x, center.y - 5.0, 2.0, color);
    draw_line(center.x, center.y, center.x + 4.0, center.y + 3.0, 2.0, color);
}

fn draw_chevron(center: Vec2, left: bool, color: Color) {
    let direction = if left { 1.0 } else { -1.0 };
    draw_line(
        center.x + direction * 3.0,
        center.y - 7.0,
        center.x - direction * 4.0,
        center.y,
        2.3,
        color,
    );
    draw_line(
        center.x - direction * 4.0,
        center.y,
        center.x + direction * 3.0,
        center.y + 7.0,
        2.3,
        color,
    );
}

fn draw_vertical_chevron(center: Vec2, up: bool, color: Color) {
    let direction = if up { 1.0 } else { -1.0 };
    draw_line(
        center.x - 7.0,
        center.y + direction * 3.0,
        center.x,
        center.y - direction * 4.0,
        2.3,
        color,
    );
    draw_line(
        center.x,
        center.y - direction * 4.0,
        center.x + 7.0,
        center.y + direction * 3.0,
        2.3,
        color,
    );
}

fn draw_crop_icon(c: Vec2, color: Color) {
    let s = 7.0;
    draw_line(c.x - s, c.y - 4.0, c.x + 4.0, c.y - 4.0, 2.0, color);
    draw_line(c.x - 4.0, c.y - s, c.x - 4.0, c.y + 4.0, 2.0, color);
    draw_line(c.x + s, c.y + 4.0, c.x - 4.0, c.y + 4.0, 2.0, color);
    draw_line(c.x + 4.0, c.y + s, c.x + 4.0, c.y - 4.0, 2.0, color);
}

fn draw_zoom_icon(c: Vec2, plus: bool, color: Color) {
    draw_circle_lines(c.x - 2.5, c.y - 2.5, 6.0, 2.0, color);
    draw_line(c.x + 2.0, c.y + 2.0, c.x + 7.5, c.y + 7.5, 2.2, color);
    draw_line(c.x - 5.5, c.y - 2.5, c.x + 0.5, c.y - 2.5, 1.7, color);
    if plus {
        draw_line(c.x - 2.5, c.y - 5.5, c.x - 2.5, c.y + 0.5, 1.7, color);
    }
}

fn draw_reset_label(c: Vec2, color: Color) {
    const LABEL: &str = "100%";
    let font_size = 13.0;
    let dims = measure_text(LABEL, None, font_size as u16, 1.0);
    draw_text(
        LABEL,
        c.x - dims.width / 2.0,
        c.y + dims.height * 0.35,
        font_size,
        color,
    );
}

fn action_name(action: ToolbarAction) -> &'static str {
    match action {
        ToolbarAction::Prev => "Ảnh trước",
        ToolbarAction::Next => "Ảnh tiếp",
        ToolbarAction::Play => "Slideshow",
        ToolbarAction::Interval => "Khoảng thời gian",
        ToolbarAction::Crop => "Cắt ảnh",
        ToolbarAction::ZoomIn => "Phóng to",
        ToolbarAction::ZoomOut => "Thu nhỏ",
        ToolbarAction::ResetView => "Đặt lại tỷ lệ",
    }
}

fn draw_tooltip(label: &str, button: Rect, font: &Font) {
    let font_size = 14.0;
    let dims = measure_text(label, Some(font), font_size as u16, 1.0);
    let width = dims.width + 14.0;
    let x = button.x + (button.w - width) / 2.0;
    let y = TOOLBAR_HEIGHT + 7.0;
    draw_rectangle(x, y, width, 23.0, Color::new(0.03, 0.04, 0.06, 0.92));
    draw_text_ex(
        label,
        x + 7.0,
        y + 16.5,
        TextParams {
            font: Some(font),
            font_size: font_size as u16,
            color: Color::new(0.94, 0.97, 1.0, 1.0),
            ..Default::default()
        },
    );
}
