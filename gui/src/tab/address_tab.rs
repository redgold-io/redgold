use crate::common::{data_item, editable_text_input_copy};
use crate::components::address_input_box::{AddressInputBox, AddressInputMode};
use crate::dependencies::gui_depends::GuiDepends;
use eframe::egui::{Context, Ui};
use itertools::Itertools;
use redgold_schema::conf::local_stored_state::SavedAddress;
use redgold_schema::ShortString;
use redgold_schema::structs::SupportedCurrency;
use crate::components::tables::text_table_advanced;

#[derive(Clone, Debug)]
pub struct AddressTabState {
    pub show_new_address: bool,
    pub new_address: String,
    pub address_name: String,
    pub contact_name: String,
    pub related_keysource_input: String,
    pub address_network: SupportedCurrency,
    pub new_address_input: AddressInputBox,
    pub show_account_name: bool
}

#[derive(Clone, Debug, Default)]
pub struct AddressTabDelta {
    pub add_new_address: Option<SavedAddress>,
    pub delete_address: Option<SavedAddress>
}

impl Default for AddressTabState {
    fn default() -> Self {
        Self {
            show_new_address: false,
            new_address: "".to_string(),
            address_name: "".to_string(),
            contact_name: "".to_string(),
            related_keysource_input: "".to_string(),
            address_network: SupportedCurrency::Redgold,
            new_address_input: Default::default(),
            show_account_name: false,
        }
    }
}

impl AddressTabState {

    pub fn saved_address(&self) -> SavedAddress {
        let string = if self.related_keysource_input.is_empty() {
            None
        } else {
            Some(self.related_keysource_input.clone())
        };
        SavedAddress {
            name: self.address_name.clone(),
            address: self.new_address_input.address.render_string().unwrap(),
            contact_name: self.contact_name.clone(),
            related_keysource: string,
        }
    }
    pub fn address_tab<G>(&mut self, ui: &mut Ui, _ctx: &Context, g: &G) -> AddressTabDelta
    where
        G: GuiDepends + Clone + 'static + Send
    {

        // TODO: Move these to state init;
        self.new_address_input.allow_mode_change = false;
        self.new_address_input.address_input_mode = AddressInputMode::Raw;

        let mut delta = AddressTabDelta::default();

        ui.horizontal(|ui| {

            ui.heading("Address Book");
            if let Some(p) = g.config_df_path_label() {
                data_item(ui, "Config Path", p);
            };

        });
        ui.separator();

        if ui.button("Enter New Address").clicked() {
            self.show_new_address = !self.show_new_address;
        }

        if self.show_new_address {
            self.new_address_input.view(ui, vec![], g);
            editable_text_input_copy(ui, "Address Name", &mut self.address_name, 200.);
            editable_text_input_copy(ui, "Contact Name", &mut self.contact_name, 200.);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_account_name, "(Optional) Account Name");
                if self.show_account_name {
                    editable_text_input_copy(ui, "", &mut self.related_keysource_input, 200.);
                }
            });
        }
        if self.new_address_input.valid {
            if ui.button("Add & Save").clicked() {
                delta.add_new_address = Some(self.saved_address());
            }
        }

        let saved = g.get_config().local.unwrap_or_default().saved_addresses.unwrap_or_default();
        if saved.len() > 0 {
            let mut data = vec![vec![
                "Name".to_string(),
                "Contact".to_string(),
                "Account".to_string(),
                "Address".to_string()
            ].iter().map(|x| x.to_string()).collect_vec()];
            for s in saved.clone() {
                let mut row = vec![];
                row.push(s.name);
                row.push(s.contact_name);
                row.push(s.related_keysource.unwrap_or("".to_string()));
                row.push(s.address);
                data.push(row);
            }
            let network = g.get_network().clone();
            let func = move |ui: &mut Ui, row: usize, col: usize, val: &String| {
                let mut column_idxs = vec![3];
                if column_idxs.contains(&col) {
                    ui.hyperlink_to(val.first_four_last_four_ellipses().unwrap_or("err".to_string()), network.explorer_hash_link(val.clone()));
                    return true
                }
                false
            };
            let event = text_table_advanced(ui, data, true, true, None, vec![], Some(func));
            if let Some(id) = event.delete_row_id {
                let delete = saved.get(id).unwrap().clone();
                delta.delete_address = Some(delete);
            }
        }

        delta
    }
}