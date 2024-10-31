use std::collections::HashMap;
use std::path::PathBuf;
use eframe::egui;
use eframe::egui::{ColorImage, Image, TextureHandle, Ui};
use eframe::egui::load::SizedTexture;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_schema::airgap::{AirgapMessage, AirgapResponse, IndexedInputProof, SignInternalResponse, TransactionSignDetails};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs;
use redgold_schema::util::times::{current_time_millis, ToTimeString};
use crate::airgap;
use crate::common::{bounded_text_area, editable_text_input_copy};
use crate::components::combo_box::combo_box;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use crate::functionality::capture::CaptureLike;
use crate::functionality::qr_render::qr_encode_image;
use crate::image_capture::CaptureStream;

#[derive(EnumString, EnumIter, Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum AirgapWindowMode {
    DisplayingMessage,
    AwaitingDataReceipt,
    CompletedDataReceipt
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
    pub cur_message_json: String,
    pub msg: AirgapMessage,
    pub mode: AirgapWindowMode,
    pub transport: AirgapTransport,
    pub visible: bool,
    pub file_write_reference: String,
    pub file_read_reference: String,
    pub capture_stream: Option<CaptureStream>,
    #[serde(skip)]
    pub qr_image: Option<egui::Image<'static>>,
    pub data_result: String,
    pub message_response: Option<AirgapResponse>,
    pub rx_message: Option<AirgapMessage>,
    pub device_name: String,
}

impl Default for AirgapSignerWindow {
    fn default() -> Self {
        AirgapSignerWindow {
            cur_message_json: "".to_string(),
            msg: AirgapMessage::default(),
            mode: Default::default(),
            transport: Default::default(),
            visible: false,
            file_write_reference: "".to_string(),
            file_read_reference: "".to_string(),
            capture_stream: None,
            qr_image: None,
            data_result: "".to_string(),
            message_response: None,
            rx_message: None,
            device_name: "".to_string(),
        }
    }
}

