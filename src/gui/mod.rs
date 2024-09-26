use eframe::{egui, Frame};
use eframe::egui::Image;
use egui_extras::RetainedImage;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_schema::structs::ErrorInfo;
// 0.17.1
// 0.8
use crate::gui::app_loop::LocalState;
// use crate::gui::image_load::Image;
use redgold_schema::conf::node_config::NodeConfig;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;

pub mod app_loop;
pub mod image_load;
pub mod initialize;
pub mod tables;
pub mod home;
pub mod tabs;
pub mod top_panel;
pub mod webcam;
pub mod qr_render;
pub mod components;
pub mod airgap;
pub mod qr_window;
pub mod native_gui_dependencies;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct ClientApp<G> where G: GuiDepends + Clone + Send {
    #[cfg_attr(feature = "persistence", serde(skip))]
    logo: Image<'static>,
    #[cfg_attr(feature = "persistence", serde(skip))]
    local_state: LocalState,
    #[cfg_attr(feature = "persistence", serde(skip))]
    gui_depends: G,
}

impl<G> ClientApp<G> where G: GuiDepends + Clone + Send{
    pub async fn from(logo: Image<'static>,
                      nc: NodeConfig,
                      res: ExternalNetworkResourcesImpl,
                      gui_depends: G
    ) -> Result<Self, ErrorInfo> where G: Send + Clone + GuiDepends {
        Ok(Self {
            logo,
            local_state: LocalState::from(nc, res).await?,
            gui_depends,
        })
    }
}

impl<G> eframe::App for ClientApp<G> where G: GuiDepends + Clone + Send {
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
