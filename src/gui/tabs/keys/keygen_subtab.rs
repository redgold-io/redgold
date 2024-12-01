use eframe::egui;
use eframe::egui::{Context, ScrollArea, TextEdit, Ui, Widget};
use itertools::Itertools;
use strum_macros::EnumString;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::util::mnemonic_builder;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::NetworkEnvironment;

use crate::gui::app_loop::{LocalState, LocalStateAddons};
use redgold_gui::common::{copy_to_clipboard, editable_text_input_copy, medium_data_item, valid_label};
use redgold_gui::components::tables::text_table;
use crate::util;
use crate::util::argon_kdf::argon2d_hash;
use crate::util::cli::commands::generate_random_mnemonic;
use crate::util::keys::ToPublicKeyFromLib;

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Clone, EnumString)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum KeyDerivation {
    DoubleSha256,
    Argon2d,
}


#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Clone, EnumString)]
enum Rounds {
    TenK,
    OneM,
    TenM,
    Custom
}


// TODO: implement a passphrase checksum as well.
// Recalculate these values on change of passphrase
#[derive(Clone)]
pub struct MnemonicWindowState {
    pub open: bool,
    pub words: String,
    label: String,
    bitcoin_p2wpkh_84: String,
    ethereum_address_44: String,
    words_checksum: String,
    seed_checksum: Option<String>,
    pub passphrase: Option<String>,
    redgold_node_address: String,
    redgold_hardware_default_address: String,
    passphrase_input: String,
    passphrase_input_show: bool,
    requires_reset: bool,
    hd_path: String,
    private_key_hex: String,
    calc_private_key_hex: bool,
    generation_time_seconds: String,
    exe_checksum: String,
    save_name: String,
    persist_disk: bool,
    set_hot_mnemonic: bool
}

impl MnemonicWindowState {
    pub fn get_private_key_hex(&self) -> String {
        // self.mnemonic_words().private_hex(self.hd_path.clone()).unwrap_or("err".to_string())
        self.words_pass().private_at(self.hd_path.clone()).unwrap()
    }
}

impl MnemonicWindowState {


    pub fn words_pass(&self) -> WordsPass {
        let w = WordsPass::new(
            self.words.clone(), self.passphrase.clone()
        );
        w
    }

    pub fn set_words_from_passphrase(&mut self) {
        let passphrase = self.passphrase.clone();
        let wp = WordsPass::new(
            self.words.clone(), passphrase.clone()
        );
        let md = wp.metadata().expect("metadata error");

        self.bitcoin_p2wpkh_84 = md.btc_84h_0h_0h_0_0_address;
        self.ethereum_address_44 = md.eth_44h_60h_0h_0_0_address;
        self.words_checksum = wp.checksum_words().unwrap();
        self.seed_checksum = passphrase.and(wp.checksum().ok());
        self.redgold_node_address = wp.default_public_key().expect("").address().expect("works").render_string().unwrap();
        let hw_addr = wp.keypair_at("m/44/0/50/0/0").expect("keypair error").address_typed().render_string().unwrap();
        self.redgold_hardware_default_address = hw_addr;
    }

    pub fn set_words(&mut self, words: impl Into<String>, label: impl Into<String>) {
        self.open = true;
        self.words = words.into();
        self.label = label.into();
        self.set_words_from_passphrase();
    }
}
#[derive(Clone)]
pub struct GenerateMnemonicState {
    random_input_mnemonic: String,
    random_input_requested: bool,
    password_input: String,
    show_password: bool,
    num_rounds: String,
    toggle_concat_password: bool,
    toggle_show_metadata: bool,
    num_modular_passwords_input: String,
    num_modular_passwords: u32,
    modular_passwords: Vec<String>,
    concat_password: String,
    metadata_fields: Vec<String>,
    key_derivation: KeyDerivation,
    rounds_type: Rounds,
    salt_words: String,
    m_cost_input: String,
    p_cost_input: String,
    m_cost: Option<u32>,
    p_cost: Option<u32>,
    t_cost: Option<u32>,
    pub t_cost_input: String,
}

