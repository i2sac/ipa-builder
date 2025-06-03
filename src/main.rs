mod app;
mod ipa_logic;
mod metrics;
mod config_utils;

use app::IpaBuilderApp;
use std::sync::Arc;
use egui::IconData;

fn load_icon_data() -> Result<IconData, Box<dyn std::error::Error>> {
    let image_bytes = std::fs::read("assets/img/ipa.png")?;
    let image = image::load_from_memory(&image_bytes)?;
    let rgba_image = image.to_rgba8();
    let (width, height) = rgba_image.dimensions();
    Ok(IconData {
        rgba: rgba_image.into_raw(),
        width,
        height,
    })
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Initialize logger
    log::info!("Starting IPA Builder application");

    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0]) // Default window size
        .with_min_inner_size([600.0, 400.0]); // Minimum window size

    match load_icon_data() {
        Ok(icon_data) => {
            viewport_builder = viewport_builder.with_icon(Arc::new(icon_data));
        }
        Err(e) => {
            log::warn!("Failed to load application icon 'assets/img/ipa.png': {}. Using default icon.", e);
        }
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    eframe::run_native(
        "IPA Builder",
        options,
        Box::new(|cc| {
            // Attempt to load previously saved app state
            let app_state = match config_utils::load_app_state(cc) {
                Ok(state) => state,
                Err(e) => {
                    log::warn!("Failed to load app state: {}. Using default.", e);
                    let mut app = IpaBuilderApp::default();
                    app.post_load_setup(cc);
                    app
                }
            };
            Box::new(app_state)
        }),
    )
}
