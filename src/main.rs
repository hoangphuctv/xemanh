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

fn window_conf() -> Conf {
    Conf {
        window_title: "XemAnh".to_owned(),
        window_width: 800,
        window_height: 600,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> Result<(), Box<dyn Error>> {
    // Unicode-safe on Windows (Chinese / CJK paths); args() would be lossy.
    let arg = std::env::args_os().nth(1).map(PathBuf::from);
    let gallery = Gallery::from_startup_arg(arg)?;
    let mut app = App::new(gallery)?;
    app.update_title();

    let (tw, th) = app.texture_size();
    let dpi = screen_dpi_scale().max(1.0);
    let (w, h) = clamp_window_target(tw / dpi, th / dpi, dpi);
    request_new_screen_size(w, h);

    loop {
        if !app.update() {
            break;
        }
        next_frame().await;
    }

    Ok(())
}
