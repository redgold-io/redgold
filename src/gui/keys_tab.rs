use eframe::egui;
use eframe::egui::{TextEdit, Ui};
use itertools::Itertools;
use crate::gui::app_loop::LocalState;
use crate::gui::tables::text_table;
use crate::util::cli::commands::{generate_mnemonic, generate_random_mnemonic};

pub struct KeygenState {
    random_window_open: bool,
    random_mnemonic: Option<String>
}

impl KeygenState {
    pub fn new() -> Self {
        Self {
            random_window_open: false,
            random_mnemonic: None,
        }
    }
}
pub fn keys_screen(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    ui.heading("Keygen");
    ui.separator();
    let keygen_state = &mut local_state.keygen_state;
    ui.spacing();
    ui.label("Utilities");
    ui.spacing();
    ui.horizontal(|ui| {
        if ui.button("Generate random mnemonic").clicked() {
            keygen_state.random_mnemonic = Some(generate_random_mnemonic().to_string());
            keygen_state.random_window_open = true;
        }
        ui.spacing();
    });

    egui::Window::new("Mnemonic")
        .open(&mut keygen_state.random_window_open)
        // .resizable(true)
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.label("Generated with internal random entropy");
            ui.separator();
            let mut string = keygen_state.random_mnemonic.clone().unwrap_or("".to_string());
            let split = string.split(" ")
                .enumerate()
                .map(|(i, s)| format!("{:?}: {}", i+1, s))
                .collect_vec().chunks(6)
                .map(|chunk| chunk.to_vec())
                .collect_vec()
                .clone();
            text_table(ui, split);
            ui.separator();
            let edit = TextEdit::multiline(&mut string);
            ui.add(edit);
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
