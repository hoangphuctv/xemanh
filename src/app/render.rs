use macroquad::prelude::*;

use crate::gallery::file_name_of;

use super::App;

impl App {
pub(crate) fn draw_overlay(&self) {
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
let format = path
.extension()
.and_then(|ext| ext.to_str())
.unwrap_or("Không rõ")
.to_uppercase();
let file_size = std::fs::metadata(&path)
.map(|metadata| {
let bytes = metadata.len();
if bytes >= 1024 * 1024 {
format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
} else if bytes >= 1024 {
format!("{:.1} KB", bytes as f64 / 1024.0)
} else {
format!("{} B", bytes)
}
})
.unwrap_or_else(|_| "Không rõ".to_string());

let color_type = format!("{:?}", self.image.color_type());
let mut lines = vec![
name,
format!("Kích thước: {:.0} × {:.0} px", width, height),
format!("Định dạng: {}", format),
format!("Dung lượng: {}", file_size),
format!("Màu: {}", color_type),
];

if let Some((current_frame, frame_count, total_duration)) = self.image.animation_info() {
lines.push(format!("Frame: {} / {}", current_frame, frame_count));
lines.push(format!(
"Thời lượng: {:.2} giây",
total_duration.as_secs_f64()
));
}
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
self.draw_help();
}

pub(crate) fn draw_checkerboard(&self, region: Rect) {
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

pub(crate) fn draw_help(&self) {
if !self.show_help {
return;
}
let items = [
"H - Ẩn/hiện bảng trợ giúp",
"← / → / ↑ / ↓ / PageUp / PageDown - Ảnh trước / ảnh sau",
"Home / 1 / End - Ảnh đầu / ảnh cuối",
"Space - Bật/tắt toàn màn hình",
"Esc - Thoát toàn màn hình / hủy cắt / thoát",
"C - Bật/tắt chế độ cắt ảnh",
"Enter - Áp dụng cắt (khi đang cắt)",
"R - Xoay phải (Shift+R: xoay trái) & lưu",
"Ctrl+S - Lưu ảnh hiện tại",
"Ctrl+C - Sao chép ảnh vào clipboard",
"Ctrl+V - Dán ảnh từ clipboard",
"Delete - Xóa ảnh (thùng rác)",
"0 - Đặt lại khung nhìn",
"I - Ẩn/hiện thông tin ảnh",
"S - Đổi thứ tự sắp xếp",
"F - Mở thư mục chứa ảnh",
"- Bật/tắt chế độ lưới (xem toàn thư mục)", "Trong lưới: mũi tên di chuyển, Enter mở, Esc/ đóng",
];
let fs = 22u16;
let lh = 34.0;
let pad = 24.0;
let max_w = items.iter().map(|l| measure_text(l, Some(&self.font), fs, 1.0).width).fold(0.0, f32::max);
let w = max_w + pad * 2.0;
let h = lh * items.len() as f32 + pad * 2.0 + 40.0;
let x = (screen_width() - w) / 2.0;
let y = (screen_height() - h) / 2.0;
draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.82));
draw_text_ex("PHÍM TẮT", x + pad, y + pad + 24.0, TextParams { font: Some(&self.font), font_size: 26, color: YELLOW, ..Default::default() });
for (i, it) in items.iter().enumerate() {
draw_text_ex(it, x + pad, y + pad + 50.0 + lh * i as f32, TextParams { font: Some(&self.font), font_size: fs, color: WHITE, ..Default::default() });
}
}
}
