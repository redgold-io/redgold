use eframe::egui;
use eframe::egui::{ComboBox, Context, Ui};
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use crate::gui::app_loop::LocalState;
use redgold_gui::common::{editable_text_input_copy, medium_data_item, valid_label};
use crate::gui::components::xpub_req;
use crate::gui::tabs::transact::{cold_wallet, hot_wallet, wallet_tab};
use crate::gui::tabs::transact::wallet_tab::WalletTab;
use crate::hardware::trezor;

