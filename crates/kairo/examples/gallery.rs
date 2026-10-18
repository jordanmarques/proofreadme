//! The Kairo gallery: all the widgets in their variants and states.
//!
//! ```sh
//! cargo run -p kairo --example gallery                       # opens the window
//! cargo run -p kairo --example gallery -- --capture out.png  # renders an image, headless
//! ```
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::Path;
use std::process::ExitCode;

use kairo::gallery::{self, Gallery};

/// The density of the capture: two pixels per point, like a Retina screen.
const CAPTURE_DENSITY: f32 = 2.0;

struct Window {
    gallery: Gallery,
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.gallery.show(ui));
        });
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match arguments.as_slice() {
        [] => open(),
        [option, path] if option == "--capture" => capture(Path::new(path)),
        _ => Err("usage: gallery [--capture <file.png>]".to_owned()),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gallery: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Opens the gallery in a window.
fn open() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Kairo 回路: gallery").with_inner_size([gallery::SIZE.x + 32.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Kairo",
        options,
        Box::new(|creation| {
            kairo::install(&creation.egui_ctx);
            Ok(Box::new(Window { gallery: Gallery::default() }))
        }),
    )
    .map_err(|error| error.to_string())
}

/// Renders the whole gallery to a PNG file, without opening a window.
fn capture(path: &Path) -> Result<(), String> {
    let mut gallery = Gallery::default();
    let mut harness =
        egui_kittest::Harness::builder().with_size(gallery::SIZE + egui::vec2(32.0, 32.0)).with_pixels_per_point(CAPTURE_DENSITY).wgpu().build_ui(
            |ui| {
                if kairo::ready(ui) {
                    egui::Frame::new().inner_margin(16).show(ui, |ui| gallery.show(ui));
                }
            },
        );
    // A few frames so that the glyph atlas is complete before rendering.
    harness.run_steps(4);
    let image = harness.render()?;
    image.save(path).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    println!("gallery: {} ({} × {} px)", path.display(), image.width(), image.height());
    Ok(())
}
