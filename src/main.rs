mod app;

use eframe::egui;
use std::fs;

use crate::app::PrologApp;

fn main() -> Result<(), eframe::Error> {
    let icon_data = load_icon();

    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_title("Daviti's Prolog Parser");

    if let Some(icon) = icon_data {
        viewport_builder = viewport_builder.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    let default_text = load_default_test_file();

    eframe::run_native(
        "Daviti's Prolog Parser",
        options,
        Box::new(move |_cc| Ok(Box::new(PrologApp::with_text(default_text)))),
    )
}

fn load_default_test_file() -> String {
    if let Ok(content) = fs::read_to_string("assets/base.txt") {
        println!("Loaded assets/base.txt");
        content
    } else if let Ok(content) = fs::read_to_string("assets/simple.txt") {
        println!("Loaded assets/simple.txt");
        content
    } else if let Ok(content) = fs::read_to_string("assets/complex.txt") {
        println!("Loaded assets/complex.txt");
        content
    } else {
        println!("Could not load asset files, using default text");
        "Bear is an animal.\nCat has fur.\nJohn likes pizza.\nAll mammals are animals.".to_string()
    }
}

fn load_icon() -> Option<egui::IconData> {
    let logo_path = "assets/logo.png";

    match image::open(logo_path) {
        Ok(img) => {
            let image_buffer = img.to_rgba8();
            let (width, height) = image_buffer.dimensions();
            let rgba = image_buffer.into_raw();

            Some(egui::IconData {
                rgba,
                width: width as u32,
                height: height as u32,
            })
        }
        Err(e) => {
            eprintln!("Failed to load icon from {}: {}", logo_path, e);
            None
        }
    }
}
