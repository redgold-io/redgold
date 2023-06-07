use eframe::egui;
use eframe::egui::{Context, Separator, TextEdit, Ui, Widget};
use itertools::Itertools;
use redgold_schema::util::mnemonic_builder;
use redgold_schema::util::mnemonic_words::MnemonicWords;
use crate::gui::app_loop::LocalState;
use crate::gui::tables::text_table;
use crate::gui::util::valid_label;
use crate::gui::wallet_tab::{data_item, medium_data_item};
use crate::util::address_external::{ToBitcoinAddress, ToEthereumAddress};
use crate::util::cli::commands::{generate_mnemonic, generate_random_mnemonic};
use crate::util::keys::ToPublicKeyFromLib;


// TODO: implement a passphrase checksum as well.
// Recalculate these values on change of passphrase
pub struct MnemonicWindowState {
    open: bool,
    words: String,
    label: String,
    bitcoin_p2wpkh_44: String,
    bitcoin_p2wpkh_84: String,
    ethereum_address_44: String,
    ethereum_address_84: String,
    words_checksum: String,
    seed_checksum: Option<String>,
    passphrase: Option<String>,
    redgold_node_address: String,
    redgold_hardware_default_address: String,
    passphrase_input: String,
    passphrase_input_show: bool,
    requires_reset: bool,
}

impl MnemonicWindowState {

    pub fn set_words_from_passphrase(&mut self) {
        let passphrase = self.passphrase.clone();
        let w = MnemonicWords::from_mnemonic_words(
            &*self.words.clone(), passphrase.clone()
        );
        self.bitcoin_p2wpkh_44 = w.btc_key_44_0().public_key.to_struct_public_key().to_bitcoin_address().unwrap();
        self.bitcoin_p2wpkh_84 = w.btc_key_84_0().public_key.to_struct_public_key().to_bitcoin_address().unwrap();
        self.ethereum_address_44 = w.eth_key_44_0().public_key.to_struct_public_key().to_ethereum_address().unwrap();
        self.ethereum_address_84 = w.eth_key_84_0().public_key.to_struct_public_key().to_ethereum_address().unwrap();
        self.words_checksum = w.words_checksum().unwrap();
        self.seed_checksum = passphrase.and(w.seed_checksum().ok());
        self.redgold_node_address = w.address().render_string().unwrap();
        self.redgold_hardware_default_address = w.hardware_default_address().render_string().unwrap();
    }

    pub fn set_words(&mut self, words: impl Into<String>, label: impl Into<String>) {
        self.open = true;
        self.words = words.into();
        self.label = label.into();
        self.set_words_from_passphrase();
    }
}

pub struct GenerateMnemonicState {
    random_input_mnemonic: String,
    random_input_requested: bool,
    password_input: String,
    show_password: bool,
    num_rounds: String
}

pub struct KeygenState {
    mnemonic_window_state: MnemonicWindowState,
    generate_mnemonic_state: GenerateMnemonicState,
}

impl KeygenState {
    pub fn new() -> Self {
        Self {
            mnemonic_window_state: MnemonicWindowState {
                open: false,
                words: "".to_string(),
                label: "".to_string(),
                bitcoin_p2wpkh_44: "".to_string(),
                bitcoin_p2wpkh_84: "".to_string(),
                ethereum_address_44: "".to_string(),
                ethereum_address_84: "".to_string(),
                words_checksum: "".to_string(),
                seed_checksum: None,
                passphrase: None,
                redgold_node_address: "".to_string(),
                redgold_hardware_default_address: "".to_string(),
                passphrase_input: "".to_string(),
                passphrase_input_show: false,
                requires_reset: false,
            },
            generate_mnemonic_state: GenerateMnemonicState {
                random_input_mnemonic: "".to_string(),
                random_input_requested: false,
                password_input: "".to_string(),
                show_password: false,
                num_rounds: "0".to_string(),
            },
        }
    }
}

pub fn keys_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    ui.heading("Keygen");
    ui.separator();
    let key = &mut local_state.keygen_state;
    ui.spacing();
    ui.label("Utilities");
    ui.spacing();
    ui.horizontal(|ui| {
        if ui.button("Generate random mnemonic").clicked() {
            key.mnemonic_window_state.set_words(
                generate_random_mnemonic().to_string(),
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

    ui.label("Generate mnemonic using password derivation");

    ui.horizontal(|ui| {
        let edit = TextEdit::multiline(
            &mut key.generate_mnemonic_state
                .password_input
        )
            .password(!key.generate_mnemonic_state.show_password)
            .lock_focus(false)
            .desired_width(400f32)
            .desired_rows(2);
        let response = ui.add(edit);
        if ui.small_button("Show").on_hover_text("Show password").clicked() {
            key.generate_mnemonic_state.show_password = !key.generate_mnemonic_state.show_password;
        };
    });

    ui.horizontal(|ui| {
        ui.label("Number of rounds");
        ui.add(TextEdit::singleline(&mut key.generate_mnemonic_state.num_rounds)
            .desired_width(150f32)
        );
            // .desired_width(100f32);
        let num_rounds = key.generate_mnemonic_state.num_rounds.parse::<u32>();
        valid_label(ui, num_rounds.is_ok());
        // TODO: Validation
    });


    if ui.button("Generate Password Mnemonic").clicked() {
        let string = key.generate_mnemonic_state.password_input.clone();
        // TODO validate on password length
        // TODO: Hover text on valid button showing reason?
        if !string.is_empty() {
            let rounds = key.generate_mnemonic_state.num_rounds.parse::<u32>()
                .unwrap_or(0) as usize;
            let mnemonic = mnemonic_builder::from_str_rounds(&*string.clone(), rounds).to_string();
            key.mnemonic_window_state.set_words(mnemonic.to_string(), "Generated from password");
        }
        // This is the original function used, need a switch around this to different ones.
    };

    let mnem = key.mnemonic_window_state.words.clone();
    mnemonic_window(
        ctx, &mut key.mnemonic_window_state
    );


}

fn mnemonic_window(
    ctx: &Context, state: &mut MnemonicWindowState
) {

    if state.requires_reset {
        state.set_words_from_passphrase();
        state.requires_reset = false;
    }
    egui::Window::new("Mnemonic")
        .open(&mut state.open)
        .resizable(false)
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
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
                    medium_data_item(ui, "Redgold m/44'/0'/50'/0/0' Address", state.redgold_hardware_default_address.clone());
                    medium_data_item(ui, "Redgold m/84'/16180'/0'/0/0 Address", state.redgold_node_address.clone());
                    medium_data_item(ui, "Bitcoin m/44'/0'/0'/0/0 P2WPKH Address", state.bitcoin_p2wpkh_44.clone());
                    medium_data_item(ui, "Bitcoin m/84'/0'/0'/0/0 P2WPKH Address", state.bitcoin_p2wpkh_84.clone());
                    medium_data_item(ui, "Ethereum m/44'/60'/0'/0/0 Address", state.ethereum_address_44.clone());
                    medium_data_item(ui, "Ethereum m/84'/60'/0'/0/0 Address", state.ethereum_address_84.clone());
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
                let edit = TextEdit::multiline(&mut string)
                    .desired_width(400f32);
                ui.add(edit);
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
