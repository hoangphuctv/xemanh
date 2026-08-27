#![windows_subsystem = "windows"]

use macroquad::prelude::*;
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const IMAGE_EXTENSIONS: [&str; 9] = [
    "jpg", "jpeg", "jpe", "jfif", "png", "bmp", "dib", "gif", "tga",
];
const JPEG_SAVE_QUALITY: u8 = 92;
const TOAST_DURATION: f64 = 1.2;
const DOUBLE_CLICK_SECS: f64 = 0.35;
const CHECKER_TILE_PX: u16 = 16;
const ICON_DATA: &[u8] = include_bytes!("../assets/xemanh.ico");

/// Small Win32 helpers (no-op stubs on other platforms so call sites stay clean).
mod win32 {
    #[cfg(target_os = "windows")]
    pub fn find_hwnd() -> usize {
        use std::ffi::c_void;
        #[link(name = "user32")]
        unsafe extern "system" {
            fn EnumWindows(
                cb: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
                lparam: *mut c_void,
            ) -> i32;
            fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
            fn IsWindowVisible(hwnd: *mut c_void) -> i32;
        }
        struct Ctx {
            pid: u32,
            hwnd: usize,
        }
        unsafe extern "system" fn cb(hwnd: *mut c_void, lparam: *mut c_void) -> i32 {
            let ctx = unsafe { &mut *(lparam as *mut Ctx) };
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
            if pid == ctx.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
                ctx.hwnd = hwnd as usize;
                return 0;
            }
            1
        }
        let mut ctx = Ctx {
            pid: std::process::id(),
            hwnd: 0,
        };
        unsafe { EnumWindows(cb, &mut ctx as *mut Ctx as *mut c_void) };        ctx.hwnd
    }

    #[cfg(not(target_os = "windows"))]
    pub fn find_hwnd() -> usize {
        0
    }

    #[cfg(target_os = "windows")]
    pub fn set_title(hwnd: usize, text: &str) {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SetWindowTextW(hwnd: *mut std::ffi::c_void, text: *const u16) -> i32;
        }
        if hwnd == 0 {
            return;
        }
        let mut wide: Vec<u16> = text.replace('…', "..").encode_utf16().collect();
        wide.push(0);
        unsafe { SetWindowTextW(hwnd as *mut _, wide.as_ptr()) };
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_title(_hwnd: usize, _text: &str) {}

    #[cfg(target_os = "windows")]
    pub fn is_zoomed(hwnd: usize) -> bool {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn IsZoomed(hwnd: *mut std::ffi::c_void) -> i32;
        }
        hwnd != 0 && unsafe { IsZoomed(hwnd as *mut _) } != 0
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_zoomed(_hwnd: usize) -> bool {
        false
    }

    /// Moves a file to the Recycle Bin (Windows). Non-Windows falls back to a hard delete.
    #[cfg(target_os = "windows")]
    pub fn recycle_delete(hwnd_parent: usize, path: &std::path::Path) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        #[repr(C)]
        #[allow(non_snake_case)]
        struct SHFILEOPSTRUCTW {
            hwnd: *mut std::ffi::c_void,
            wFunc: u32,
            pFrom: *const u16,
            pTo: *const u16,
            fFlags: u16,
            fAnyOperationsAborted: i32,
            hNameMappings: *mut std::ffi::c_void,
            lpszProgressTitle: *const u16,
        }
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn SHFileOperationW(lpFileOp: *mut SHFILEOPSTRUCTW) -> i32;
        }
        const FO_DELETE: u32 = 3;
        const FOF_SILENT: u16 = 0x0004;
        const FOF_NOCONFIRMATION: u16 = 0x0010;
        const FOF_ALLOWUNDO: u16 = 0x0040;

        let mut from: Vec<u16> = path.as_os_str().encode_wide().collect();
        from.push(0); // path list terminator
        from.push(0); // double-null terminated
        let mut op = SHFILEOPSTRUCTW {
            hwnd: hwnd_parent as *mut _,
            wFunc: FO_DELETE,
            pFrom: from.as_ptr(),
            pTo: std::ptr::null(),
            fFlags: FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT,
            fAnyOperationsAborted: 0,
            hNameMappings: std::ptr::null_mut(),
            lpszProgressTitle: std::ptr::null(),
        };
        let rc = unsafe { SHFileOperationW(&mut op) };
        if rc == 0 && op.fAnyOperationsAborted == 0 {
            Ok(())
        } else {
            Err(format!("Cannot delete file (code {rc})"))
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn recycle_delete(_hwnd_parent: usize, path: &Path) -> Result<(), String> {
        fs::remove_file(path).map_err(|e| format!("Cannot delete file: {e}"))
    }

    /// Sets the window icon from embedded ICO data (PNG entries).
    /// Parses the ICO format, picks the largest (≤256) entry for ICON_BIG
    /// and a small one for ICON_SMALL, then sends WM_SETICON.
    #[cfg(target_os = "windows")]
    pub fn set_icon(hwnd: usize, ico_bytes: &[u8]) {
        use std::ffi::c_void;
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SendMessageW(
                hwnd: *mut c_void,
                msg: u32,
                wparam: usize,
                lparam: isize,
            ) -> isize;
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn CreateIconFromResourceEx(
                presbits: *mut u8,
                dwressize: u32,
                ficon: i32,
                dwver: u32,
                cx: i32,
                cy: i32,
                uflags: u32,
            ) -> *mut c_void;
        }
        const WM_SETICON: u32 = 0x0080;
        const ICON_SMALL: usize = 0;
        const ICON_BIG: usize = 1;
        const LR_DEFAULTCOLOR: u32 = 0;
        const PNG_VER: u32 = 0x00030000;

        if hwnd == 0 || ico_bytes.len() < 6 {
            return;
        }
        // ICO header: 2 reserved, 2 type, 2 count
        let count = u16::from_le_bytes([ico_bytes[4], ico_bytes[5]]) as usize;
        // Parse directory entries to find the best big and small PNG offsets
        let mut best_big: (u32, u32, u32) = (0, 0, 0); // (size, offset, length)
        let mut best_small: (u32, u32, u32) = (256, 0, 0);
        for i in 0..count {
            let base = 6 + i * 16;
            if base + 16 > ico_bytes.len() {
                return;
            }
            let w = ico_bytes[base] as u32;
            let _h = ico_bytes[base + 1] as u32;
            let size = u32::from_le_bytes([
                ico_bytes[base + 8],
                ico_bytes[base + 9],
                ico_bytes[base + 10],
                ico_bytes[base + 11],
            ]);
            let offset = u32::from_le_bytes([
                ico_bytes[base + 12],
                ico_bytes[base + 13],
                ico_bytes[base + 14],
                ico_bytes[base + 15],
            ]);
            let dim = if w == 0 { 256 } else { w };
            if dim <= 256 && dim >= best_big.0 && size > 0 {
                best_big = (dim, offset, size);
            }
            if dim <= 32 && dim <= best_small.0 && size > 0 {
                best_small = (dim, offset, size);
            }
        }
        if best_big.2 == 0 && best_small.2 == 0 {
            return;
        }
        // If we only got one, use it for both
        if best_big.2 == 0 {
            best_big = best_small;
        }
        if best_small.2 == 0 {
            best_small = best_big;
        }
        let make_icon = |off: u32, len: u32| -> *mut c_void {
            let start = off as usize;
            let end = start + len as usize;
            if end > ico_bytes.len() {
                return std::ptr::null_mut();
            }
            let mut buf = ico_bytes[start..end].to_vec();
            unsafe {
                CreateIconFromResourceEx(
                    buf.as_mut_ptr(),
                    len,
                    1, // fIcon = TRUE
                    PNG_VER,
                    0, 0, // let system decide
                    LR_DEFAULTCOLOR,
                )
            }
        };
        let h_big = make_icon(best_big.1, best_big.2);
        let h_small = make_icon(best_small.1, best_small.2);
        unsafe {
            if !h_big.is_null() {
                SendMessageW(hwnd as *mut _, WM_SETICON, ICON_BIG, h_big as isize);
            }
            if !h_small.is_null() {
                SendMessageW(hwnd as *mut _, WM_SETICON, ICON_SMALL, h_small as isize);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_icon(_hwnd: usize, _ico_bytes: &[u8]) {}
}

/// Resolves a path, replacing a leading `~` with the user's home directory.
fn resolve_path(path: PathBuf) -> PathBuf {
    let Some(path_str) = path.to_str() else {
        return path;
    };
    if let Some(rest) = path_str.strip_prefix('~') {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let mut resolved = PathBuf::from(home);
        let clean_sub = rest.trim_start_matches(|c| c == '/' || c == '\\');
        if !clean_sub.is_empty() {
            resolved.push(clean_sub);
        }
        resolved
    } else {
        path
    }
}

fn is_image_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| IMAGE_EXTENSIONS.iter().any(|&e| ext.eq_ignore_ascii_case(e)))
            .unwrap_or(false)
}

