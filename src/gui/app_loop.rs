#![allow(dead_code)]
use eframe::egui::widgets::TextEdit;
use eframe::egui::{Align, TextStyle};
use eframe::{egui, epi};

use crate::util::sym_crypt;
// 0.8
use crate::gui::image_load::TexMngr;
use crate::gui::ClientApp;
use crate::util;
use rand::Rng;
use redgold_schema::util::{dhash_vec, mnemonic_builder};

pub struct LocalState {
    session_salt: [u8; 32],
    session_password_hashed: Option<[u8; 32]>,
    session_locked: bool,
    // This is only used by the text box and should be cleared immediately
    password_entry: String,
    // This is only used by the text box and should be cleared immediately
    wallet_passphrase_entry: String,
    // wallet_words_entry: String,
    active_passphrase: Option<String>,
    password_visible: bool,
    show_mnemonic: bool,
    visible_mnemonic: Option<String>,
    // TODO: Encrypt these with session password
    // TODO: Allow multiple passphrase, i'm too lazy for now
    // stored_passphrases: HashMap<String, String>,
    stored_passphrase: Vec<u8>,
    // stored_mnemonics: Vec<String>,
    stored_private_key_hexes: Vec<String>,
    iv: [u8; 16],
    wallet_first_load_state: bool,
}

#[allow(dead_code)]
impl LocalState {
    pub(crate) fn default() -> LocalState {
        let iv = sym_crypt::get_iv();
        return LocalState {
            session_salt: random_bytes(),
            session_password_hashed: None,
            session_locked: false,
            password_entry: "".to_string(),
            wallet_passphrase_entry: "".to_string(),
            // wallet_words_entry: "".to_string(),
            active_passphrase: None,
            password_visible: false,
            show_mnemonic: false,
            visible_mnemonic: None,
            stored_passphrase: vec![],
            // stored_passphrases: HashMap::new(),
            // stored_mnemonics: vec![],
            stored_private_key_hexes: vec![],
            iv,
            wallet_first_load_state: true,
        };
    }

    fn encrypt(&self, str: String) -> Vec<u8> {
        return sym_crypt::encrypt(
            str.as_bytes(),
            &self.session_password_hashed.unwrap(),
            &self.iv,
        )
        .unwrap();
    }

    fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        return sym_crypt::decrypt(data, &self.session_password_hashed.unwrap(), &self.iv).unwrap();
    }

    pub fn accept_passphrase(&mut self, pass: String) {
        let encrypted = self.encrypt(pass);
        self.stored_passphrase = encrypted;
    } // https://www.quora.com/Is-it-useful-to-multi-hash-like-10-000-times-a-password-for-an-anti-brute-force-encryption-algorithm-Do-different-challenges-exist

    fn hash_password(&mut self) -> [u8; 32] {
        let mut vec = self.password_entry.as_bytes().to_vec();
        vec.extend(self.session_salt.to_vec());
        return dhash_vec(&vec);
    }
    fn store_password(&mut self) {
        self.session_password_hashed = Some(self.hash_password());
    }
}

fn random_bytes() -> [u8; 32] {
    return rand::thread_rng().gen::<[u8; 32]>();
}

#[derive(Debug)]
#[repr(i32)]
pub enum Tab {
    Home,
    Wallet,
    Trust,
    Node,
    Settings,
}

fn update_lock_screen(app: &mut ClientApp, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
    let ClientApp { local_state, .. } = app;
    egui::CentralPanel::default().show(ctx, |ui| {
        let layout = egui::Layout::top_down(egui::Align::Center);
        ui.with_layout(layout, |ui| {
            ui.add_space(ctx.available_rect().max.y / 3f32);
            ui.heading("Enter session password");
            ui.add_space(20f32);
            let edit = TextEdit::singleline(&mut local_state.password_entry)
                .password(true)
                .lock_focus(true);
            ui.add(edit).request_focus();
            if ctx.input().key_pressed(egui::Key::Enter) {
                if local_state.session_locked {
                    if local_state.session_password_hashed.unwrap() == local_state.hash_password() {
                        local_state.session_locked = false;
                    } else {
                        panic!("Fuck");
                    }
                } else {
                    local_state.store_password();
                }
                local_state.password_entry = "".to_string();
            }
            ();
            //ui.text_edit_singleline(texts);
        });
    });
}

