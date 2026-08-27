//! Small Win32 helpers (no-op stubs on other platforms so call sites stay clean).

/// Computes the target window size, clamped to 90% of the screen on Windows.
pub fn clamp_window_target(width: f32, height: f32) -> (f32, f32) {
    let mut w = width;
    let mut h = height;

    // Enforce a minimum window size of 800x600 (landscape/square) or 600x800 (portrait).
    if w < 800.0 && h < 600.0 {
        if w >= h {
            w = 800.0;
            h = 600.0;
        } else {
            w = 600.0;
            h = 800.0;
        }
    }

    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        let screen_w = unsafe { GetSystemMetrics(0) } as f32;
        let screen_h = unsafe { GetSystemMetrics(1) } as f32;
        if w < screen_w && h < screen_h {
            (w, h)
        } else {
            let ratio = (screen_w * 0.9 / w).min(screen_h * 0.9 / h);
            (w * ratio, h * ratio)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        (w, h)
    }
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
