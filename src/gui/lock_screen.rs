// use eframe::egui;
// use eframe::egui::TextEdit;
// use redgold_gui::dependencies::gui_depends::GuiDepends;
// use crate::gui::app_loop::LocalStateAddons;
// use crate::gui::ClientApp;
//
// fn update_lock_screen<G>(app: &mut ClientApp<G>, ctx: &egui::Context) where G: GuiDepends + Clone + Send {
//     let ClientApp { local_state, .. } = app;
//     egui::CentralPanel::default().show(ctx, |ui| {
//         let layout = egui::Layout::top_down(egui::Align::Center);
//         ui.with_layout(layout, |ui| {
//             ui.add_space(ctx.available_rect().max.y / 3f32);
//             ui.heading("Enter session password");
//             ui.add_space(20f32);
//
//             let edit = TextEdit::singleline(&mut local_state.password_entry)
//                 .password(true)
//                 .lock_focus(true);
//             ui.add(edit).request_focus();
//             if ctx.input(|i| { i.key_pressed(egui::Key::Enter)}) {
//                 if local_state.session_locked {
//                     if local_state.session_password_hashed.unwrap() == local_state.hash_password() {
//                         local_state.session_locked = false;
//                     } else {
//                         panic!("Session password state error");
//                     }
//                 } else {
//                     local_state.store_password();
//                 }
//                 local_state.password_entry = "".to_string();
//                 ()
//             };
//             //ui.text_edit_singleline(texts);
//         });
//     });
// }