impl GenerateMnemonicState {
    pub fn compound_passwords(&mut self) {
        let mod_join = self.modular_passwords.iter().join("");
        let metadata_join = self.metadata_fields.iter()
            .map(|s| s.to_uppercase()).join("");
        self.concat_password = format!("{}{}", mod_join, metadata_join);
    }
}
#[derive(Clone)]
pub struct KeygenState {
    pub(crate) mnemonic_window_state: MnemonicWindowState,
    generate_mnemonic_state: GenerateMnemonicState,
}

impl KeygenState {
    pub fn new(exe_checksum: String) -> Self {
        Self {
            mnemonic_window_state: MnemonicWindowState {
                open: false,
                words: "".to_string(),
                label: "".to_string(),

                bitcoin_p2wpkh_84: "".to_string(),
                ethereum_address_44: "".to_string(),

                words_checksum: "".to_string(),
                seed_checksum: None,
                passphrase: None,
                redgold_node_address: "".to_string(),
                redgold_hardware_default_address: "".to_string(),
                passphrase_input: "".to_string(),
                passphrase_input_show: false,
                requires_reset: false,
                hd_path: "m/44'/5555'/0'/0/0".to_string(),
                private_key_hex: "".to_string(),
                calc_private_key_hex: false,
                generation_time_seconds: "".to_string(),
                exe_checksum,
                save_name: "keygen".to_string(),
                persist_disk: false,
                set_hot_mnemonic: false
            },
            generate_mnemonic_state: GenerateMnemonicState {
                random_input_mnemonic: "".to_string(),
                random_input_requested: false,
                password_input: "".to_string(),
                show_password: false,
                num_rounds: "10000".to_string(),
                toggle_concat_password: false,
                toggle_show_metadata: false,
                num_modular_passwords_input: "6".to_string(),
                num_modular_passwords: 6,
                modular_passwords: (0..6).map(|_| "".to_string()).collect_vec(),
                concat_password: "".to_string(),
                metadata_fields: (0..4).map(|_| "".to_string()).collect_vec(),
                key_derivation: KeyDerivation::Argon2d,
                rounds_type: Rounds::TenK,
                salt_words: "".to_string(),
                m_cost_input: "65536".to_string(),
                p_cost_input: "2".to_string(),
                t_cost_input: "10".to_string(),
                m_cost: Some(65536),
                p_cost: Some(2),
                t_cost: Some(10),
            },
        }
    }
}

pub fn keys_screen_scroll(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    let key = &mut local_state.keygen_state;
    ui.horizontal(|ui| {
        if ui.button("Generate Random Entropy Mnemonic Words").clicked() {
            key.mnemonic_window_state.set_words(
                generate_random_mnemonic().words,
                "Generated with internal random entropy"
            );
        }
        ui.spacing();
    });

    // ui.toggle_value(
    //     &mut key.generate_mnemonic_state.random_input_requested,
    //     "Enable Source Entropy Input",
    // );
    //
    // if key.generate_mnemonic_state.random_input_requested {
    //     // TODO: Toggle here to load manually or load from file
    //     // TODO: abstract function
    //     // let edit = TextEdit::multiline(
    //     //     &mut key.generate_mnemonic_state
    //     //         .random_input_mnemonic
    //     // )
    //     //     .password(!key.generate_mnemonic_state.show_password)
    //     //     .lock_focus(false)
    //     //     .desired_width(400f32)
    //     //     .desired_rows(2);
    // }

    password_derivation(key, ui);

    // let mnem = key.mnemonic_window_state.words.clone();
    mnemonic_window(
        ctx, local_state
    );

}

pub fn keys_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    ui.heading("Keygen");
    ui.separator();
    ScrollArea::vertical().show(ui, |ui| keys_screen_scroll(ui, ctx, local_state));
}

