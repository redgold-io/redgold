use std::sync::Arc;
use eframe::{egui};
use crate::gui::egui::IconData;
use egui_extras::RetainedImage;
use tokio::runtime::Runtime;
use redgold_schema::{error_info, ErrorInfoContext};
use redgold_schema::structs::ErrorInfo;
use crate::gui;
use crate::gui::ClientApp;
use crate::node_config::NodeConfig;

pub(crate) fn load_icon() -> IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../resources/svg_rg_2_resized_220_2.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        println!("Loaded icon image with width {} height {}", width, height);
        (rgba, width, height)
    };

    IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

pub async fn attempt_start(nc: NodeConfig
                           // , rt: Arc<Runtime>
) -> Result<(), ErrorInfo> {
    let resources = crate::resources::Resources::default();
    let bytes = resources.logo_bytes;
    let ri = RetainedImage::from_image_bytes("logo", &*bytes).expect("img");
    let app = gui::ClientApp::from(ri, nc).await?;
    let native_options = eframe::NativeOptions::default();
    // native_options. = Some(egui::Vec2::new(1024., 632.));
    // native_options.
    

    // Doesn't seem to work?
    // native_options.icon_data = Some(load_icon());

    eframe::run_native(
        "Redgold",
        native_options,
        Box::new(|_cc| Box::<ClientApp>::new(app))
    ).map_err(|e| error_info(format!("GUI failed to start: {}", e.to_string())))
}