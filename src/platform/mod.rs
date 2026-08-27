pub mod win32;

#[cfg(target_os = "linux")]
mod linux;

pub use win32::*;

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