fn password_derivation(key: &mut KeygenState, ui: &mut Ui) {

    ui.separator();
    ui.spacing();
    ui.label("Generate mnemonic using password derivation");

    if !key.generate_mnemonic_state.toggle_concat_password {
        for m in &mut key.generate_mnemonic_state.modular_passwords {
            m.clear();
        }
    }

    if !key.generate_mnemonic_state.toggle_show_metadata {
        for m in &mut key.generate_mnemonic_state.metadata_fields {
            m.clear();
        }
    }

    ui.horizontal(|ui| {
        ui.checkbox(
            &mut key.generate_mnemonic_state.show_password,
            "Show Password",
        );

        ui.checkbox(&mut key.generate_mnemonic_state.toggle_concat_password,
                    "Modular Concat Password"
        );


        if key.generate_mnemonic_state.toggle_concat_password {
            ui.checkbox(&mut key.generate_mnemonic_state.toggle_show_metadata,
                        "Metadata Fields"
            );

            ui.label("Num Passwords");
            TextEdit::singleline(&mut key.generate_mnemonic_state.num_modular_passwords_input)
                .desired_width(30f32).show(ui);
        }

    });
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label("Salt Words");
            valid_label(ui, WordsPass::new(&key.generate_mnemonic_state.salt_words, None).mnemonic().is_ok(), );
        });
        TextEdit::multiline(&mut key.generate_mnemonic_state.salt_words)
            .desired_width(500f32)
            .desired_rows(2)
            .show(ui);
    });

    // TODO: Only validate on update.

    if key.generate_mnemonic_state.toggle_concat_password {
        ui.horizontal(|_ui| {

            if let Some(mut i) = key.generate_mnemonic_state.num_modular_passwords_input.clone().parse::<u32>().ok() {
                if i >= 10 {
                    i = 10;
                }
                let current = key.generate_mnemonic_state.num_modular_passwords;
                if current != i {
                    let diff = i as i32 - current as i32;
                    if diff > 0 {
                        for _ in 0..diff {
                            key.generate_mnemonic_state.modular_passwords.push("".to_string());
                        }
                    } else {
                        for _ in 0..diff.abs() {
                            key.generate_mnemonic_state.modular_passwords.pop();
                        }
                    }
                }
                key.generate_mnemonic_state.num_modular_passwords = i;
            }
        });
        for i in 0..key.generate_mnemonic_state.num_modular_passwords {
            ui.horizontal(|ui| {
                ui.label(format!("Password {}", i + 1));
                TextEdit::multiline(&mut key.generate_mnemonic_state.modular_passwords[i as usize])
                    .password(!key.generate_mnemonic_state.show_password)
                    .lock_focus(false)
                    .desired_width(450f32)
                    .desired_rows(1)
                    .show(ui);
            });
        }
        key.generate_mnemonic_state.password_input = key.generate_mnemonic_state.modular_passwords.join("");
    }

    if key.generate_mnemonic_state.toggle_show_metadata && key.generate_mnemonic_state.toggle_concat_password {
        ui.label("Metadata Fields Used For Increasing Entropy");
        ui.horizontal(|ui| {
            ui.label("First Name:");
            TextEdit::singleline(&mut key.generate_mnemonic_state.metadata_fields[0])
                .desired_width(100f32).show(ui);
            ui.label("Middle Name:");
            TextEdit::singleline(&mut key.generate_mnemonic_state.metadata_fields[1])
                .desired_width(100f32).show(ui);
            ui.label("Last Name:");
            TextEdit::singleline(&mut key.generate_mnemonic_state.metadata_fields[2])
                .desired_width(100f32).show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("Birthdate YYYYMMDD");
            TextEdit::singleline(&mut key.generate_mnemonic_state.metadata_fields[3])
                .desired_width(100f32).show(ui);
        });
    }

    key.generate_mnemonic_state.compound_passwords();

    ui.horizontal(|ui| {
        let mut fixed_concat = key.generate_mnemonic_state.concat_password.clone();
        let text = if key.generate_mnemonic_state.toggle_concat_password {
            &mut fixed_concat
        } else {
            &mut key.generate_mnemonic_state
                .password_input
        };
        ui.label("Password");
        let edit = TextEdit::multiline(text)
            .password(!key.generate_mnemonic_state.show_password)
            .lock_focus(false)
            .desired_width(400f32)
            .desired_rows(2);
        ui.add(edit);

    });

    ui.horizontal(|ui| {

        let radio = &mut key.generate_mnemonic_state.key_derivation;
        ui.label("KDF");
        egui::ComboBox::from_id_source("Key Derive Function")
            .selected_text(format!("{:?}", radio))
            .show_ui(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.set_min_width(60.0);
                ui.selectable_value(radio, KeyDerivation::DoubleSha256, "DSha256");
                ui.selectable_value(radio, KeyDerivation::Argon2d, "Argon2d");
            });

        if key.generate_mnemonic_state.key_derivation == KeyDerivation::DoubleSha256 {
            ui.horizontal(|ui| {
                ui.label("Number of rounds");
                let radio = &mut key.generate_mnemonic_state.rounds_type;
                ui.radio_value(radio, Rounds::TenK, "10k");
                ui.radio_value(radio, Rounds::OneM, "1m");
                ui.radio_value(radio, Rounds::TenM, "10m");
                ui.radio_value(radio, Rounds::Custom, "Custom");
                match radio {
                    Rounds::TenK => {
                        key.generate_mnemonic_state.num_rounds = "10000".to_string();
                    }
                    Rounds::OneM => {
                        key.generate_mnemonic_state.num_rounds = "1000000".to_string();
                    }
                    Rounds::TenM => {
                        key.generate_mnemonic_state.num_rounds = "10000000".to_string();
                    }
                    Rounds::Custom => {
                        ui.add(TextEdit::singleline(&mut key.generate_mnemonic_state.num_rounds)
                            .desired_width(80f32)
                        );
                        let num_rounds = key.generate_mnemonic_state.num_rounds.parse::<u32>();
                        valid_label(ui, num_rounds.is_ok(), );
                    }
                }
            });
        }

        if key.generate_mnemonic_state.key_derivation == KeyDerivation::Argon2d {
            // TODO: Validators
            ui.horizontal(|ui| {
                ui.label("Memory KiB");
                TextEdit::singleline(&mut key.generate_mnemonic_state.m_cost_input)
                    .desired_width(100f32)
                    .show(ui);
                ui.label("Threads");
                TextEdit::singleline(&mut key.generate_mnemonic_state.p_cost_input)
                    .desired_width(100f32)
                    .show(ui);
                ui.label("Iterations");
                TextEdit::singleline(&mut key.generate_mnemonic_state.t_cost_input)
                    .desired_width(100f32)
                    .show(ui);

                key.generate_mnemonic_state.m_cost = key.generate_mnemonic_state.m_cost_input.parse::<u32>().ok();
                key.generate_mnemonic_state.p_cost = key.generate_mnemonic_state.p_cost_input.parse::<u32>().ok();
                key.generate_mnemonic_state.t_cost = key.generate_mnemonic_state.t_cost_input.parse::<u32>().ok();
            });
        }


        // .desired_width(100f32);
        // TODO: Validation
    });

    if ui.button("Generate Password Mnemonic").clicked() {
        let string = get_displayed_password(key);
        // let string = key.generate_mnemonic_state.password_input.clone();
        // TODO validate on password length
        // TODO: Hover text on valid button showing reason?
        if !string.is_empty() {
            match key.generate_mnemonic_state.key_derivation {
                KeyDerivation::DoubleSha256 => {
                    if let Some(rounds) = key.generate_mnemonic_state.num_rounds.parse::<u32>().ok() {
                        let mnemonic = mnemonic_builder::from_str_rounds(
                            &*string.clone(),
                            rounds as usize
                        );
                        key.mnemonic_window_state.set_words(mnemonic, "Generated from password");
                    }
                }
                KeyDerivation::Argon2d => {
                    if let (Some(m_cost), Some(p_cost), Some(t_cost), Some(m)) = (
                        key.generate_mnemonic_state.m_cost,
                        key.generate_mnemonic_state.p_cost,
                        key.generate_mnemonic_state.t_cost,
                        WordsPass::new(&*key.generate_mnemonic_state.salt_words, None).validate().ok()
                    ) {
                        let salt = m.seed().expect("works").to_vec();
                        // let salt = m.to_seed(None).0;
                        tracing::info!("Attempting to run Argon2d with params: {} {} {}", m_cost, p_cost, t_cost);
                        let start = util::current_time_millis_i64();
                        let result = argon2d_hash(salt, string.as_bytes().to_vec(), m_cost, t_cost, p_cost);
                        let end = util::current_time_millis_i64();
                        let delta_seconds = ((end - start) as f64) / 1000.0;
                        tracing::info!("Argon2d took {} seconds", delta_seconds.clone());

                        if let Some(r) = result.ok() {
                            if let Some(w) = WordsPass::from_bytes(&*r).ok() {
                                key.mnemonic_window_state.set_words(w.words,
                                                                    format!("Generated from password Argon2d in {} seconds", delta_seconds));
                                key.mnemonic_window_state.generation_time_seconds = delta_seconds.to_string();
                            }
                        }
                    }
                }
            }

        }
        // This is the original function used, need a switch around this to different ones.
    };

}

