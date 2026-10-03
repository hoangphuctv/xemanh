use super::App;

impl App {
pub(crate) fn toggle_slideshow(&mut self) {
if self.gallery.next_index().is_none() {
self.slideshow_active = false;
self.slideshow_elapsed = 0.0;
self.set_toast("Cần ít nhất 2 ảnh để Slideshow", true);
return;
}

self.slideshow_active = !self.slideshow_active;
self.slideshow_elapsed = 0.0;

if self.slideshow_active {
self.set_toast(
format!("Slideshow: {} giây", self.slideshow_interval),
false,
);
} else {
self.set_toast("Slideshow dừng", false);
}
}

pub(crate) fn cycle_slideshow_interval(&mut self) {
const INTERVALS: [f32; 5] = [1.0, 2.0, 3.0, 5.0, 10.0];

let current = INTERVALS
.iter()
.position(|&value| value == self.slideshow_interval)
.unwrap_or(2);
self.slideshow_interval = INTERVALS[(current + 1) % INTERVALS.len()];
self.slideshow_elapsed = 0.0;
self.set_toast(
format!("Khoảng thời gian: {} giây", self.slideshow_interval),
false,
);
}
}