impl AirgapSignerWindow {
    pub fn interior_view<G>(&mut self, ui: &mut Ui, g: &G, sign_info: Option<&TransactionSignInfo>) where G: GuiDepends + Clone + Send
    {
        ui.vertical(|ui| {
            match &self.mode {
                AirgapWindowMode::CompletedDataReceipt => {
                    self.completed_data_receipt(g, sign_info, ui);
                }
                AirgapWindowMode::DisplayingMessage => {
                    bounded_text_area(ui, &mut self.cur_message_json);
                }
                _ => {}
            }
            match &self.transport {
                AirgapTransport::Qr(pref_capture) => {
                    match &self.mode {
                        AirgapWindowMode::DisplayingMessage => {
                            if let Some(img) = &self.qr_image {
                                ui.add(img.clone());
                            } else {
                                ui.label("Missing QR Image data");
                            }
                        },
                        AirgapWindowMode::AwaitingDataReceipt => {
                            if let Some(mut s) = self.capture_stream.as_mut() {
                                let devs = s.get_device_names();
                                if let Ok(devices) = devs {
                                    let name_to_index = devices.iter().enumerate().map(|(i, d)| (d.clone(), i)).collect::<HashMap<String, usize>>();
                                    let changed = combo_box(ui, &mut self.device_name, "Select Capture Device", devices.clone(), false, 300.0, None);
                                    if changed {
                                        let device = name_to_index.get(&self.device_name).map(|i| devices.get(*i).cloned()).flatten();
                                        if device != s.active_device {
                                            if let Some(d) = device {
                                                s.change(d).ok();
                                            }
                                        }
                                    }
                                }
                                let qr = s.read_qr();
                                if let Ok((image, res)) = qr {
                                    // Convert DynamicImage to egui::ColorImage
                                    let color_image = ColorImage::from_rgb(
                                        [image.width() as usize, image.height() as usize],
                                        image.to_rgb8().as_raw()
                                    );

                                    // Create or update the texture
                                    let texture = ui.ctx().load_texture(
                                        "camera_frame",
                                        color_image,
                                        egui::TextureOptions::default()
                                    );
                                    let sized_texture = SizedTexture::new(
                                        texture.id(),
                                        texture.size_vec2()
                                    );

                                    ui.add(Image::new(sized_texture));

                                    if let Ok((meta, data)) = res {
                                        self.finish(data);
                                    }
                                } else {
                                    ui.label("Capture Error");
                                }
                            };
                        }
                        _ => {}
                    }
                }
                AirgapTransport::File(file_out_dir) => {
                    match &self.mode {
                        AirgapWindowMode::DisplayingMessage => {
                            self.transport_selector_and_file_input_box(ui, false);
                            if ui.button("Write To File").clicked() {
                                self.cur_message_json.write_json(&self.file_write_reference).ok();
                            }
                        }
                        AirgapWindowMode::AwaitingDataReceipt => {
                            editable_text_input_copy(ui, "File Read Input", &mut self.file_write_reference, 300.0);

                            ui.label("Awaiting Data Receipt");
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    fn completed_data_receipt<G>(&mut self, g: &G, sign_info: Option<&TransactionSignInfo>, ui: &mut Ui) where G: GuiDepends + Clone + Send {
        ui.label("Data Received");
        if let Some(tsi) = sign_info {
            bounded_text_area(ui, &mut self.rx_message.json_or());
            self.transport_selector_and_file_input_box(ui, true);
            if ui.button("Sign & Render").clicked() {
                if let Some(si) = self.rx_message.as_ref().and_then(|m| m.sign_internal.as_ref()) {
                    let processed = si.txs.iter().filter_map(|tx| {
                        let res = g.sign_transaction(tx, tsi);
                        let post_signed = res.ok().and_then(|r| {
                            let sigs = r.inputs.iter().enumerate().filter_map(|(i, inp)| {
                                let p = inp.proof.clone();
                                if p.is_empty() { None } else {
                                    let mut indexed = IndexedInputProof::default();
                                    indexed.proof = p;
                                    indexed.index = i as i64;
                                    Some(indexed)
                                }
                            }).collect::<Vec<IndexedInputProof>>();
                            if sigs.is_empty() { None } else {
                                let mut ret = TransactionSignDetails::default();
                                ret.hash = Some(tx.hash_or());
                                ret.signatures = sigs;
                                Some(ret)
                            }
                        });
                        post_signed
                    }).collect::<Vec<TransactionSignDetails>>();
                    let mut airgap_response = AirgapResponse::default();
                    let mut sir = SignInternalResponse::default();
                    sir.signed_txs = processed;
                    airgap_response.sign_internal = Some(sir);
                    let hex = airgap_response.proto_serialize_hex();
                    match self.transport {
                        AirgapTransport::Qr(_) => {
                            let img = qr_encode_image(hex);
                            self.qr_image = Some(img);
                        }
                        AirgapTransport::File(_) => {
                            hex.write_json(&self.file_write_reference).ok();
                        }
                    }
                    self.mode = AirgapWindowMode::DisplayingMessage;
                }
            }
        } else {
            bounded_text_area(ui, &mut self.message_response.json_or());
        }
    }

    fn transport_selector_and_file_input_box(&mut self, ui: &mut Ui, show_transport: bool) {
        ui.horizontal(|ui| {
            if show_transport {
                combo_box(ui, &mut self.transport, "Transport", AirgapTransport::iter().collect(), false, 100.0, None);
            }
            match &self.transport {
                AirgapTransport::File(_) => {
                    editable_text_input_copy(ui, "File Output", &mut self.file_write_reference, 300.0);
                }
                _ => {}
            };
        });
    }

    pub fn window_view<G>(&mut self, ui: &mut Ui, g: &G) where G: GuiDepends + Clone + Send {
        let mut is_visible = self.visible;

        egui::Window::new("Airgap Send")
            .open(&mut is_visible)
            .resizable(false)
            .default_pos(egui::Pos2::new(0.0, 0.0))
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 0.0))
            .collapsible(false)
            .min_width(500.0)
            .default_width(500.0)
            .show(ui.ctx(), |ui| {
                self.interior_view(ui, g, None);
            });

        self.visible = is_visible;
    }

    pub fn finish(&mut self, data: String) {
        self.data_result = data.clone();
        self.message_response = data.json_from::<AirgapResponse>().ok();
        self.mode = AirgapWindowMode::CompletedDataReceipt;
        self.visible = false;
    }

    pub fn init_qr_capture(&mut self) {
        match &self.transport {
            AirgapTransport::Qr(c) => {
                self.capture_stream = CaptureStream::new(c.clone()).ok();
                if let Some(s) = self.capture_stream.as_mut() {
                    let name_0 = s.get_device_names().ok()
                        .and_then(|d| d.get(0).cloned());
                    self.device_name = c.clone().or(name_0).unwrap_or("".to_string());
                }
            }
            _ => {}
        }
    }

    pub fn init_qr_image_render(&mut self) {
        match &self.transport {
            AirgapTransport::Qr(c) => {
                let h = self.msg.proto_serialize_hex();
                let img = qr_encode_image(h);
                self.qr_image = Some(img);
            }
            _ => {}

        }
    }

    pub fn message_ending() -> String {
        "request.hex".to_string()
    }

    pub fn response_ending() -> String {
        "response.hex".to_string()
    }

    pub fn init_file_render(&mut self) {
        match &self.transport {
            AirgapTransport::File(f) => {
                let buf = PathBuf::from(f);
                let cur = current_time_millis().to_time_string_shorter_underscores();
                let mut path = buf.join(format!("{}-{}", cur, Self::message_ending()));
                let mut read_path = buf.join(format!("{}-{}", cur, Self::response_ending()));
                self.file_write_reference = path.to_str().unwrap().to_string();
                self.file_read_reference = read_path.to_str().unwrap().to_string();
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
        self.cur_message_json = self.msg.json_or();
        self.init_qr_image_render();
        self.init_file_render();
    }

}