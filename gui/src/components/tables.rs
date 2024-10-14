use std::collections::HashMap;
use eframe::egui;
use eframe::egui::{Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder};

#[derive(Clone)]
pub struct TextTableEvent {
    pub delete_row_id: Option<usize>,
}

pub fn table_nonetype() -> Option<fn(&mut Ui, usize, usize, &String) -> bool> {
    None::<fn(&mut Ui, usize, usize, &String) -> bool>
}

pub fn text_table(ui: &mut Ui, data: Vec<Vec<String>>) {
    text_table_advanced(ui, data, false, false, None, vec![], table_nonetype());
}

pub fn text_table_advanced(
    ui: &mut Ui,
    data: Vec<Vec<String>>,
    delete_button: bool,
    show_empty_headers: bool,
    link_column_and_replacement_text: Option<(usize, Vec<String>)>,
    green_fields: Vec<(usize, usize)>,
    opt_fn: Option<impl Fn(&mut Ui, usize, usize, &String) -> bool>
) -> TextTableEvent where
    {

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

    let id = headers.join("_");

    egui::Frame::none().show(ui, |ui| {
        ui.push_id(id, |ui| {
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
                    for (column_idx, cell) in row_data.iter().enumerate() {
                        if let Some((link_column, replacement_text)) = &link_column_and_replacement_text {
                            if column_idx == *link_column {
                                row.col(|ui| {
                                    ui.hyperlink_to(replacement_text.get(row_index).expect("replacement text missing"), cell);
                                });
                                continue;
                            }
                        }
                        let is_green = green_fields.iter().any(|(ridx, column_index)| {
                            *ridx == row_index && column_idx == *column_index
                        });
                        row.col(|ui| {

                            // here's the main one
                            let mut skip_normal = false;

                            if let Some(f) = &opt_fn {
                                if f(ui, row_index, column_idx, cell) {
                                    skip_normal = true;
                                }
                            }
                            if !skip_normal {
                                if is_green {
                                    ui.label(RichText::new(cell).color(Color32::GREEN));
                                } else {
                                    ui.label(cell);
                                }
                            }


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
        })
    });
    event
}
