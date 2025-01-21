use std::collections::HashMap;
use eframe::{egui, Frame};
use eframe::egui::Image;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_schema::structs::{ErrorInfo, PublicKey};
use redgold_schema::structs;
use crate::gui::app_loop::LocalState;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::party::party_internal_data::PartyInternalData;
use crate::gui::ls_ext::local_state_from;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node_config::ApiNodeConfig;

pub mod app_loop;
pub mod initialize;
pub mod tabs;
pub mod top_panel;
pub mod webcam;
pub mod qr_render;
pub mod components;
pub mod qr_window;
pub mod native_gui_dependencies;
pub mod lock_screen;
pub mod ls_ext;
pub mod gui_update;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct ClientApp<G, E> where G: GuiDepends + Clone + Send + 'static + Sync, E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    #[cfg_attr(feature = "persistence", serde(skip))]
    // logo: Image<'static>,
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub local_state: LocalState<E>,
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub gui_depends: G,
}

impl<G,E> eframe::App for ClientApp<G,E > where G: GuiDepends + Clone + Send + 'static + Sync, E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        gui_update::app_update(self, ctx, frame);
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
