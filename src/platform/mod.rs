pub mod win32;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

pub use win32::*;

/// Resizes the window for auto-fit. On macOS uses logical points and centers the
/// window; elsewhere delegates to macroquad (which scales by DPI for Win32, etc.).
pub fn request_window_size(width: f32, height: f32) {
    #[cfg(target_os = "macos")]
    macos::set_window_frame(width, height);

    #[cfg(not(target_os = "macos"))]
    macroquad::prelude::request_new_screen_size(width, height);
}

/// Sets fullscreen. On Linux/X11 uses a correct EWMH path (miniquad's is broken
/// when exiting). Elsewhere delegates to miniquad.
pub fn set_fullscreen(fullscreen: bool) {
    #[cfg(target_os = "linux")]
    {
        if linux::try_set_fullscreen(fullscreen) {
            return;
        }
    }
    macroquad::miniquad::window::set_fullscreen(fullscreen);
}
