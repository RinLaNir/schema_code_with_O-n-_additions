use super::utils::compare_benchmark_params;
use crate::benchmark::{BenchmarkSummary, Implementation};
use crate::ui::constants::{self, heading_size, small_size};
use crate::ui::localization::Localization;
use eframe::egui::{RichText, ScrollArea, Ui};
use egui_plot as plot;

#[derive(Clone, Copy, PartialEq)]
pub enum ChartType {
    Bar,
    Line,
}

#[derive(Clone)]
pub struct VisualizationTab {
    summary: Option<BenchmarkSummary>,
    localization: Localization,
    chart_type: ChartType,
}

// --- helper functions ---

fn draw_grid(plot_ui: &mut plot::PlotUi, y_max: f64) {
    for i in 0..=10_u32 {
        let value = y_max * f64::from(i) / 10.0;
        let label = if value >= 1000.0 {
            format!("{:.1} s", value / 1000.0)
        } else {
            format!("{:.0} ms", value)
        };
        plot_ui.hline(plot::HLine::new("", value).style(plot::LineStyle::dashed_dense()));
        plot_ui.text(plot::Text::new(
            "",
            plot::PlotPoint::new(-0.3, value),
            RichText::new(label).size(10.0),
        ));
    }
}

/// Renders parameter labels below each X-axis tick position.
fn draw_param_labels(
    plot_ui: &mut plot::PlotUi,
    param_labels: &[(f64, String)],
    y_max: f64,
    label_size: f32,
) {
    for (x, label) in param_labels {
        plot_ui.text(plot::Text::new(
            "",
            plot::PlotPoint::new(*x, -y_max * 0.05),
            RichText::new(label.as_str()).size(label_size),
        ));
    }
}

fn draw_chart_title(
    plot_ui: &mut plot::PlotUi,
    title: &str,
    x_center: f64,
    y_max: f64,
    title_size: f32,
) {
    plot_ui.text(plot::Text::new(
        "",
        plot::PlotPoint::new(x_center, y_max * 1.1),
        RichText::new(title).size(title_size).strong(),
    ));
}

// --- VisualizationTab implementation ---

