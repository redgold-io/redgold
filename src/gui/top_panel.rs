use eframe::egui;
use eframe::egui::{ComboBox, Context, RichText};
use redgold_gui::common::copy_button;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::util::times::ToTimeString;
use crate::gui::app_loop::LocalState;

fn round_down_to_minute(time_millis: i64) -> i64 {
    time_millis - (time_millis % 60000)
}

pub fn render_top(ctx: &Context, local_state: &mut LocalState) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let cur = ctx.pixels_per_point();
            let string = format!("Pixels per point: {}", cur);
            // ui.text_style_height(&TextStyle::Small);
            // TODO: Make button smaller
            if ui.small_button("+Text")
                .on_hover_text(string.clone()).clicked() {
                ctx.set_pixels_per_point(cur + 0.25);
            }

            if ui.small_button("-Text")
                .on_hover_text(string).clicked() {
                ctx.set_pixels_per_point(cur - 0.25);
            }

            ui.label("Network: ");
            ComboBox::from_label("")
                .width(80.0)
                .selected_text(local_state.node_config.network.to_std_string())
                .show_ui(ui, |ui| {
                    for style in NetworkEnvironment::gui_networks() {
                        ui.selectable_value(&mut local_state.node_config.network, style.clone(), style.to_std_string());
                    }
                });
            ui.label("Network Health:");
            match local_state.home_state.network_healthy {
                true => {
                    ui.label(RichText::new("Healthy").color(egui::Color32::GREEN));
                }
                false => {
                    ui.label(RichText::new("Unhealthy").color(egui::Color32::RED));
                }
            }
            ui.label("Int Time:");
            let current_time = local_state.current_time.to_string();
            let rounded = (round_down_to_minute(local_state.current_time)/1000).to_string();
            ui.label(RichText::new(rounded).color(egui::Color32::KHAKI));
            copy_button(ui, current_time);
            ui.label("Date:");
            let time_str = local_state.current_time.to_time_string_shorter_no_seconds_am_pm();
            ui.label(RichText::new(time_str.clone()).color(egui::Color32::LIGHT_BLUE));
            copy_button(ui, local_state.current_time.to_time_string_shorter());
            ui.hyperlink_to("Explorer Link", local_state.node_config.network.explorer_link());
        });


        // The top panel is often a good place for a menu bar:
        // egui::menu::bar(ui, |ui| {
        //     ui.style_mut().override_text_style = Some(TextStyle::Heading);
        //     egui::menu::menu(ui, "File", |ui| {
        //         ui.style_mut().override_text_style = Some(TextStyle::Heading);
        //         if ui.button("Quit").clicked() {
        //             frame.quit();
        //         }
        //     });
        // });
    });
}
