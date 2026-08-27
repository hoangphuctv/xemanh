//! Linux X11 helpers. Work around miniquad's broken `set_fullscreen(false)` on X11
//! (see https://github.com/not-fl3/macroquad/issues/629).

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_long, c_ulong};

type Display = c_void;
type Window = c_ulong;
type Atom = c_ulong;
type Bool = c_int;
type Status = c_int;

const CLIENT_MESSAGE: c_int = 33;
const NONE: Window = 0;
const POINTER_ROOT: Window = 1;
const SUCCESS: c_int = 0;
/// `_NET_WM_STATE_REMOVE` / `_NET_WM_STATE_ADD`
const NET_WM_STATE_REMOVE: c_long = 0;
const NET_WM_STATE_ADD: c_long = 1;
/// `SubstructureRedirectMask | SubstructureNotifyMask` (EWMH)
const SUBSTRUCTURE_MASK: c_long = 1_048_576 | 524_288;
/// `AnyPropertyType`
const ANY_PROPERTY_TYPE: Atom = 0;

#[repr(C)]
struct XClientMessageEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: Bool,
    display: *mut Display,
    window: Window,
    message_type: Atom,
    format: c_int,
    data: [c_long; 5],
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
    fn XDefaultRootWindow(display: *mut Display) -> Window;
    fn XInternAtom(display: *mut Display, atom_name: *const c_char, only_if_exists: Bool) -> Atom;
    fn XGetInputFocus(display: *mut Display, focus_return: *mut Window, revert_to: *mut c_int) -> c_int;
    fn XQueryTree(
        display: *mut Display,
        w: Window,
        root_return: *mut Window,
        parent_return: *mut Window,
        children_return: *mut *mut Window,
        nchildren_return: *mut u32,
    ) -> Status;
    fn XGetWindowProperty(
        display: *mut Display,
        w: Window,
        property: Atom,
        long_offset: c_long,
        long_length: c_long,
        delete: Bool,
        req_type: Atom,
        actual_type: *mut Atom,
        actual_format: *mut c_int,
        nitems: *mut c_ulong,
        bytes_after: *mut c_ulong,
        prop: *mut *mut u8,
    ) -> c_int;
    fn XSendEvent(
        display: *mut Display,
        w: Window,
        propagate: Bool,
        event_mask: c_long,
        event_send: *mut XClientMessageEvent,
    ) -> Status;
    fn XFlush(display: *mut Display) -> c_int;
    fn XFree(data: *mut c_void) -> c_int;
}

unsafe fn has_property(display: *mut Display, window: Window, property: Atom) -> bool {
    unsafe {
        let mut actual_type = 0 as Atom;
        let mut actual_format = 0;
        let mut nitems = 0 as c_ulong;
        let mut bytes_after = 0 as c_ulong;
        let mut prop: *mut u8 = std::ptr::null_mut();
        let status = XGetWindowProperty(
            display,
            window,
            property,
            0,
            0,
            0,
            ANY_PROPERTY_TYPE,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut prop,
        );
        if !prop.is_null() {
            XFree(prop as *mut c_void);
        }
        status == SUCCESS && actual_type != 0
    }
}

/// Find the ICCCM client window (has `WM_STATE`). Must not use the WM frame
/// (direct child of root) — Cinnamon/Muffin ignore `_NET_WM_STATE` on frames.
unsafe fn client_window(display: *mut Display, mut window: Window) -> Window {
    unsafe {
        let wm_state = XInternAtom(display, c"WM_STATE".as_ptr(), 0);
        let mut fallback = window;
        loop {
            if wm_state != 0 && has_property(display, window, wm_state) {
                return window;
            }
            let mut root = NONE;
            let mut parent = NONE;
            let mut children: *mut Window = std::ptr::null_mut();
            let mut nchildren = 0u32;
            if XQueryTree(display, window, &mut root, &mut parent, &mut children, &mut nchildren) == 0
            {
                return fallback;
            }
            if !children.is_null() {
                XFree(children as *mut c_void);
            }
            if parent == NONE || parent == root {
                return fallback;
            }
            fallback = window;
            window = parent;
        }
    }
}

/// Toggle fullscreen via EWMH `_NET_WM_STATE`. Returns `true` if the request was sent.
///
/// Unlike miniquad, this always uses the `_NET_WM_STATE_FULLSCREEN` atom and sends
/// ADD or REMOVE (never an empty atom / always-ADD). Also skips Unmap/Map, which
/// steals focus and drops key events on Cinnamon/X11.
pub fn try_set_fullscreen(fullscreen: bool) -> bool {
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return false;
        }

        let mut focus = NONE;
        let mut revert = 0;
        XGetInputFocus(display, &mut focus, &mut revert);
        if focus == NONE || focus == POINTER_ROOT {
            XCloseDisplay(display);
            return false;
        }
        let window = client_window(display, focus);

        let wm_state = XInternAtom(display, c"_NET_WM_STATE".as_ptr(), 0);
        let wm_fullscreen = XInternAtom(display, c"_NET_WM_STATE_FULLSCREEN".as_ptr(), 0);
        if wm_state == 0 || wm_fullscreen == 0 {
            XCloseDisplay(display);
            return false;
        }

        let mut ev = XClientMessageEvent {
            type_: CLIENT_MESSAGE,
            serial: 0,
            send_event: 1,
            display,
            window,
            message_type: wm_state,
            format: 32,
            data: [
                if fullscreen {
                    NET_WM_STATE_ADD
                } else {
                    NET_WM_STATE_REMOVE
                },
                wm_fullscreen as c_long,
                0,
                1, // source: application
                0,
            ],
        };

        let root = XDefaultRootWindow(display);
        let ok = XSendEvent(display, root, 0, SUBSTRUCTURE_MASK, &mut ev) != 0;
        XFlush(display);
        XCloseDisplay(display);
        ok
    }
}
