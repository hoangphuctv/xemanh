//! macOS window placement and screen metrics.

use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};

type ObjcId = *mut Object;

#[repr(C)]
#[derive(Copy, Clone)]
struct NSPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct NSSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct NSRect {
    origin: NSPoint,
    size: NSSize,
}

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

/// Usable screen area in logical points (excludes menu bar and dock).
pub fn screen_size_logical() -> (f32, f32) {
    unsafe {
        let screen: ObjcId = msg_send![class!(NSScreen), mainScreen];
        if screen.is_null() {
            return fallback_screen_size();
        }
        let frame: NSRect = msg_send![screen, visibleFrame];
        let w = frame.size.width as f32;
        let h = frame.size.height as f32;
        if w > 0.0 && h > 0.0 {
            (w, h)
        } else {
            fallback_screen_size()
        }
    }
}

fn fallback_screen_size() -> (f32, f32) {
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> CGRect;
    }

    unsafe {
        let bounds = CGDisplayBounds(CGMainDisplayID());
        (bounds.size.width as f32, bounds.size.height as f32)
    }
}

unsafe fn app_window() -> ObjcId {
    let app: ObjcId = msg_send![class!(NSApplication), sharedApplication];
    if app.is_null() {
        return std::ptr::null_mut();
    }

    let window: ObjcId = msg_send![app, keyWindow];
    if !window.is_null() {
        return window;
    }

    let window: ObjcId = msg_send![app, mainWindow];
    if !window.is_null() {
        return window;
    }

    let windows: ObjcId = msg_send![app, windows];
    if windows.is_null() {
        return std::ptr::null_mut();
    }

    let count: usize = msg_send![windows, count];
    if count == 0 {
        return std::ptr::null_mut();
    }

    msg_send![windows, objectAtIndex: 0usize]
}

/// Sets window size in logical points and centers it within the visible screen area.
///
/// miniquad's `set_window_size` and macroquad's `request_new_screen_size` both pass
/// backing-store pixels on macOS, but `NSWindow` expects points — so we set the frame
/// directly and center it.
pub fn set_window_frame(logical_w: f32, logical_h: f32) {
    let w = logical_w.round().max(1.0) as f64;
    let h = logical_h.round().max(1.0) as f64;

    unsafe {
        let window = app_window();
        if window.is_null() {
            // Window not ready yet; still use logical points (not backing pixels).
            macroquad::miniquad::window::set_window_size(w as u32, h as u32);
            return;
        }

        let screen: ObjcId = msg_send![class!(NSScreen), mainScreen];
        let visible: NSRect = if screen.is_null() {
            NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: w,
                    height: h,
                },
            }
        } else {
            msg_send![screen, visibleFrame]
        };

        let origin_x = visible.origin.x + (visible.size.width - w).max(0.0) / 2.0;
        let origin_y = visible.origin.y + (visible.size.height - h).max(0.0) / 2.0;

        let frame = NSRect {
            origin: NSPoint {
                x: origin_x,
                y: origin_y,
            },
            size: NSSize { width: w, height: h },
        };
        let _: () = msg_send![window, setFrame:frame display:true animate:false];
    }
}
