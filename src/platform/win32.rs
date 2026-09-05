//! Small Win32 helpers (no-op stubs on other platforms so call sites stay clean).

use crate::constants::{
    WINDOW_MIN_LANDSCAPE_H, WINDOW_MIN_LANDSCAPE_W, WINDOW_MIN_PORTRAIT_H, WINDOW_MIN_PORTRAIT_W,
};

/// Returns the usable screen size in logical pixels for auto window sizing.
fn screen_size_logical(dpi: f32) -> (f32, f32) {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        let dpi = dpi.max(1.0);
        (
            unsafe { GetSystemMetrics(0) } as f32 / dpi,
            unsafe { GetSystemMetrics(1) } as f32 / dpi,
        )
    }
    #[cfg(target_os = "macos")]
    {
        let _ = dpi;
        super::macos::screen_size_logical()
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let _ = dpi;
        (f32::MAX, f32::MAX)
    }
}

/// Shrinks `(w, h)` to fit within `max_w` × `max_h` while preserving aspect ratio.
fn clamp_to_screen_limits(
    w: f32,
    h: f32,
    min_w: f32,
    min_h: f32,
    max_w: f32,
    max_h: f32,
) -> (f32, f32) {
    let (mut w, mut h) = (w, h);
    if w > max_w || h > max_h {
        let ratio = (max_w / w).min(max_h / h);
        w *= ratio;
        h *= ratio;
    }
    // After downscaling, re-apply minimums (extreme aspect ratios can shrink one axis too far).
    w = w.max(min_w.min(max_w));
    h = h.max(min_h.min(max_h));
    (w, h)
}

/// Computes the target window size in logical pixels, clamped to 90% of the screen.
/// Used only for programmatic auto-resize; manual user resizing is not limited.
/// `dpi` converts physical screen metrics into the same units as `width`/`height`.
pub fn clamp_window_target(width: f32, height: f32, dpi: f32) -> (f32, f32) {
    let (min_w, min_h) = if height > width {
        (WINDOW_MIN_PORTRAIT_W, WINDOW_MIN_PORTRAIT_H)
    } else {
        (WINDOW_MIN_LANDSCAPE_W, WINDOW_MIN_LANDSCAPE_H)
    };

    // Apply minimums per axis so wide-but-short images still get a usable window.
    let w = width.max(min_w);
    let h = height.max(min_h);

    let (screen_w, screen_h) = screen_size_logical(dpi);
    let max_w = screen_w * 0.9;
    let max_h = screen_h * 0.9;
    clamp_to_screen_limits(w, h, min_w, min_h, max_w, max_h)
}

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
    unsafe { EnumWindows(cb, &mut ctx as *mut Ctx as *mut c_void) };
    ctx.hwnd
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

