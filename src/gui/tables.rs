use eframe::egui;
use eframe::egui::Ui;
use egui_extras::{Column, TableBuilder};

pub fn text_table(ui: &mut Ui, data: Vec<Vec<String>>) {

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