/// Scans a directory for image files, sorted case-insensitively by file name.
fn scan_dir_images(dir: &Path) -> Vec<PathBuf> {
    let mut images: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_image_file(&path) {
                images.push(path);
            }
        }
    }
    images.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    images
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

/// Reads the EXIF orientation (1-8) from raw image bytes; 1 when absent or unreadable.
fn exif_orientation(bytes: &[u8]) -> u32 {
    let exif = exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok();
    let Some(exif) = exif else { return 1 };
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
}

/// Bakes the EXIF orientation into pixels so the image displays upright.
fn apply_orientation(img: image::DynamicImage, o: u32) -> image::DynamicImage {
    match o {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        // transpose = rotate 90 CW, then flip horizontal
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        // transverse = rotate 90 CCW (270 CW), then flip horizontal
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn load_texture(path: &Path) -> Result<Texture2D, String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let mut img = image::load_from_memory(&bytes).map_err(|e| format!("Cannot decode image: {e}"))?;
    let orientation = exif_orientation(&bytes);
    if orientation > 1 {
        img = apply_orientation(img, orientation);
    }
    let rgba = img.to_rgba8();
    let mq_image = Image {
        width: rgba.width() as u16,
        height: rgba.height() as u16,
        bytes: rgba.into_raw(),
    };
    Ok(Texture2D::from_image(&mq_image))
}

/// 32x32 pattern texture (2x2 squares of 16 px, white / light gray) for transparent areas.
fn make_checkerboard() -> Texture2D {
    let size = CHECKER_TILE_PX * 2;
    let tile = CHECKER_TILE_PX as u16;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let v: u8 = if ((x / tile) + (y / tile)) % 2 == 0 { 255 } else { 204 };
            bytes.extend_from_slice(&[v, v, v, 255]);
        }
    }
    Texture2D::from_image(&Image {
        width: size,
        height: size,
        bytes,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Rot {
    None,
    Cw,
    Ccw,
}

/// Re-encodes the image at `path` (optionally rotated) and overwrites the original file.
fn save_transformed(path: &Path, rot: Rot) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let img =
        image::load_from_memory(&bytes).map_err(|e| format!("Cannot decode image: {e}"))?;

    let fmt = image::ImageFormat::from_path(path)
        .map_err(|e| format!("Unknown file format: {e}"))?;

    let out_img = match fmt {
        image::ImageFormat::Jpeg => {
            let rgb8 = img.to_rgb8();
            image::DynamicImage::ImageRgb8(match rot {
                Rot::None => rgb8,
                Rot::Cw => image::imageops::rotate90(&rgb8),
                Rot::Ccw => image::imageops::rotate270(&rgb8),
            })
        }
        _ => {
            let rgba8 = img.to_rgba8();
            image::DynamicImage::ImageRgba8(match rot {
                Rot::None => rgba8,
                Rot::Cw => image::imageops::rotate90(&rgba8),
                Rot::Ccw => image::imageops::rotate270(&rgba8),
            })
        }
    };

    let mut out_bytes: Vec<u8> = Vec::new();
    match fmt {
        image::ImageFormat::Jpeg => {
            let rgb = out_img.to_rgb8();
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                Cursor::new(&mut out_bytes),
                JPEG_SAVE_QUALITY,
            );
            use image::ImageEncoder;
            encoder
                .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8)
                .map_err(|e| format!("Cannot encode JPEG: {e}"))?;
        }
        _ => out_img
            .write_to(&mut Cursor::new(&mut out_bytes), fmt)
            .map_err(|e| format!("Cannot encode image: {e}"))?,
    }

    fs::write(path, &out_bytes).map_err(|e| format!("Cannot write file: {e}"))
}

