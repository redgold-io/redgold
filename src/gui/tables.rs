use eframe::egui;
use eframe::egui::Ui;
use egui_extras::{Column, TableBuilder};


#[derive(Clone)]
pub struct TextTableEvent {
    pub delete_row_id: Option<usize>,
}

pub fn text_table(ui: &mut Ui, data: Vec<Vec<String>>) {
    text_table_advanced(ui, data, false, false);
}
pub fn text_table_advanced(ui: &mut Ui, data: Vec<Vec<String>>, delete_button: bool, show_empty_headers: bool) -> TextTableEvent {

    let mut event = TextTableEvent {
        delete_row_id: None,
    };

    if data.len() < 2 && !show_empty_headers {
        return event;
    }

    let mut headers = data.get(0).expect("").clone();
    if delete_button {
        headers.push("".to_string());
    }
    let columns = headers.len();

    let text_height = 25.0;
    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
        // .min_scrolled_height(0.0);

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
            let data_ri = row_index + 1;
            let row_data = data.get(data_ri).expect("value row missing");
            for cell in row_data {
                row.col(|ui| {
                    ui.label(cell);
                    ui.spacing();
                });
            }
            if delete_button {
                row.col(|ui| {
                    if ui.button("Delete").clicked() {
                        event.delete_row_id = Some(row_index);
                    }
                });
            }
        });
    });
    event
}
