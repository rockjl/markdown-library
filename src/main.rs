// Copyright (c) 2025 markdown-library
// SPDX-License-Identifier: MIT

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod asr;
mod attachments;
mod debug_log;
mod editor_actions;
mod export;
mod find_replace;
mod highlight;
mod note;
mod search;
mod settings;
mod storage;
mod theme;
mod toc;
mod ui;
mod voice;
mod watcher;
mod wikilinks;

use ui::MarkdownApp;

const ICON_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));

/// Decode the embedded PNG icon into egui's `IconData`.
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

/// Application entry point. Creates the native window and runs the egui event loop.
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