/// Computes the target window size, clamped to 90% of the screen on Windows.
fn clamp_window_target(width: f32, height: f32) -> (f32, f32) {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        let screen_w = unsafe { GetSystemMetrics(0) } as f32;
        let screen_h = unsafe { GetSystemMetrics(1) } as f32;
        if width < screen_w && height < screen_h {
            (width, height)
        } else {
            let ratio = (screen_w * 0.9 / width).min(screen_h * 0.9 / height);
            (width * ratio, height * ratio)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        (width, height)
    }
}

struct Toast {
    message: String,
    is_error: bool,
    deadline: f64,
}

struct App {
    entries: Vec<PathBuf>,
    index: usize,
    texture: Texture2D,
    checker: Texture2D,
    zoom: f32,
    zoom_target: f32,
    pan: Vec2,
    fullscreen: bool,
    toast: Option<Toast>,
    last_click_time: f64,
    hwnd: usize,
    was_maximized: bool,
    scroll_acc: f32,
}

impl App {
    fn set_toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some(Toast {
            message: message.into(),
            is_error,
            deadline: get_time() + TOAST_DURATION,
        });
    }

    fn reset_view(&mut self) {
        self.zoom_target = 1.0;
        self.pan = Vec2::ZERO;
    }

    /// Shows `filename [i/N] - XemAnh` in the window title bar.
    fn update_title(&mut self) {
        if self.hwnd == 0 {
            self.hwnd = win32::find_hwnd();
            if self.hwnd != 0 {
                win32::set_icon(self.hwnd, ICON_DATA);
            }
        }
        let title = if self.entries.is_empty() {
            "(no images) - XemAnh".to_string()
        } else {
            format!(
                "{} [{}/{}] - XemAnh",
                file_name_of(&self.entries[self.index]),
                self.index + 1,
                self.entries.len()
            )
        };
        win32::set_title(self.hwnd, &title);
    }

    /// Tracks maximize state; when the user un-maximizes, restore a window size
    /// matching the current image.
    fn sync_window_state(&mut self) {
        if self.hwnd == 0 {
            self.hwnd = win32::find_hwnd();
            if self.hwnd == 0 {
                return;
            }
        }
        let maximized = win32::is_zoomed(self.hwnd);
        if self.was_maximized && !maximized && !self.fullscreen {
            let (w, h) = clamp_window_target(self.texture.width(), self.texture.height());
            request_new_screen_size(w, h);
        }
        self.was_maximized = maximized;
    }

    /// Loads the image at `index`, resizes the window accordingly and resets the view.
    fn load_index(&mut self, index: usize) {
        let path = self.entries[index].clone();
        match load_texture(&path) {
            Ok(texture) => {
                self.index = index;
                self.texture = texture;
                self.reset_view();
                // Never resize while fullscreen or maximized: it breaks the window
                // state and desynchronizes the GL viewport from the window.
                if !self.fullscreen && !win32::is_zoomed(self.hwnd) {
                    let (w, h) = clamp_window_target(self.texture.width(), self.texture.height());
                    request_new_screen_size(w, h);
                }
                self.update_title();
            }
            Err(err) => self.set_toast(format!("[{}] {}", index + 1, err), true),
        }
    }

    fn next_image(&mut self) {
        if self.entries.len() > 1 {
            let next = (self.index + 1) % self.entries.len();
            self.load_index(next);
        }
    }

    fn prev_image(&mut self) {
        if self.entries.len() > 1 {
            let prev = (self.index + self.entries.len() - 1) % self.entries.len();
            self.load_index(prev);
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        macroquad::miniquad::window::set_fullscreen(self.fullscreen);
        if !self.fullscreen {
            // Restore a sane window size for the current image after exiting fullscreen.
            let (w, h) = clamp_window_target(self.texture.width(), self.texture.height());
            request_new_screen_size(w, h);
        }
    }

    fn rotate_and_save(&mut self, rot: Rot) {
        let path = self.entries[self.index].clone();
        match save_transformed(&path, rot) {
            Ok(()) => match load_texture(&path) {
                Ok(texture) => {
                    self.texture = texture;
                    self.reset_view();
                    self.set_toast(format!(
                        "Saved {}",
                        file_name_of(&path)
                    ), false);
                }
                Err(err) => self.set_toast(format!("Saved but reload failed: {err}"), true),
            },
            Err(err) => self.set_toast(err, true),
        }
    }

    /// Deletes the current image to the Recycle Bin and shows the next one.
    fn delete_current(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let path = self.entries[self.index].clone();
        let name = file_name_of(&path);
        match win32::recycle_delete(self.hwnd, &path) {
            Ok(()) => {
                self.entries.remove(self.index);
                if self.entries.is_empty() {
                    self.set_toast(format!("Deleted {name}. No more images."), false);
                    self.update_title();
                    return;
                }
                // Stay at the same position: it now points at the next image.
                let next = self.index.min(self.entries.len() - 1);
                self.load_index(next);
                self.set_toast(format!("Deleted {name}"), false);
            }
            Err(err) => self.set_toast(err, true),
        }
    }

    fn fit_scale(&self) -> f32 {
        let win_w = screen_width();
        let win_h = screen_height();
        (win_w / self.texture.width()).min(win_h / self.texture.height())
    }

    /// Scale at zoom = 1.0: native size when the image fits the window,
    /// scaled down only when the image is larger than the window.
    fn base_scale(&self) -> f32 {
        self.fit_scale().min(1.0)
    }

    /// Displayed rectangle of the texture (after pan-clamping).
    fn view_rect(&self) -> Rect {
        let scale = self.base_scale() * self.zoom;
        let disp_w = self.texture.width() * scale;
        let disp_h = self.texture.height() * scale;

        let mut pan = self.pan;
        let max_x = (disp_w - screen_width()) / 2.0;
        let max_y = (disp_h - screen_height()) / 2.0;
        pan.x = if max_x <= 0.0 { 0.0 } else { pan.x.clamp(-max_x, max_x) };
        pan.y = if max_y <= 0.0 { 0.0 } else { pan.y.clamp(-max_y, max_y) };

        Rect {
            x: screen_width() / 2.0 + pan.x - disp_w / 2.0,
            y: screen_height() / 2.0 + pan.y - disp_h / 2.0,
            w: disp_w,
            h: disp_h,
        }
    }

    /// Zooms by `factor`, keeping the point under the mouse cursor fixed.
    fn zoom_at_mouse(&mut self, factor: f32) {
        let before = self.view_rect();
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);

        let min_zoom = 0.25;
        let max_zoom = (8.0 / self.base_scale()).max(1.0);
        self.zoom_target = (self.zoom_target * factor).clamp(min_zoom, max_zoom);

        let after = self.view_rect_unclamped();
        // pan' = m - center - (m - center - pan_before) * (size_after / size_before)
        let center = vec2(screen_width() / 2.0, screen_height() / 2.0);
        let pan_before = self.pan;
        let ratio = after.w / before.w;
        self.pan = mouse - center - (mouse - center - pan_before) * ratio;
    }

    fn view_rect_unclamped(&self) -> Rect {
        let scale = self.base_scale() * self.zoom;
        let disp_w = self.texture.width() * scale;
        let disp_h = self.texture.height() * scale;
        Rect {
            x: screen_width() / 2.0 + self.pan.x - disp_w / 2.0,
            y: screen_height() / 2.0 + self.pan.y - disp_h / 2.0,
            w: disp_w,
            h: disp_h,
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
        if is_key_pressed(KeyCode::Home) && !self.entries.is_empty() {
            self.load_index(0);
        }
        if is_key_pressed(KeyCode::End) && !self.entries.is_empty() {
            self.load_index(self.entries.len() - 1);
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
            self.zoom_at_mouse(factor);
        }

        // Pan with left mouse drag
        if is_mouse_button_down(MouseButton::Left) {
            self.pan += mouse_delta_position();
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

    fn update(&mut self) -> bool {
        self.sync_window_state();
        if !self.handle_input() {
            return false;
        }

        // Smooth zoom interpolation
        let lerp_speed = 12.0;
        let t = 1.0 - (-lerp_speed * get_frame_time()).exp();
        self.zoom += (self.zoom_target - self.zoom) * t;
        if (self.zoom - self.zoom_target).abs() < 0.0001 {
            self.zoom = self.zoom_target;
        }

        clear_background(BLACK);

        let rect = self.view_rect();
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

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string())
}

#[macroquad::main("XemAnh")]
async fn main() -> Result<(), Box<dyn Error>> {
    // Unicode-safe on Windows (Chinese / CJK paths); args() would be lossy.
    let arg = std::env::args_os().nth(1).map(PathBuf::from);

    // Determine the initial image path
    let initial_path: PathBuf = if let Some(val) = arg {
        let resolved = resolve_path(val);
        if resolved.is_dir() {
            scan_dir_images(&resolved)
                .into_iter()
                .next()
                .ok_or_else(|| format!("No image files found in directory: {:?}", resolved))?
        } else if resolved.is_file() {
            resolved
        } else {
            return Err(format!(
                "Path does not exist or is not a file/directory: {:?}",
                resolved
            )
            .into());
        }
    } else {
        // Default to ~/Pictures if no argument is provided
        let pictures_dir = resolve_path(PathBuf::from("~/Pictures"));
        if pictures_dir.is_dir() {
            scan_dir_images(&pictures_dir)
                .into_iter()
                .next()
                .ok_or_else(|| format!("No image files found in ~/Pictures directory: {:?}", pictures_dir))?
        } else {
            return Err(format!("~/Pictures directory does not exist: {:?}", pictures_dir).into());
        }
    };

    // Scan the containing folder so arrows can navigate between images
    let dir = initial_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let mut entries = scan_dir_images(&dir);
    if entries.is_empty() {
        entries.push(initial_path.clone());
    }
    let index = entries
        .iter()
        .position(|p| same_path(p, &initial_path))
        .unwrap_or(0);

    let texture = load_texture(&entries[index])?;

    let mut app = App {
        entries,
        index,
        texture,
        checker: make_checkerboard(),
        zoom: 1.0,
        zoom_target: 1.0,
        pan: Vec2::ZERO,
        fullscreen: false,
        toast: None,
        last_click_time: 0.0,
        hwnd: 0,
        was_maximized: false,
        scroll_acc: 0.0,
    };
    app.update_title();

    // Size the window to the initial image
    let (w, h) = clamp_window_target(app.texture.width(), app.texture.height());
    request_new_screen_size(w, h);

    loop {
        if !app.update() {
            break;
        }
        next_frame().await;
    }

    Ok(())
}
