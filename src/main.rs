#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod asr;
mod search;
mod attachments;
mod editor_actions;
mod export;
mod find_replace;
mod highlight;
mod note;
mod settings;
mod storage;
mod theme;
mod toc;
mod voice;
mod wikilinks;
mod watcher;

use app::MarkdownApp;

const ICON_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(ICON_PNG)
        .expect("decode embedded icon")
        .to_rgba8();
    let (width, height) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Markdown Library")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Markdown Library",
        options,
        Box::new(|cc| Ok(Box::new(MarkdownApp::new(cc)))),
    )
}
