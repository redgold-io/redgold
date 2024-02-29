use std::env::current_dir;
use eframe::egui;
use eframe::egui::{Context, Ui};
use egui_extras::{Column, TableBuilder};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;
use crate::gui::common::{bounded_text_area_size, bounded_text_area_size_id, editable_text_input_copy};
use crate::gui::tables::text_table;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtpMetadata {
    randomness_source: String,
    megabytes_per_part: usize,
    num_parts: usize,
    id: String,
    source_contact: String,
    contact: String,
    file_use_metadata: Vec<OtpFileUseMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtpMessage {
    pad_id: String,
    part: usize,
    byte_offset: usize,
    hex_contents: String
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtpFileUseMetadata {
    id: String,
    part: usize,
    filename: String,
    used_offset: usize,
    byte_length: usize,
}

#[derive(Clone, Debug)]
pub struct OtpState {
    sub_tab: OtpSubTab,
    randomness_source: String,
    megabytes_per_part: String,
    num_parts: String,
    id: String,
    source_contact: String,
    contact: String,
    otp_dir: String,
    status: Option<String>,
    use_hot_mnemonic: bool,
    input_message: String,
    output_message: String,
    byte_offset: String,
    part: String,
    awaiting_qr_input: bool
}

impl Default for OtpState {
    fn default() -> Self {
        let cwd = current_dir().expect("cd");
        let dir = cwd.to_str().expect("str").to_string();
        Self {
            sub_tab: OtpSubTab::Receive,
            randomness_source: "/dev/urandom".to_string(),
            megabytes_per_part: "1".to_string(),
            num_parts: "1".to_string(),
            id: "debug_otp_id".to_string(),
            source_contact: "".to_string(),
            contact: "".to_string(),
            otp_dir: dir.to_string(),
            status: None,
            use_hot_mnemonic: false,
            input_message: "".to_string(),
            output_message: "".to_string(),
            byte_offset: "0".to_string(),
            part: "0".to_string(),
            awaiting_qr_input: false,
        }
    }
}

use std::fs::File;
use std::io::Read;
use std::io;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use redgold_schema::{EasyJson, EasyJsonDeser, error_info, RgResult};

fn get_random_from_device(device_path: impl Into<String>) -> io::Result<Vec<u8>> {
    let mut file = File::open(device_path.into())?;
    let mut buffer = [0u8; 1024]; // u32 is 4 bytes
    file.read_exact(&mut buffer)?;
    Ok(buffer.to_vec())
}

#[derive(EnumIter, Clone, Debug, EnumString)]
enum OtpSubTab {
    Receive,
    Transmit,
    Keygen,
    // KeyLoad
}

fn otp_xor(msg: &Vec<u8>, pad: &Vec<u8>) -> RgResult<Vec<u8>> {
    if msg.len() != pad.len() {
        return Err(error_info("The length of the pad must be the same as the message."));
    }

    Ok(msg.iter().zip(pad.iter()).map(|(a, b)| a ^ b).collect_vec())
}

pub(crate) fn otp_tab(ui: &mut Ui, _ctx: &Context, ls: &mut LocalState) {
    ui.heading("One Time Pad Tab");
    ui.horizontal(|ui| {
       for t in OtpSubTab::iter() {
           if ui.button(format!("{:?}", t)).clicked() {
               ls.otp_state.sub_tab = t;
           }
       }
    });
    // TODO: change with callback, enable_and(callback)
    if ls.otp_state.awaiting_qr_input {
        if let Some(c) = &ls.qr_state.contents {
            let decode = c.json_from::<OtpMessage>();
            match decode {
                Ok(m) => {
                    ls.otp_state.input_message = m.hex_contents.clone();
                    ls.otp_state.byte_offset = m.byte_offset.to_string();
                    ls.otp_state.part = m.part.to_string();
                    ls.otp_state.id = m.pad_id.clone();
                }
                Err(_) => {}
            }
            ls.otp_state.awaiting_qr_input = false;
        }
    }
    match ls.otp_state.sub_tab {
        OtpSubTab::Receive => {
            ui.heading("Receive OTP Message Decryption");

            // TODO: Deterministic RSA
            // use_hot_mnemonic(ui, ls);
            ui.label("Encrypted Message Input");
            if ui.button("Scan QR Input").clicked() {
                ls.otp_state.awaiting_qr_input = true;
                ls.qr_state.enable();
            }
            otp_pad_logic(ui, ls, "Decrypt", false);
        }
        OtpSubTab::Transmit => {
            ui.heading("Encrypt OTP Message for Transmission");
            otp_pad_logic(ui, ls, "Encrypt", true);
            if ui.button("Render QR").clicked() {
                ls.qr_show_state.enable(ls.otp_state.output_message.clone());
            }
        }
        OtpSubTab::Keygen => {
            ui.heading("OTP Keygen");
            editable_text_input_copy(ui, "Randomness Source", &mut ls.otp_state.randomness_source, 300.);
            editable_text_input_copy(ui, "MiB Per Part", &mut ls.otp_state.megabytes_per_part, 300.);
            editable_text_input_copy(ui, "Num Parts", &mut ls.otp_state.num_parts, 300.);
            ui.horizontal(|ui| {
                editable_text_input_copy(ui, "Id", &mut ls.otp_state.id, 300.);
                if ui.button("New Id").clicked() {
                    ls.otp_state.id = Uuid::new_v4().to_string();
                }
            });
            editable_text_input_copy(ui, "Source Contact", &mut ls.otp_state.source_contact, 300.);
            editable_text_input_copy(ui, "Contact", &mut ls.otp_state.contact, 300.);
            editable_text_input_copy(ui, "Output Dir", &mut ls.otp_state.otp_dir, 300.);

            if ui.button("Generate").clicked() {
                let mb = ls.otp_state.megabytes_per_part.parse::<usize>().ok();
                let parts = ls.otp_state.num_parts.parse::<usize>().ok();
                if let (Some(mb), Some(parts)) = (mb, parts) {
                    let output_dir = format!("{}/{}", ls.otp_state.otp_dir, ls.otp_state.id);
                    let mut file_use_md = vec![];
                    for part_num in 0..parts {
                        let mut part = vec![];
                        for _ in 0..(1024 * mb) {
                            let randomness = get_random_from_device(&ls.otp_state.randomness_source).ok().unwrap_or(vec![]);
                            part.extend(randomness);
                        }
                        std::fs::create_dir_all(output_dir.clone()).expect("Create dir failure");
                        let fnm = format!("{}/{}.otp", ls.otp_state.id, part_num);
                        let byte_length = part.len();
                        std::fs::write(fnm.clone(), part).expect("Write failure");

                        file_use_md.push(OtpFileUseMetadata{
                            id: ls.otp_state.id.clone(),
                            part: part_num,
                            filename: fnm.clone(),
                            used_offset: 0,
                            byte_length
                        });
                    }
                    let md = OtpMetadata{
                        randomness_source: ls.otp_state.randomness_source.clone(),
                        megabytes_per_part: mb,
                        num_parts: parts,
                        id: ls.otp_state.id.clone(),
                        source_contact: ls.otp_state.source_contact.clone(),
                        contact: ls.otp_state.contact.clone(),
                        file_use_metadata: file_use_md,
                    };
                    std::fs::write(format!("{}/_metadata.json", output_dir),
                                   serde_json::to_string(&md).expect("Serialize failure"))
                        .expect("Write failure");
                }
            }
        }
    }
}

fn otp_pad_logic(ui: &mut Ui, ls: &mut LocalState, button_name: impl Into<String>, do_encrypt: bool) {
    let name = button_name.into();
    bounded_text_area_size_id(ui, &mut ls.otp_state.input_message, 300., 5, format!("{}_input", name.clone()));
    editable_text_input_copy(ui, "Input Dir", &mut ls.otp_state.otp_dir, 300.);
    editable_text_input_copy(ui, "Id", &mut ls.otp_state.id, 300.);
    editable_text_input_copy(ui, "Part", &mut ls.otp_state.part, 300.);
    editable_text_input_copy(ui, "Byte Offset", &mut ls.otp_state.byte_offset, 300.);

    if ui.button(name.clone()).clicked() {
        let id = ls.otp_state.id.clone();
        let part = ls.otp_state.part.parse::<usize>().ok();
        let byte_offset = ls.otp_state.byte_offset.parse::<usize>().ok();
        if let (Some(part), Some(byte_offset)) = (part, byte_offset) {
            let metadata_file = format!("{}/{}/_metadata.json", ls.otp_state.otp_dir, id);
            let metadata = std::fs::read_to_string(metadata_file.clone())
                .expect("read").json_from::<OtpMetadata>();
            // TODO: Enable for not debug mode
            // .and_then(|m| {
            //     if m.file_use_metadata.get(part)
            //         .map(|f| f.used_offset > byte_offset)
            //         .unwrap_or(false) {
            //         Ok(m)
            //     } else {
            //         Err(error_info("Offset already used".to_string()))
            //     }
            // });
            match metadata {
                Ok(m) => {
                    let file = format!("{}/{}/{}.otp", ls.otp_state.otp_dir, id, part);
                    let msg = if !do_encrypt {
                        hex::decode(ls.otp_state.input_message.clone()).expect("hex decode")
                    } else {
                        ls.otp_state.input_message.clone().into_bytes()
                    };
                    let byte_end = byte_offset + msg.len();
                    let bytes = std::fs::read(file).expect("read")[byte_offset..byte_end].to_vec();
                    let decrypted = otp_xor(&msg, &bytes).expect("works");
                    let mut updated = m.clone();
                    let md = updated.file_use_metadata.get_mut(part);
                    md.expect("asdf").used_offset = byte_end;
                    std::fs::write(metadata_file, updated.json_or()).expect("write failure");
                    ls.otp_state.output_message = if !do_encrypt {
                        String::from_utf8(decrypted).expect("utf8")
                    } else {
                        let hx = hex::encode(decrypted);
                        let m = OtpMessage {
                            pad_id: id,
                            part,
                            byte_offset,
                            hex_contents: hx,
                        };
                        m.json_or()
                    }
                }
                Err(_) => {
                    ls.otp_state.status = Some("Metadata not found or already used".to_string());
                }
            }
        }
    }

    bounded_text_area_size_id(ui, &mut ls.otp_state.output_message,300., 5, format!("{}_output", name));
}

fn use_hot_mnemonic(ui: &mut Ui, ls: &mut LocalState) {
    ui.checkbox(&mut ls.otp_state.use_hot_mnemonic, "Use Hot PGP Key");
    if ls.otp_state.use_hot_mnemonic {
        ui.horizontal(|ui| {
            ui.label("Mnemonic checksum");
            ui.label(ls.wallet_state.hot_mnemonic().checksum().expect("ok"));
        });
    }
}