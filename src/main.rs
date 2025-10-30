mod app;

use eframe::egui;
use std::fs;

use crate::app::PrologApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Daviti's Prolog Parser"),
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