pub fn app_update(app: &mut ClientApp, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
    let ClientApp {
        tab,
        logo,
        local_state,
    } = app;

    // let mut style: egui::Style = (*ctx.style()).clone();
    // style.visuals.widgets.
    //style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    // ctx.set_style(style);
    // Examples of how to create different panels and windows.
    // Pick whichever suits you.
    // Tip: a good default choice is to just keep the `CentralPanel`.
    // For inspiration and more examples, go to https://emilk.github.io/egui
    if local_state.session_password_hashed.is_none() || local_state.session_locked {
        update_lock_screen(app, ctx, frame);
        return;
    }

    // egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
    //     // The top panel is often a good place for a menu bar:
    //     egui::menu::bar(ui, |ui| {
    //         ui.style_mut().override_text_style = Some(TextStyle::Heading);
    //         egui::menu::menu(ui, "File", |ui| {
    //             ui.style_mut().override_text_style = Some(TextStyle::Heading);
    //             if ui.button("Quit").clicked() {
    //                 frame.quit();
    //             }
    //         });
    //     });
    // });

    let img = logo;
    let texture_id = TexMngr::default().texture(frame, "asdf", img).unwrap();

    egui::SidePanel::left("side_panel")
        .resizable(false)
        .show(ctx, |ui| {
            // ui.horizontal(|ui| {
            //     ui.label("Write something: ");
            //     ui.text_edit_singleline(label);
            // });
            // ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));

            //https://github.com/emilk/egui/blob/master/egui_demo_lib/src/apps/http_app.rs
            // ui.image(TextureId::default())
            ui.set_max_width(104f32);

            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::default()),
                |ui| {
                    let scale = 4;
                    let size =
                        egui::Vec2::new((img.size.0 / scale) as f32, (img.size.1 / scale) as f32);
                    // ui.style_mut().spacing.window_padding.y += 20.0f32;
                    ui.add_space(10f32);
                    ui.image(texture_id, size);
                    ui.style_mut().override_text_style = Some(TextStyle::Heading);

                    ui.style_mut().spacing.item_spacing.y = 5f32;
                    ui.add_space(10f32);
                    //
                    // if ui.button("Home").clicked() {
                    //     *tab = Tab::Home;
                    // }

                    if ui.button("Wallet").clicked() {
                        *tab = Tab::Wallet;
                    }

                    if ui.button("Settings").clicked() {
                        *tab = Tab::Wallet;
                    }
                },
            );

            // ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            //     ui.add(
            //         egui::Hyperlink::new("https://github.com/emilk/egui/").text("powered by egui"),
            //     );
            // });
        });

    if ctx.input().key_pressed(egui::Key::Escape) {
        local_state.session_locked = true;
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        match tab {
            Tab::Home => {
                ui.heading("egui template");
            }
            Tab::Wallet => {
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
                    if response.lost_focus() && ctx.input().key_pressed(egui::Key::Enter) {
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
            }
            Tab::Settings => {}
            Tab::Trust => {}
            Tab::Node => {}
        }
        // ui.hyperlink("https://github.com/emilk/egui_template");
        // ui.add(egui::github_link_file!(
        //     "https://github.com/emilk/egui_template/blob/master/",
        //     "Source code."
        // ));
        ui.with_layout(egui::Layout::top_down(Align::BOTTOM), |ui| {
            egui::warn_if_debug_build(ui)
        });
    });

    // sync local data to RDS -- apart from data associated with phrases
    // discuss extra features around confirmation process. p2p negotation, contacts table.
}
