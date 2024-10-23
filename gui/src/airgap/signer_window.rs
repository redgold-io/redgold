use eframe::egui;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use crate::dependencies::gui_depends::AirgapMessage;
use crate::functionality::capture::CaptureLike;
use crate::image_capture::CaptureStream;

#[derive(EnumString, EnumIter, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum AirgapWindowMode {
    DisplayingMessage,
    AwaitingDataReceipt,
}

#[derive(EnumString, EnumIter, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum AirgapTransport {
    Qr(Option<String>),
    File(String)
}

impl Default for AirgapWindowMode {
    fn default() -> Self {
        AirgapWindowMode::DisplayingMessage
    }
}

impl Default for AirgapTransport {
    fn default() -> Self {
        AirgapTransport::Qr(None)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AirgapSignerWindow {
    msg: AirgapMessage,
    mode: AirgapWindowMode,
    transport: AirgapTransport,
    visible: bool,
    file_input: String,
    capture_stream: Option<CaptureStream>
}

impl Default for AirgapSignerWindow {
    fn default() -> Self {
        AirgapSignerWindow {
            msg: AirgapMessage::GetXPubLike(Default::default()),
            mode: Default::default(),
            transport: Default::default(),
            visible: false,
            file_input: "".to_string(),
            capture_stream: None,
        }
    }
}

impl AirgapSignerWindow {
    pub fn view(&mut self, ui: &mut Ui) {
        egui::Window::new("Airgap Send")
            .open(&mut self.visible)
            .resizable(false)
            .default_pos(egui::Pos2::new(0.0, 0.0))
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0))
            .collapsible(false)
            .min_width(500.0)
            .default_width(500.0)
            .show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    match &self.mode {
                        AirgapWindowMode::DisplayingMessage => {
                            ui.label("Displaying Message");
                        }
                        AirgapWindowMode::AwaitingDataReceipt => {
                            ui.label("Awaiting Data Receipt");
                        }
                    }
                    match (&self.transport, &self.mode) {
                        (AirgapTransport::Qr(pref), AirgapWindowMode::DisplayingMessage) => {

                        }
                        AirgapTransport::File(_) => {}
                    }
                    // if let Some(i) = state.qr_show_state.qr_image.as_ref() {
                    //     i.show_scaled(ui, 1.0);
                    // }
                    // if let Some(t) = &mut state.qr_show_state.qr_text.clone() {
                    //     bounded_text_area(ui, t);
                    // }
                });
            });
    }

    pub fn init_qr_capture(&mut self) {
        match &self.transport {
            AirgapTransport::Qr(c) => {
                self.capture_stream = CaptureStream::new(c.clone()).ok();
            }
            _ => {}
        }
    }

    pub fn initialize_with(
        &mut self,
        msg: AirgapMessage,
        transport: AirgapTransport
    ) {
        self.visible = true;
        self.msg = msg;
        self.transport = transport;
        self.mode = AirgapWindowMode::DisplayingMessage;
    }

}