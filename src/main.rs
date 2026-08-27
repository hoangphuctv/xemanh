#![windows_subsystem = "windows"]

mod app;
mod constants;
mod gallery;
mod image_io;
mod platform;
mod view;

use std::error::Error;
use std::path::PathBuf;

use macroquad::prelude::*;

use app::App;
use gallery::Gallery;
use platform::clamp_window_target;

#[macroquad::main("XemAnh")]
async fn main() -> Result<(), Box<dyn Error>> {
    // Unicode-safe on Windows (Chinese / CJK paths); args() would be lossy.
    let arg = std::env::args_os().nth(1).map(PathBuf::from);
    let gallery = Gallery::from_startup_arg(arg)?;
    let mut app = App::new(gallery)?;
    app.update_title();

    let (tw, th) = app.texture_size();
    let (w, h) = clamp_window_target(tw, th);
    request_new_screen_size(w, h);

    loop {
        if !app.update() {
            break;
        }
        next_frame().await;
    }

    Ok(())
}