#[cfg(target_os = "macos")]
pub fn set_title(_hwnd: usize, text: &str) {
    crate::platform::macos::set_title(text);
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
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
pub fn recycle_delete(_hwnd_parent: usize, path: &std::path::Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| format!("Cannot delete file: {e}"))
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
                0,
                0, // let system decide
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

/// Copies an RGBA image onto the Windows clipboard as CF_DIB (wide app support)
/// and PNG (keeps transparency in modern apps).
#[cfg(target_os = "windows")]
pub fn copy_image_to_clipboard(
    hwnd: usize,
    width: u32,
    height: u32,
    rgba: &[u8],
    png: &[u8],
) -> Result<(), String> {
    use std::ffi::c_void;
    use std::ptr;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(hwnd: *mut c_void) -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, mem: *mut c_void) -> *mut c_void;
        fn CloseClipboard() -> i32;
        fn RegisterClipboardFormatW(name: *const u16) -> u32;
    }

    const CF_DIB: u32 = 8;

    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || rgba.len() != w * h * 4 {
        return Err("Cannot copy image: invalid pixel data".into());
    }

    let dib = encode_dib_bgra(width, height, rgba);
    let h_dib = alloc_hglobal(&dib)?;
    let h_png = if png.is_empty() {
        ptr::null_mut()
    } else {
        match alloc_hglobal(png) {
            Ok(h) => h,
            Err(err) => {
                unsafe { global_free(h_dib) };
                return Err(err);
            }
        }
    };

    if unsafe { OpenClipboard(hwnd as *mut c_void) } == 0 {
        unsafe {
            global_free(h_dib);
            if !h_png.is_null() {
                global_free(h_png);
            }
        }
        return Err("Cannot open clipboard".into());
    }

    let mut dib_on_clipboard = false;
    let mut png_on_clipboard = false;
    let mut err: Option<String> = None;

    if unsafe { EmptyClipboard() } == 0 {
        err = Some("Cannot clear clipboard".into());
    } else if unsafe { SetClipboardData(CF_DIB, h_dib) }.is_null() {
        err = Some("Cannot put image on clipboard".into());
    } else {
        dib_on_clipboard = true;
        if !h_png.is_null() {
            let mut png_name: Vec<u16> = "PNG".encode_utf16().collect();
            png_name.push(0);
            let png_fmt = unsafe { RegisterClipboardFormatW(png_name.as_ptr()) };
            if png_fmt != 0 && !unsafe { SetClipboardData(png_fmt, h_png) }.is_null() {
                png_on_clipboard = true;
            }
        }
    }

    unsafe { CloseClipboard() };

    if !dib_on_clipboard {
        unsafe { global_free(h_dib) };
    }
    if !h_png.is_null() && !png_on_clipboard {
        unsafe { global_free(h_png) };
    }

    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(target_os = "windows")]
fn encode_dib_bgra(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let stride = w * 4;
    let mut dib = vec![0u8; 40 + stride * h];
    dib[0..4].copy_from_slice(&40u32.to_le_bytes());
    dib[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    dib[8..12].copy_from_slice(&(height as i32).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    let size_image = (stride * h) as u32;
    dib[20..24].copy_from_slice(&size_image.to_le_bytes());
    for y in 0..h {
        let src_row = (h - 1 - y) * stride;
        let dst_row = 40 + y * stride;
        for x in 0..w {
            let s = src_row + x * 4;
            let d = dst_row + x * 4;
            dib[d] = rgba[s + 2];
            dib[d + 1] = rgba[s + 1];
            dib[d + 2] = rgba[s];
            dib[d + 3] = rgba[s + 3];
        }
    }
    dib
}

#[cfg(target_os = "windows")]
fn alloc_hglobal(bytes: &[u8]) -> Result<*mut std::ffi::c_void, String> {
    const GMEM_MOVEABLE: u32 = 0x0002;
    let mem = unsafe { global_alloc(GMEM_MOVEABLE, bytes.len()) };
    if mem.is_null() {
        return Err("Cannot allocate clipboard memory".into());
    }
    let ptr = unsafe { global_lock(mem) };
    if ptr.is_null() {
        unsafe { global_free(mem) };
        return Err("Cannot lock clipboard memory".into());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        global_unlock(mem);
    }
    Ok(mem)
}

#[cfg(target_os = "windows")]
unsafe fn global_alloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
    }
    unsafe { GlobalAlloc(flags, bytes) }
}

#[cfg(target_os = "windows")]
unsafe fn global_lock(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalLock(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }
    unsafe { GlobalLock(mem) }
}

#[cfg(target_os = "windows")]
unsafe fn global_unlock(mem: *mut std::ffi::c_void) {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalUnlock(mem: *mut std::ffi::c_void) -> i32;
    }
    unsafe {
        GlobalUnlock(mem);
    }
}

#[cfg(target_os = "windows")]
unsafe fn global_free(mem: *mut std::ffi::c_void) {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalFree(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }
    unsafe {
        GlobalFree(mem);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn copy_image_to_clipboard(
    _hwnd: usize,
    _width: u32,
    _height: u32,
    _rgba: &[u8],
    _png: &[u8],
) -> Result<(), String> {
    Err("Clipboard copy is only supported on Windows".into())
}
