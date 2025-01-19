use eframe::egui;
use eframe::egui::{Context, Ui};
use egui_extras::{Column, TableBuilder};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;
use redgold_gui::common::editable_text_input_copy;
use redgold_gui::components::tables::text_table;

#[derive(Clone, Debug)]
pub struct RecoveryState {
    sub_tab: RecoverySubtab
}

impl Default for RecoveryState {
    fn default() -> Self {
        Self {
            sub_tab: RecoverySubtab::Receive,
        }
    }
}

#[derive(EnumIter, Clone, Debug)]
enum RecoverySubtab {
    Receive,
    Transmit,
    Keygen,
    KeyLoad
}
//
// pub(crate) fn recovery_tab(ui: &mut Ui, _ctx: &Context, _ls: &mut LocalState<E>) {
//     ui.heading("Recovery");
//     // ui.horizontal(|ui| {
//     //    for t in RecoverySubtab::iter() {
//     //        if ui.button(t.to_string()).clicked() {
//     //            ls.otp_state.sub_tab = t;
//     //        }
//     //    }
//     // });
//     // match ls.otp_state.sub_tab {
//     //     RecoverySubtab::Receive => {
//     //         ui.heading("Receive");
//     //         ui.label("Receive OTP");
//     //     }
//     //     RecoverySubtab::Transmit => {
//     //         ui.heading("Transmit");
//     //         ui.label("Transmit OTP");
//     //     }
//     //     RecoverySubtab::Keygen => {
//     //         ui.heading("Keygen");
//     //         ui.label("Keygen OTP");
//     //     }
//     //     RecoverySubtab::KeyLoad => {
//     //         ui.heading("KeyLoad");
//     //         ui.label("KeyLoad OTP");
//     //     }
//     // }
// }