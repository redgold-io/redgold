use eframe::egui::Ui;
use egui_plot::{Line, Plot};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PortfolioTabState {
    pub time_range: PlotTimeRangeBuckets,
    pub plot_points: Vec<[f64; 2]>,
}

#[derive(PartialEq, Default, Clone, Debug, Serialize, Deserialize, EnumIter, EnumString)]
enum PlotTimeRangeBuckets {
    #[default]
    Week,
    Month,
    Quarter,
    QTD,
    YTD,
    Year,
    All
}

impl PortfolioTabState {
    pub fn view(&mut self, ui: &mut Ui) {
        self.plot_points = (0..100).into_iter().map(|x| {
            [x as f64, x as f64]
        }).collect_vec();
        ui.heading("Portfolio");
        ui.separator();
        let line = Line::new(self.plot_points.clone())
            .name("Balance")
            .width(2.0);

        Plot::new("portfolio_balance")
            .height(300.0)
            .allow_drag(false)
            .allow_zoom(false)
            // .label_formatter(|name, value| {
            //     if !name.is_empty() {
            //         let date = DateTime::<Utc>::from_timestamp(value[0] as i64, 0)
            //             .map(|d| d.format("%Y-%m-%d").to_string())
            //             .unwrap_or_else(|| "Invalid date".to_string());
            //         format!("{}: ${:.2}", date, value[1])
            //     } else {
            //         "".to_string()
            //     }
            // })
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });

    }
}