fn get_displayed_password(key: &mut KeygenState) -> String {
    let mut fixed_concat = key.generate_mnemonic_state.concat_password.clone();
    let text = if key.generate_mnemonic_state.toggle_concat_password {
        &mut fixed_concat
    } else {
        &mut key.generate_mnemonic_state
            .password_input
    };
    text.clone()
}

pub(crate) fn mnemonic_window(
    ctx: &Context, ls: &mut LocalState
) {

    if ls.keygen_state.mnemonic_window_state.set_hot_mnemonic {
        ls.add_with_pass_mnemonic(ls.keygen_state.mnemonic_window_state.save_name.clone(),
                        ls.keygen_state.mnemonic_window_state.words.clone(),
                        ls.keygen_state.mnemonic_window_state.persist_disk,
                        ls.keygen_state.mnemonic_window_state.passphrase.clone()
        );
        ls.keygen_state.mnemonic_window_state.set_hot_mnemonic = false;
    }

    let salt = &mut ls.keygen_state.generate_mnemonic_state.salt_words;
    let state = &mut ls.keygen_state.mnemonic_window_state;


    if state.requires_reset {
        state.set_words_from_passphrase();
        state.requires_reset = false;
    }
    if state.calc_private_key_hex {
        state.private_key_hex = state.get_private_key_hex();
        state.calc_private_key_hex = false;
    }

    egui::Window::new("Mnemonic")
        .open(&mut state.open)
        .resizable(false)
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
            // Layout doesn't seem to work here.
            // let layout = egui::Layout::top_down(egui::Align::Center);
            // ui.with_layout(layout, |ui| {
                ui.vertical(|ui| {

                    ui.label(state.label.clone());
                    // ui.add(Separator::default().spacing(400f32));
                    let mut string = state.words.clone();
                    let split = string.split(" ")
                        .enumerate()
                        .map(|(i, s)| format!("{:?}: {}", i + 1, s))
                        .collect_vec().chunks(6)
                        .map(|chunk| chunk.to_vec())
                        .collect_vec()
                        .clone();
                    text_table(ui, split);
                    ui.vertical(|ui| {
                        medium_data_item(ui, "RDG m/44'/0'/50'/0/0'", state.redgold_hardware_default_address.clone());
                        medium_data_item(ui, "RDG m/84'/16180'/0'/0/0", state.redgold_node_address.clone());
                        medium_data_item(ui, "BTC m/84'/0'/0'/0/0 P2WPKH", state.bitcoin_p2wpkh_84.clone());
                        medium_data_item(ui, "ETH m/44'/60'/0'/0/0", state.ethereum_address_44.clone());
                        ui.horizontal(|ui| {
                            medium_data_item(ui, "Words Checksum", state.words_checksum.clone());
                            if let Some(c) = &state.seed_checksum {
                                medium_data_item(ui, "Seed Checksum", c.clone());
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Passphrase Input");
                            TextEdit::singleline(&mut state.passphrase_input)
                                .desired_width(150f32)
                                .password(state.passphrase_input_show)
                                .lock_focus(false)
                                .ui(ui);
                            if ui.small_button("Show").on_hover_text("Show password").clicked() {
                                state.passphrase_input_show = !state.passphrase_input_show;
                            };
                            if ui.button("Update").clicked() {
                                let string = state.passphrase_input.clone();
                                if string.is_empty() {
                                    state.passphrase = None;
                                } else {
                                    state.passphrase = Some(string);
                                }
                                state.requires_reset = true;
                            }
                        })
                    });
                    // ui.add(Separator::default().spacing(400f32));
                    ui.horizontal(|ui| {
                        TextEdit::multiline(&mut string)
                            .desired_width(400f32).show(ui);
                        copy_to_clipboard(ui, string.clone());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Get Private Key Hex");
                        ui.label("Path");
                        TextEdit::singleline(&mut state.hd_path)
                            .desired_width(130f32)
                            .ui(ui);
                        if ui.button("Calculate").clicked() {
                           state.calc_private_key_hex = true;
                        }
                        if ui.button("Clear").clicked() {
                            state.private_key_hex = "".to_string();
                        }
                        medium_data_item(ui, "Private Key Hex", state.private_key_hex.clone());
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Save ./metadata.json").clicked() {
                            let words = state.words.clone();
                            let pass = state.passphrase.clone();
                            let wp = WordsPass::new(words, pass);
                            let metadata_json = wp.metadata().expect("metadata error")
                                .with_exe_checksum(state.exe_checksum.clone())
                                .json_pretty_or();
                            std::fs::write("metadata.json", metadata_json)
                                .expect("Unable to write file");
                        }
                        editable_text_input_copy(ui, "Mnemonic Name", &mut state.save_name, 100.0);
                        ui.checkbox(&mut state.persist_disk, "Persist Disk");
                        if ui.button("Set As Hot Mnemonic").clicked() {
                            state.set_hot_mnemonic = true;
                        }
                        if ui.button("Use As Salt").clicked() {
                            *salt = state.words.clone();
                        }
                    });
            });
        });
}


