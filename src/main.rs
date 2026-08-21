use macroquad::prelude::*;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves a path string, replacing a leading `~` with the user's home directory.
fn resolve_path(path_str: &str) -> PathBuf {
    if path_str.starts_with('~') {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let mut resolved = PathBuf::from(home);
        if path_str.len() > 1 {
            let sub = &path_str[1..];
            // Normalize path separators by removing leading slashes/backslashes
            let clean_sub = sub.trim_start_matches(|c| c == '/' || c == '\\');
            resolved.push(clean_sub);
        }
        resolved
    } else {
        PathBuf::from(path_str)
    }
}

/// Scans a directory and returns the path to the first image file found.
fn find_first_image(dir: &Path) -> Option<PathBuf> {
    let extensions = ["jpg", "jpeg", "png", "bmp", "tga", "gif"];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.iter().any(|&e| ext.eq_ignore_ascii_case(e)) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

#[macroquad::main("Macroquad Image Viewer")]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get the first command line argument (excluding the binary name itself)
    let arg = std::env::args().nth(1);

    // Determine the image path to load
    let path_to_load = if let Some(val) = arg {
        let resolved = resolve_path(&val);
        if resolved.is_dir() {
            find_first_image(&resolved).ok_or_else(|| {
                format!("No image files found in directory: {:?}", resolved)
            })?
        } else if resolved.is_file() {
            resolved
        } else {
            return Err(format!("Path does not exist or is not a file/directory: {:?}", resolved).into());
        }
    } else {
        // Default to ~/Pictures if no argument is provided
        let pictures_dir = resolve_path("~/Pictures");
        if pictures_dir.is_dir() {
            find_first_image(&pictures_dir).ok_or_else(|| {
                format!("No image files found in ~/Pictures directory: {:?}", pictures_dir)
            })?
        } else {
            return Err(format!("~/Pictures directory does not exist: {:?}", pictures_dir).into());
        }
    };

    println!("Loading image from: {:?}", path_to_load);

    // Read the file synchronously from the local system
    let bytes = fs::read(&path_to_load)?;
    
    // Decode the image bytes using the `image` crate (enabling Jpeg support)
    let img = image::load_from_memory(&bytes)?;
    let rgba = img.to_rgba8();
    
    // Construct Macroquad's Image struct
    let image = Image {
        width: rgba.width() as u16,
        height: rgba.height() as u16,
        bytes: rgba.into_raw(),
    };
    
    // Create the Texture2D from the loaded image
    let texture = Texture2D::from_image(&image);

    let mut window_centered = false;

    loop {
        // Center the window on the screen on Windows
        if !window_centered {
            if center_window_on_screen("Macroquad Image Viewer") {
                window_centered = true;
            }
        }

        // Clear background to a sleek dark color
        clear_background(BLACK);

        // Calculate coordinates to center the image in the window
        let x = (screen_width() - texture.width()) / 2.0;
        let y = (screen_height() - texture.height()) / 2.0;

        // Draw the texture at the calculated center coordinates
        draw_texture_ex(
            &texture,
            x,
            y,
            WHITE,
            DrawTextureParams::default(),
        );

        next_frame().await;
    }
}

/// Centers the application window on the screen using native OS APIs.
fn center_window_on_screen(window_title: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::CString;
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct RECT {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }
        #[link(name = "user32")]
        extern "system" {
            fn FindWindowA(lpClassName: *const u8, lpWindowName: *const u8) -> *mut std::ffi::c_void;
            fn GetSystemMetrics(nIndex: i32) -> i32;
            fn GetWindowRect(hWnd: *mut std::ffi::c_void, lpRect: *mut RECT) -> i32;
            fn SetWindowPos(
                hWnd: *mut std::ffi::c_void,
                hWndInsertAfter: *mut std::ffi::c_void,
                X: i32,
                Y: i32,
                cx: i32,
                cy: i32,
                uFlags: u32,
            ) -> i32;
        }

        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;
        const SWP_NOSIZE: u32 = 0x0001;
        const SWP_NOZORDER: u32 = 0x0004;

        if let Ok(c_title) = CString::new(window_title) {
            unsafe {
                let hwnd = FindWindowA(std::ptr::null(), c_title.as_ptr() as *const u8);
                if !hwnd.is_null() {
                    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    if GetWindowRect(hwnd, &mut rect) != 0 {
                        let win_width = rect.right - rect.left;
                        let win_height = rect.bottom - rect.top;
                        let screen_width = GetSystemMetrics(SM_CXSCREEN);
                        let screen_height = GetSystemMetrics(SM_CYSCREEN);
                        let x = (screen_width - win_width) / 2;
                        let y = (screen_height - win_height) / 2;
                        SetWindowPos(
                            hwnd,
                            std::ptr::null_mut(),
                            x,
                            y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER,
                        );
                        return true;
                    }
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}
