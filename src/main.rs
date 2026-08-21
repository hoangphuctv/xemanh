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

    loop {
        // Clear background to a sleek dark color
        clear_background(BLACK);

        // Draw the texture at coordinates (0, 0)
        draw_texture_ex(
            &texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams::default(),
        );

        next_frame().await;
    }
}
