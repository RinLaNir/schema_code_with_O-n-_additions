use eframe::egui::{RichText, ScrollArea, Ui};
use egui_plot as plot;
use crate::benchmark::{BenchmarkSummary, Implementation};
use crate::ui::localization::Localization;
use crate::ui::constants::{self, heading_size, small_size};
use std::cmp::Ordering;

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
            if let Some(summary) = &self.summary.clone() {
                ui.heading(RichText::new(self.localization.get("chart_title")).size(heading_size(ui)));
                ui.add_space(constants::SMALL_SPACING);
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.localization.get("chart_type_label")).strong());
                    ui.add_space(constants::SMALL_SPACING);
                    
                    let bar_label = self.localization.get("chart_type_bar");
                    let line_label = self.localization.get("chart_type_line");
                    
                    if ui.selectable_label(self.chart_type == ChartType::Bar, bar_label).clicked() {
                        self.chart_type = ChartType::Bar;
                    }
                    if ui.selectable_label(self.chart_type == ChartType::Line, line_label).clicked() {
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
    
    fn sort_entries<'a>(&self, entries: &mut Vec<(&'a crate::benchmark::BenchmarkParams, &'a crate::benchmark::BenchmarkStats)>) {
        entries.sort_by(|a, b| {
            let decoder_a = format!("{:?}", a.0.decoder_type);
            let decoder_b = format!("{:?}", b.0.decoder_type);
            let decoder_cmp = decoder_a.cmp(&decoder_b);
            if decoder_cmp != Ordering::Equal {
                return decoder_cmp;
            }
            
            let rate_a = format!("{:?}", a.0.ldpc_rate);
            let rate_b = format!("{:?}", b.0.ldpc_rate);
            let rate_cmp = rate_a.cmp(&rate_b);
            if rate_cmp != Ordering::Equal {
                return rate_cmp;
            }
            
            match (a.0.implementation, b.0.implementation) {
                (Implementation::Sequential, Implementation::Parallel) => Ordering::Less,
                (Implementation::Parallel, Implementation::Sequential) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        });
    }
    
    fn show_bar_chart(&self, ui: &mut Ui, summary: &BenchmarkSummary, plot_height: f32) {
        ui.push_id("bar_chart_section", |ui| {
            let max_time_ms = summary.total_stats.values()
                .map(|stats| stats.avg.as_millis() as f64)
                .fold(0.0, f64::max);
            
            let y_max = max_time_ms * 1.2;
            
            let plot = plot::Plot::new("bar_chart_plot")
                .height(plot_height)
                .legend(plot::Legend::default())
                .y_axis_width(4)
                .y_axis_label(RichText::new(self.localization.get("axis_time_ms")).size(small_size(ui)))
                .x_axis_label(RichText::new(self.localization.get("axis_parameters")).size(small_size(ui)))
                .allow_zoom(true)
                .allow_drag(true)
                .allow_scroll(true)
                .view_aspect(2.0)
                .show_x(true)
                .show_y(true)
                .include_y(0.0);
            
            let impl_sequential = self.localization.get("impl_sequential").to_string();
            let impl_parallel = self.localization.get("impl_parallel").to_string();
            let legend_sequential = self.localization.get("legend_sequential").to_string();
            let legend_parallel = self.localization.get("legend_parallel").to_string();
            let chart_comparison_title = self.localization.get("chart_comparison_title").to_string();
            
            let title_size = heading_size(ui);
            let label_size = small_size(ui);
            
            plot.show(ui, |plot_ui| {
                let mut entries: Vec<_> = summary.total_stats.iter().collect();
                self.sort_entries(&mut entries);
                
                let mut seq_values = Vec::new();
                let mut par_values = Vec::new();
                
                let mut param_labels = Vec::new();
                let mut bar_index = 0.0;
                
                for (params, stats) in entries {
                    let avg_ms = stats.avg.as_millis() as f64;
                    
                    let param_label = format!("{:?}_{:?}_{:?}", 
                        params.ldpc_rate, 
                        params.ldpc_info_size, 
                        params.decoder_type);
                    param_labels.push((bar_index, param_label.clone()));
                    
                    let impl_name = match params.implementation {
                        Implementation::Sequential => &impl_sequential,
                        Implementation::Parallel => &impl_parallel,
                    };
                    let bar_value = plot::Bar::new(bar_index, avg_ms)
                        .name(format!("{} ({}): {:.2} ms", impl_name, param_label, avg_ms))
                        .width(0.7);
                    
                    match params.implementation {
                        Implementation::Sequential => seq_values.push(bar_value),
                        Implementation::Parallel => par_values.push(bar_value),
                    }
                    
                    bar_index += 1.0;
                }
                
                let intervals = 10;
                for i in 0..=intervals {
                    let value = (y_max * i as f64) / intervals as f64;
                    let label = if value >= 1000.0 {
                        format!("{:.1} s", value / 1000.0)
                    } else {
                        format!("{:.0} ms", value)
                    };
                    plot_ui.hline(plot::HLine::new(value).style(plot::LineStyle::dashed_dense()));
                    
                    plot_ui.text(plot::Text::new(
                        plot::PlotPoint::new(-0.3, value),
                        RichText::new(label).size(10.0)
                    ));
                }
                
                if !seq_values.is_empty() {
                    let seq_chart = plot::BarChart::new(seq_values)
                        .name(&legend_sequential)
                        .color(constants::sequential_color());
                    
                    plot_ui.bar_chart(seq_chart);
                }
                
                if !par_values.is_empty() {
                    let par_chart = plot::BarChart::new(par_values)
                        .name(&legend_parallel) 
                        .color(constants::parallel_color());
                    
                    plot_ui.bar_chart(par_chart);
                }
                
                plot_ui.text(
                    plot::Text::new(
                        plot::PlotPoint::new(bar_index as f64 / 2.0, y_max * 1.1),
                        RichText::new(&chart_comparison_title).size(title_size).strong()
                    )
                );
                
                if !param_labels.is_empty() {
                    for (x, label) in param_labels {
                        plot_ui.text(
                            plot::Text::new(
                                plot::PlotPoint::new(x, -y_max * 0.05), 
                                RichText::new(&label).size(label_size)
                            )
                        );
                    }
                }
                
                plot_ui.set_plot_bounds(plot::PlotBounds::from_min_max(
                    [-0.5, -y_max * 0.1], [(bar_index + 0.5) as f64, y_max * 1.15]
                ));
            });
        });
    }
    
    fn show_line_chart(&self, ui: &mut Ui, summary: &BenchmarkSummary, plot_height: f32) {
        ui.push_id("line_chart_section", |ui| {
            let max_time_ms = summary.total_stats.values()
                .map(|stats| stats.avg.as_millis() as f64)
                .fold(0.0, f64::max);
            
            let y_max = max_time_ms * 1.2;
            
            let plot = plot::Plot::new("line_chart_plot")
                .height(plot_height)
                .legend(plot::Legend::default())
                .y_axis_width(4)
                .y_axis_label(RichText::new(self.localization.get("axis_time_ms")).size(small_size(ui)))
                .x_axis_label(RichText::new(self.localization.get("axis_parameters")).size(small_size(ui)))
                .allow_zoom(true)
                .allow_drag(true)
                .allow_scroll(true)
                .view_aspect(2.0)
                .show_x(true)
                .show_y(true)
                .include_y(0.0);
            
            let legend_sequential = self.localization.get("legend_sequential").to_string();
            let legend_parallel = self.localization.get("legend_parallel").to_string();
            let chart_comparison_title = self.localization.get("chart_comparison_title").to_string();
            
            let title_size = heading_size(ui);
            let label_size = small_size(ui);
            
            plot.show(ui, |plot_ui| {
                let mut entries: Vec<_> = summary.total_stats.iter().collect();
                self.sort_entries(&mut entries);
                
                let mut seq_points: Vec<[f64; 2]> = Vec::new();
                let mut par_points: Vec<[f64; 2]> = Vec::new();
                
                let mut param_labels = Vec::new();
                let mut config_index = 0.0;
                
                let mut configs_seen = std::collections::HashMap::new();
                
                for (params, stats) in &entries {
                    let avg_ms = stats.avg.as_millis() as f64;
                    
                    let config_key = format!("{:?}_{:?}_{:?}_{}", 
                        params.ldpc_rate, 
                        params.ldpc_info_size, 
                        params.decoder_type,
                        params.c_value);
                    
                    let x_index = if let Some(&idx) = configs_seen.get(&config_key) {
                        idx
                    } else {
                        let idx = config_index;
                        configs_seen.insert(config_key.clone(), idx);
                        
                        let param_label = format!("{:?}_{:?}_{:?}", 
                            params.ldpc_rate, 
                            params.ldpc_info_size, 
                            params.decoder_type);
                        param_labels.push((idx, param_label));
                        
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
                
                let intervals = 10;
                for i in 0..=intervals {
                    let value = (y_max * i as f64) / intervals as f64;
                    let label = if value >= 1000.0 {
                        format!("{:.1} s", value / 1000.0)
                    } else {
                        format!("{:.0} ms", value)
                    };
                    plot_ui.hline(plot::HLine::new(value).style(plot::LineStyle::dashed_dense()));
                    
                    plot_ui.text(plot::Text::new(
                        plot::PlotPoint::new(-0.3, value),
                        RichText::new(label).size(10.0)
                    ));
                }
                
                if !seq_points.is_empty() {
                    let seq_line = plot::Line::new(plot::PlotPoints::from(seq_points.clone()))
                        .name(&legend_sequential)
                        .color(constants::sequential_color())
                        .width(2.5);
                    plot_ui.line(seq_line);
                    
                    let seq_markers = plot::Points::new(plot::PlotPoints::from(seq_points))
                        .name(&legend_sequential)
                        .color(constants::sequential_color())
                        .radius(5.0);
                    plot_ui.points(seq_markers);
                }
                
                if !par_points.is_empty() {
                    let par_line = plot::Line::new(plot::PlotPoints::from(par_points.clone()))
                        .name(&legend_parallel)
                        .color(constants::parallel_color())
                        .width(2.5);
                    plot_ui.line(par_line);
                    
                    let par_markers = plot::Points::new(plot::PlotPoints::from(par_points))
                        .name(&legend_parallel)
                        .color(constants::parallel_color())
                        .radius(5.0);
                    plot_ui.points(par_markers);
                }
                
                plot_ui.text(
                    plot::Text::new(
                        plot::PlotPoint::new(config_index as f64 / 2.0, y_max * 1.1),
                        RichText::new(&chart_comparison_title).size(title_size).strong()
                    )
                );
                
                if !param_labels.is_empty() {
                    for (x, label) in param_labels {
                        plot_ui.text(
                            plot::Text::new(
                                plot::PlotPoint::new(x, -y_max * 0.05), 
                                RichText::new(&label).size(label_size)
                            )
                        );
                    }
                }
                
                plot_ui.set_plot_bounds(plot::PlotBounds::from_min_max(
                    [-0.5, -y_max * 0.1], [(config_index + 0.5) as f64, y_max * 1.15]
                ));
            });
        });
    }
}
