use eframe::{egui, Frame};
use egui_extras::RetainedImage;
use redgold_schema::structs::ErrorInfo;
// 0.17.1
// 0.8
use crate::gui::app_loop::LocalState;
// use crate::gui::image_load::Image;
use crate::node_config::NodeConfig;

pub mod app_loop;
pub mod image_load;
pub mod initialize;
pub mod tables;
pub mod server_tab;
pub mod home;
pub mod wallet_tab;
pub mod keys_tab;
pub mod common;
pub mod hot_wallet;
pub mod cold_wallet;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct ClientApp {
    #[cfg_attr(feature = "persistence", serde(skip))]
    logo: RetainedImage,
    #[cfg_attr(feature = "persistence", serde(skip))]
    local_state: LocalState,
}

impl ClientApp {
    pub async fn from(logo: RetainedImage, nc: NodeConfig
                // , rt: Arc<Runtime>
    ) -> Result<Self, ErrorInfo> {
        Ok(Self {
            logo,
            local_state: LocalState::from(nc).await?,
        })
    }
}

impl eframe::App for ClientApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        app_loop::app_update(self, ctx, frame);
    }

    /// Called by the framework to load old app state (if any).
    #[cfg(feature = "persistence")]
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut Frame<'_>,
        storage: Option<&dyn epi::Storage>,
    ) {
        if let Some(storage) = storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    // fn name(&self) -> &str {
    //     "Redgold"
    // }
}
//             //     "https://github.com/emilk/egui_template/blob/master/",
