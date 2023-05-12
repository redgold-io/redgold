use std::sync::Arc;
use egui_extras::RetainedImage;
use tokio::runtime::Runtime;
use redgold_schema::{error_info, ErrorInfoContext};
use redgold_schema::structs::ErrorInfo;
use crate::gui;
use crate::gui::ClientApp;
use crate::node_config::NodeConfig;

pub fn attempt_start(nc: NodeConfig, rt: Arc<Runtime>) -> Result<(), ErrorInfo> {
    // TODO: Start GUI
    // use crate::gui::image_load::Image;
    let resources = crate::resources::Resources::default();
    let bytes = resources.logo_bytes;
    // let image = Image::decode(&*bytes).unwrap();
    let ri = RetainedImage::from_image_bytes("logo", &*bytes).expect("img");
    let app = gui::ClientApp::from(ri, nc, rt);
    let native_options = eframe::NativeOptions::default();
    //
    // icon_data: Some(
    //     eframe::IconData::try_from_png_bytes(&include_bytes!("../../../media/icon.png")[..])
    // .unwrap(),

    // ),
    //        Box::new(|_cc| Box::<MyApp>::default()),
    eframe::run_native(
        "Redgold",
        native_options,
        Box::new(|_cc| Box::<ClientApp>::new(app))
    ).map_err(|e| error_info(format!("GUI failed to start: {}", e.to_string())))
}