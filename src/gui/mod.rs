use eframe::{egui, epi};
// 0.17.1
// 0.8
use crate::gui::app_loop::{LocalState, Tab};
use crate::gui::image_load::Image;

pub mod app_loop;
pub mod image_load;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct ClientApp {
    tab: Tab,
    #[cfg_attr(feature = "persistence", serde(skip))]
    logo: Image,
    #[cfg_attr(feature = "persistence", serde(skip))]
    local_state: LocalState,
}

impl ClientApp {
    pub fn from_logo(logo: Image) -> Self {
        Self {
            tab: Tab::Wallet,
            logo: logo,
            local_state: LocalState::default(),
        }
    }
}

impl epi::App for ClientApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        app_loop::app_update(self, ctx, frame);
    }

    /// Called by the framework to load old app state (if any).
    #[cfg(feature = "persistence")]
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
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

    fn name(&self) -> &str {
        "Redgold"
    }
}
//             //     "https://github.com/emilk/egui_template/blob/master/",
