use eframe::egui::Ui;
use egui_plot::{HPlacement, Line, Plot};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::structs::{NetworkEnvironment, SupportedCurrency};
use redgold_schema::util::dollar_formatter::{format_dollar_amount_brief_with_prefix_and_suffix, format_dollar_amount_with_prefix_and_suffix};
use redgold_schema::util::times::{current_time_millis, ToTimeString};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use itertools::Itertools;
use crate::data_query::data_query::DataQueryInfo;

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
    pub fn view<E>(&mut self, ui: &mut Ui, d: &DataQueryInfo<E>, n: NetworkEnvironment) where E: ExternalNetworkResources + Send + Clone + 'static  + Sync{

        let now = current_time_millis();
        let balances = d.balance_totals(&n, None);
        let historical = d.daily_one_year.lock().unwrap().clone();
        let pev = d.first_party.lock().unwrap().party_events.clone();
        // pev.as_ref().map(|pe| pe.central_price_history)
        let num_points = 60;
        // day;
        let bucket_duration = 1000*60*60*24;
        let start = now - bucket_duration * num_points;

        self.plot_points = (0..60).into_iter().map(|x| {
            let plot_point_time = start + x * bucket_duration;
            let mut nav_usd = 0f64;
            for (b_cur, v) in balances.iter() {
                if let Some(h) = historical.get(&b_cur) {
                    h.iter().filter(|(time, price_usd)| {
                        *time <= plot_point_time
                    }).last().iter().for_each(|(time, price_usd)| {
                        nav_usd += v * price_usd;
                    });
                }
                if b_cur == &SupportedCurrency::Redgold {
                    if let Some(pv) = pev.as_ref() {
                        let p_rdg = pv.get_rdg_max_bid_usd_estimate_at(plot_point_time);
                        if let Some(p) = p_rdg {
                            nav_usd += v * p;
                        }
                    }
                }
            }
            [plot_point_time as f64, nav_usd]
        }).collect_vec();
        ui.heading("Portfolio");
        ui.separator();
        let line = Line::new(self.plot_points.clone())
            .name("Balance")
            .width(2.0);

        Plot::new("portfolio_balance")
            .height(300.0)
            .y_axis_position(HPlacement::Right)
            .allow_drag(false)
            .allow_zoom(false)
            .label_formatter(|name, value| {
                if !name.is_empty() {
                    let date = (value.x as i64).to_time_string_day();
                    let yy = format_dollar_amount_with_prefix_and_suffix(value.y);
                    format!("{}: {}", date, yy)
                } else {
                    "".to_string()
                }
            })
            .x_axis_formatter(|value, range| {
                (value.value as i64).to_time_string_day()
            })
            .y_axis_formatter(|value, range| {
                format_dollar_amount_brief_with_prefix_and_suffix(value.value)
            })
            .show(ui, |plot_ui| {
                plot_ui.line(line);
            });
        ui.label("More options coming soon");
    }
}