impl VisualizationTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            summary: None,
            localization,
            chart_type: ChartType::Bar,
        }
    }

    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }

    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        self.summary = Some(summary.clone());
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let available_height = ui.available_height();
        let plot_height = (available_height * 0.6).clamp(250.0, 600.0);

        ScrollArea::vertical().show(ui, |ui| {
            if let Some(ref summary) = self.summary {
                ui.heading(
                    RichText::new(self.localization.get("chart_title")).size(heading_size(ui)),
                );
                ui.add_space(constants::SMALL_SPACING);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.localization.get("chart_type_label")).strong());
                    ui.add_space(constants::SMALL_SPACING);

                    let bar_label = self.localization.get("chart_type_bar");
                    let line_label = self.localization.get("chart_type_line");

                    if ui
                        .selectable_label(self.chart_type == ChartType::Bar, bar_label)
                        .clicked()
                    {
                        self.chart_type = ChartType::Bar;
                    }
                    if ui
                        .selectable_label(self.chart_type == ChartType::Line, line_label)
                        .clicked()
                    {
                        self.chart_type = ChartType::Line;
                    }
                });

                ui.add_space(constants::ITEM_SPACING);

                match self.chart_type {
                    ChartType::Bar => self.show_bar_chart(ui, summary, plot_height),
                    ChartType::Line => self.show_line_chart(ui, summary, plot_height),
                }
            }
        });
    }

    fn build_plot(&self, ui: &mut Ui, id: &str, plot_height: f32) -> plot::Plot<'_> {
        plot::Plot::new(id)
            .height(plot_height)
            .legend(plot::Legend::default().position(plot::Corner::LeftTop))
            .y_axis_min_width(4.0)
            .y_axis_label(RichText::new(self.localization.get("axis_time_ms")).size(small_size(ui)))
            .x_axis_label(
                RichText::new(self.localization.get("axis_parameters")).size(small_size(ui)),
            )
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .view_aspect(2.0)
            .show_x(true)
            .show_y(true)
            .include_y(0.0)
    }

    fn show_bar_chart(&self, ui: &mut Ui, summary: &BenchmarkSummary, plot_height: f32) {
        ui.push_id("bar_chart_section", |ui| {
            let max_time_ms = summary
                .total_stats
                .values()
                .map(|stats| stats.avg.as_millis() as f64)
                .fold(0.0, f64::max);
            let y_max = max_time_ms * 1.2;

            let impl_sequential = self.localization.get("impl_sequential").to_string();
            let impl_parallel = self.localization.get("impl_parallel").to_string();
            let legend_sequential = self.localization.get("legend_sequential").to_string();
            let legend_parallel = self.localization.get("legend_parallel").to_string();
            let chart_comparison_title =
                self.localization.get("chart_comparison_title").to_string();
            let title_size = heading_size(ui);
            let label_size = small_size(ui);

            self.build_plot(ui, "bar_chart_plot", plot_height)
                .show(ui, |plot_ui| {
                    let mut entries: Vec<_> = summary.total_stats.iter().collect();
                    entries.sort_by(|a, b| compare_benchmark_params(a.0, b.0));

                    let mut seq_values = Vec::new();
                    let mut par_values = Vec::new();
                    let mut param_labels: Vec<(f64, String)> = Vec::new();
                    let mut bar_index = 0.0_f64;

                    for (params, stats) in entries {
                        let avg_ms = stats.avg.as_millis() as f64;
                        let param_label = format!(
                            "{:?}\n{:?}\n{:?}",
                            params.ldpc_rate, params.ldpc_info_size, params.decoder_type
                        );
                        param_labels.push((bar_index, param_label));

                        let impl_name = match params.implementation {
                            Implementation::Sequential => &impl_sequential,
                            Implementation::Parallel => &impl_parallel,
                        };
                        let tooltip_label = format!(
                            "{:?}_{:?}_{:?}",
                            params.ldpc_rate, params.ldpc_info_size, params.decoder_type
                        );
                        let bar_value = plot::Bar::new(bar_index, avg_ms)
                            .name(format!(
                                "{} ({}): {:.2} ms",
                                impl_name, tooltip_label, avg_ms
                            ))
                            .width(0.7);

                        match params.implementation {
                            Implementation::Sequential => seq_values.push(bar_value),
                            Implementation::Parallel => par_values.push(bar_value),
                        }

                        bar_index += 1.0;
                    }

                    draw_grid(plot_ui, y_max);

                    if !seq_values.is_empty() {
                        plot_ui.bar_chart(
                            plot::BarChart::new(&legend_sequential, seq_values)
                                .color(constants::sequential_color()),
                        );
                    }
                    if !par_values.is_empty() {
                        plot_ui.bar_chart(
                            plot::BarChart::new(&legend_parallel, par_values)
                                .color(constants::parallel_color()),
                        );
                    }

                    draw_chart_title(
                        plot_ui,
                        &chart_comparison_title,
                        bar_index / 2.0,
                        y_max,
                        title_size,
                    );
                    draw_param_labels(plot_ui, &param_labels, y_max, label_size);

                    plot_ui.set_plot_bounds(plot::PlotBounds::from_min_max(
                        [-0.5, -y_max * 0.1],
                        [bar_index + 0.5, y_max * 1.15],
                    ));
                });
        });
    }

    fn show_line_chart(&self, ui: &mut Ui, summary: &BenchmarkSummary, plot_height: f32) {
        ui.push_id("line_chart_section", |ui| {
            let max_time_ms = summary
                .total_stats
                .values()
                .map(|stats| stats.avg.as_millis() as f64)
                .fold(0.0, f64::max);
            let y_max = max_time_ms * 1.2;

            let legend_sequential = self.localization.get("legend_sequential").to_string();
            let legend_parallel = self.localization.get("legend_parallel").to_string();
            let chart_comparison_title =
                self.localization.get("chart_comparison_title").to_string();
            let title_size = heading_size(ui);
            let label_size = small_size(ui);

            self.build_plot(ui, "line_chart_plot", plot_height)
                .show(ui, |plot_ui| {
                    let mut entries: Vec<_> = summary.total_stats.iter().collect();
                    entries.sort_by(|a, b| compare_benchmark_params(a.0, b.0));

                    let mut seq_points: Vec<[f64; 2]> = Vec::new();
                    let mut par_points: Vec<[f64; 2]> = Vec::new();
                    let mut param_labels: Vec<(f64, String)> = Vec::new();
                    let mut config_index = 0.0_f64;
                    let mut configs_seen = std::collections::HashMap::new();

                    for (params, stats) in &entries {
                        let avg_ms = stats.avg.as_millis() as f64;

                        // Each unique (rate, info_size, decoder, ell) configuration gets one X position,
                        // shared by both sequential and parallel implementations.
                        let config_key = format!(
                            "{:?}_{:?}_{:?}_{}",
                            params.ldpc_rate,
                            params.ldpc_info_size,
                            params.decoder_type,
                            params.secret.bit_len
                        );

                        let x_index = if let Some(&idx) = configs_seen.get(&config_key) {
                            idx
                        } else {
                            let idx = config_index;
                            configs_seen.insert(config_key, idx);
                            param_labels.push((
                                idx,
                                format!(
                                    "{:?}\n{:?}\n{:?}",
                                    params.ldpc_rate, params.ldpc_info_size, params.decoder_type
                                ),
                            ));
                            config_index += 1.0;
                            idx
                        };

                        match params.implementation {
                            Implementation::Sequential => seq_points.push([x_index, avg_ms]),
                            Implementation::Parallel => par_points.push([x_index, avg_ms]),
                        }
                    }

                    seq_points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                    par_points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());

                    draw_grid(plot_ui, y_max);

                    if !seq_points.is_empty() {
                        plot_ui.line(
                            plot::Line::new(
                                &legend_sequential,
                                plot::PlotPoints::from(seq_points.clone()),
                            )
                            .color(constants::sequential_color())
                            .width(2.5),
                        );
                        plot_ui.points(
                            plot::Points::new("", plot::PlotPoints::from(seq_points))
                                .color(constants::sequential_color())
                                .radius(5.0),
                        );
                    }
                    if !par_points.is_empty() {
                        plot_ui.line(
                            plot::Line::new(
                                &legend_parallel,
                                plot::PlotPoints::from(par_points.clone()),
                            )
                            .color(constants::parallel_color())
                            .width(2.5),
                        );
                        plot_ui.points(
                            plot::Points::new("", plot::PlotPoints::from(par_points))
                                .color(constants::parallel_color())
                                .radius(5.0),
                        );
                    }

                    draw_chart_title(
                        plot_ui,
                        &chart_comparison_title,
                        config_index / 2.0,
                        y_max,
                        title_size,
                    );
                    draw_param_labels(plot_ui, &param_labels, y_max, label_size);

                    plot_ui.set_plot_bounds(plot::PlotBounds::from_min_max(
                        [-0.5, -y_max * 0.1],
                        [config_index + 0.5, y_max * 1.15],
                    ));
                });
        });
    }
}
