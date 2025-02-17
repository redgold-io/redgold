use eframe::egui;
use eframe::egui::{Context, ScrollArea, TextEdit, Ui, Widget};
use itertools::Itertools;
use log::info;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::structs::NetworkEnvironment;
use strum_macros::EnumString;
use redgold_schema::util;
use redgold_schema::util::times::current_time_millis;
use crate::common::{copy_to_clipboard, editable_text_input_copy, medium_data_item, valid_label};
use crate::components::tables::text_table;
use crate::dependencies::gui_depends::GuiDepends;
use crate::state::local_state::{LocalState, LocalStateAddons};
use crate::tab::keys::keygen::{GenerateMnemonicState, KeyDerivation, KeygenState, MnemonicWindowState, Rounds};


pub fn keys_screen_scroll<E, G>(
    ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState<E>, g: &G)
where E: ExternalNetworkResources + Sync + Send + Clone, G: GuiDepends {
    let key = &mut local_state.keygen_state;
    ui.horizontal(|ui| {
        if ui.button("Generate Random Entropy Mnemonic Words").clicked() {
            key.mnemonic_window_state.set_words(
                G::generate_random_mnemonic().words,
                "Generated with internal random entropy",
                g
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

    password_derivation(key, ui, g);

    // let mnem = key.mnemonic_window_state.words.clone();
    mnemonic_window(
        ctx, local_state,g
    );

}

pub fn keys_screen<E, G>(
    ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState<E>, g: &G
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone, G: GuiDepends {
    ui.heading("Keygen");
    ui.separator();
    ScrollArea::vertical().show(ui, |ui| keys_screen_scroll(ui, ctx, local_state, g));
}

fn password_derivation<G>(key: &mut KeygenState, ui: &mut Ui, g: &G) where G: GuiDepends {

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
            let pass = WordsPass::new(&key.generate_mnemonic_state.salt_words, None);
            let v = G::validate_mnemonic(pass).is_ok();
            valid_label(ui, v, );
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
                        let mnemonic = G::mnemonic_builder_from_str_rounds(
                            &string.clone(),
                            rounds as usize
                        ).words;
                        key.mnemonic_window_state.set_words(mnemonic, "Generated from password", g);
                    }
                }
                KeyDerivation::Argon2d => {
                    let pass1 = WordsPass::new(&*key.generate_mnemonic_state.salt_words, None);
                    if let (Some(m_cost), Some(p_cost), Some(t_cost), Some(m)) = (
                        key.generate_mnemonic_state.m_cost,
                        key.generate_mnemonic_state.p_cost,
                        key.generate_mnemonic_state.t_cost,
                        G::validate_mnemonic(pass1.clone()).ok().map(|w| pass1)
                    ) {
                        let seed = G::mnemonic_to_seed(m);
                        let salt = seed;
                        // let salt = m.to_seed(None).0;
                        info!("Attempting to run Argon2d with params: {} {} {}", m_cost, p_cost, t_cost);
                        let start = current_time_millis();
                        let result = G::argon2d_hash(salt, string.as_bytes().to_vec(), m_cost, t_cost, p_cost);
                        let end = current_time_millis();
                        let delta_seconds = ((end - start) as f64) / 1000.0;
                        info!("Argon2d took {} seconds", delta_seconds.clone());

                        if let Some(r) = result.ok() {
                            if let Some(w) = G::words_pass_from_bytes(&*r).ok() {
                                key.mnemonic_window_state.set_words(
                                    w.words,
                                    format!("Generated from password Argon2d in {} seconds", delta_seconds), g);
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

pub fn mnemonic_window<E, G>(
    ctx: &Context, ls: &mut LocalState<E>, g: &G
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone, G: GuiDepends {

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
        state.set_words_from_passphrase(g);
        state.requires_reset = false;
    }
    if state.calc_private_key_hex {
        state.private_key_hex = state.get_private_key_hex(g);
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
                            let mut metadata = G::words_pass_metadata(wp);
                            let metadata_json = metadata.with_exe_checksum(state.exe_checksum.clone())
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
