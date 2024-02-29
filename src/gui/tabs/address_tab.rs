use eframe::egui;
use eframe::egui::{Context, Ui};
use egui_extras::{Column, TableBuilder};
use redgold_schema::structs::SupportedCurrency;
use crate::gui::app_loop::LocalState;
use crate::gui::common::editable_text_input_copy;
use crate::gui::tables::text_table;


pub fn data_table(ui: &mut Ui, data: Vec<Vec<String>>) {

    if data.len() == 0 {
        return;
    }

    let headers = data.get(0).expect("").clone();
    let columns = headers.len();

    let text_height = 25.0;
    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(0.0);

    for _ in 0..columns {
        table = table.column(Column::auto());
    };

    table
        .header(text_height, |mut header| {
            for h in headers {
                header.col(|ui| {
                    ui.strong(h);
                });
            }
        }).body(|body| {
        body.rows(text_height, data.len() - 1, |mut row| {
            let row_index = row.index();
            let row_data = data.get(row_index + 1).expect("value row missing");
            for cell in row_data {
                row.col(|ui| {
                    ui.label(cell);
                    ui.spacing();
                });
            }
        });
    });

}

#[derive(Clone, Debug)]
pub struct AddressState {
    pub show_new_address: bool,
    pub new_address: String,
    pub address_name: String,
    pub contact_name: String,
    pub xpub_name: String,
    pub address_network: SupportedCurrency
}

impl Default for AddressState {
    fn default() -> Self {
        Self {
            show_new_address: false,
            new_address: "".to_string(),
            address_name: "".to_string(),
            contact_name: "".to_string(),
            xpub_name: "".to_string(),
            address_network: SupportedCurrency::Redgold,
        }
    }
}

pub(crate) fn address_tab(ui: &mut Ui, _ctx: &Context, ls: &mut LocalState) {
    ui.heading("Address Tab");

    let ast = &mut ls.address_state;

    if ui.button("Add New Address").clicked() {
        ast.show_new_address = !ast.show_new_address;
    }

    if ast.show_new_address {
        editable_text_input_copy(ui, "Address", &mut ast.new_address, 400.);
        editable_text_input_copy(ui, "Address Name", &mut ast.address_name, 400.);
        editable_text_input_copy(ui, "Contact Name", &mut ast.contact_name, 400.);
        // editable_text_input_copy(ui, "Xpub Name", &mut ls.new_address, 400.);
    }


}