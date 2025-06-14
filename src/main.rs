mod app;
mod compiler;
mod gui;
mod types;

use crate::app::LndwApp;

const APP_NAME: &str = "Lange Nacht der Wissenschaften";

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(LndwApp::new(cc)))),
    );
}
