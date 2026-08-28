pub const IMAGE_EXTENSIONS: [&str; 9] = [
    "jpg", "jpeg", "jpe", "jfif", "png", "bmp", "dib", "gif", "tga",
];
pub const JPEG_SAVE_QUALITY: u8 = 100;
pub const TOAST_DURATION: f64 = 1.2;
pub const DOUBLE_CLICK_SECS: f64 = 0.35;
pub const CHECKER_TILE_PX: u16 = 16;
pub const ICON_DATA: &[u8] = include_bytes!("../assets/xemanh.ico");

/// Scale multiplier applied per mouse-wheel notch (~5.5%).
pub const ZOOM_PER_NOTCH: f32 = 1.055;
/// Pixel movement before a press counts as a drag (not a double-click).
pub const DRAG_THRESHOLD_PX: f32 = 4.0;
/// How quickly displayed zoom catches up to the target (lower = calmer).
pub const ZOOM_LERP_SPEED: f32 = 6.0;
/// Windows `WHEEL_DELTA`; used to turn raw wheel events into notches.
pub const WHEEL_DELTA_UNIT: f32 = 120.0;
/// Cap notches processed from a single wheel burst (trackpad flicks).
pub const ZOOM_MAX_NOTCHES_PER_EVENT: f32 = 2.0;