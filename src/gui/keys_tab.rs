use eframe::egui;
use eframe::egui::{Context, Separator, TextEdit, Ui};
use itertools::Itertools;
use redgold_schema::util::mnemonic_builder;
use crate::gui::app_loop::LocalState;
use crate::gui::tables::text_table;
use crate::gui::util::valid_label;
use crate::util::cli::commands::{generate_mnemonic, generate_random_mnemonic};

pub struct MnemonicWindowState {
    mnemonic_window_open: bool,
    mnemonic_window_value: String,
    mnemonic_window_label: String,
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
                mnemonic_window_open: false,
                mnemonic_window_value: "".to_string(),
                mnemonic_window_label: "".to_string(),
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
            key.mnemonic_window_state.mnemonic_window_value =
                generate_random_mnemonic().to_string();
            key.mnemonic_window_state.mnemonic_window_open = true;
            key.mnemonic_window_state.mnemonic_window_label =
                "Generated with internal random entropy".to_string();
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
            key.mnemonic_window_state.mnemonic_window_value = mnemonic.to_string();
            key.mnemonic_window_state.mnemonic_window_open = true;
            key.mnemonic_window_state.mnemonic_window_label =
                "Generated from password".to_string();
        }
        // This is the original function used, need a switch around this to different ones.
    };

    let mnem = key.mnemonic_window_state.mnemonic_window_value.clone();
    mnemonic_window(
        ctx,
        &mnem,
        &mut key.mnemonic_window_state.mnemonic_window_open,
        &key.mnemonic_window_state.mnemonic_window_label,
    );


}

fn mnemonic_window<S: Into<String>>(
    ctx: &Context, display_words: &String, open: &mut bool, label: S
) {
    egui::Window::new("Mnemonic")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .min_width(500.0)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(label.into());
                // ui.add(Separator::default().spacing(400f32));
                let mut string = display_words.clone();
                let split = string.split(" ")
                    .enumerate()
                    .map(|(i, s)| format!("{:?}: {}", i + 1, s))
                    .collect_vec().chunks(6)
                    .map(|chunk| chunk.to_vec())
                    .collect_vec()
                    .clone();
                text_table(ui, split);
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
