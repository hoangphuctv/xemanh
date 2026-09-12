//! macOS window placement and screen metrics.

use objc::runtime::{self, Object};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::{CStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

type ObjcId = *mut Object;

static OPEN_DOCUMENT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static OPEN_DOCUMENT_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

fn open_document_slot() -> &'static Mutex<Option<PathBuf>> {
    OPEN_DOCUMENT.get_or_init(|| Mutex::new(None))
}

extern "C" fn application_open_files(
    _this: &mut Object,
    _: objc::runtime::Sel,
    _application: ObjcId,
    filenames: ObjcId,
) {
    if filenames.is_null() {
        return;
    }

    unsafe {
        let count: usize = msg_send![filenames, count];
        if count == 0 {
            return;
        }

        let filename: ObjcId = msg_send![filenames, objectAtIndex: 0usize];
        if filename.is_null() {
            return;
        }

        let bytes: *const std::ffi::c_char =
            msg_send![filename, fileSystemRepresentation];
        if bytes.is_null() {
            return;
        }

        let path = PathBuf::from(OsString::from_vec(
            CStr::from_ptr(bytes).to_bytes().to_vec(),
        ));

        if let Ok(mut slot) = open_document_slot().lock() {
            *slot = Some(path);
        }
    }
}

pub fn install_open_document_handler() {
    if OPEN_DOCUMENT_HANDLER_INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }

    unsafe {
        let delegate = class!(NSAppDelegate) as *const _ as *mut _;
        let types = b"v@:@@\0";

        let imp: runtime::Imp = std::mem::transmute(application_open_files as unsafe extern "C" fn(&mut Object, objc::runtime::Sel, ObjcId, ObjcId));
        runtime::class_addMethod(
            delegate,
            sel!(application:openFiles:),
            imp,
            types.as_ptr().cast(),
        );
    }
}

pub fn take_open_document() -> Option<PathBuf> {
    open_document_slot().lock().ok()?.take()
}

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

/// Resizes the window in logical points, keeping its current top-left position.
///
/// miniquad's `set_window_size` and macroquad's `request_new_screen_size` both pass
/// backing-store pixels on macOS, but `NSWindow` expects points — so we set the frame
/// directly. The origin is preserved (no re-centering) so the window never jumps
/// during auto-fit.
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

        // NSWindow's frame origin is the bottom-left corner. Adjust y so the top
        // edge stays where it is while resizing; keep x unchanged.
        let current: NSRect = msg_send![window, frame];
        let frame = NSRect {
            origin: NSPoint {
                x: current.origin.x,
                y: current.origin.y + current.size.height - h,
            },
            size: NSSize { width: w, height: h },
        };
        let _: () = msg_send![window, setFrame:frame display:true animate:false];
    }
}

pub fn set_title(text: &str) {
    unsafe {
        let window = app_window();
        if window.is_null() {
            return;
        }
        let cls = class!(NSString);
        let alloc: ObjcId = msg_send![cls, alloc];
        let string: ObjcId = msg_send![alloc, initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: () = msg_send![window, setTitle:string];
        let _: () = msg_send![string, release];
    }
}