/*
          egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.gallery_grid_contents(ui);
                });
        });
 */



/*
if local_state.active_passphrase.is_none() {
                ui.heading("No wallet loaded.");
                ui.add_space(20f32);

                ui.heading("Enter mnemonic generation phrase:");
                let edit = TextEdit::singleline(&mut local_state.wallet_passphrase_entry)
                    .password(!local_state.password_visible)
                    .lock_focus(true)
                    .desired_width(300f32);
                let response = ui.add(edit);

                if local_state.wallet_first_load_state {
                    response.request_focus();
                    local_state.wallet_first_load_state = false;
                }
                if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let string = local_state.wallet_passphrase_entry.clone();
                    local_state.active_passphrase = Some(string.clone());
                    local_state.visible_mnemonic = Some(
                        mnemonic_builder::from_str_rounds(&*string.clone(), 0).to_string(),
                    );
                    local_state.wallet_passphrase_entry = "".to_string();
                    //local_state.
                    // mnemonic_builder::from_str
                }
                if ui.button("Show password text").clicked() {
                    local_state.password_visible = !local_state.password_visible;
                };
            } else {
                ui.heading("Wallet loaded");

                if ui.button("Show mnemonic").clicked() {
                    local_state.show_mnemonic = !local_state.show_mnemonic;
                }
                if local_state.show_mnemonic {
                    let mut option = local_state.visible_mnemonic.as_ref().unwrap().clone();
                    let edit = TextEdit::multiline(&mut option);
                    ui.add(edit);
                }
            }
 */
