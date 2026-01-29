use eframe::egui::{self, RichText, ScrollArea, Ui};
use crate::benchmark::{BenchmarkSummary, BenchmarkParams, Implementation};
use crate::ui::localization::Localization;
use crate::ui::constants::{self, heading_size, small_size};
use super::utils::format_duration;
use super::table_builder::{ResultsTable, TableColumn};
use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ConfigKey {
    c_value: usize,
    ldpc_info_size: String,
    ldpc_rate: String,
    decoder_type: String,
}

impl ConfigKey {
    fn from_params(params: &BenchmarkParams) -> Self {
        Self {
            c_value: params.c_value,
            ldpc_info_size: format!("{:?}", params.ldpc_info_size),
            ldpc_rate: format!("{:?}", params.ldpc_rate),
            decoder_type: format!("{:?}", params.decoder_type),
        }
    }
    
    #[allow(dead_code)]
    fn display_label(&self) -> String {
        format!("C{} {:?} {:?} {:?}", 
            self.c_value, 
            self.ldpc_rate, 
            self.ldpc_info_size, 
            self.decoder_type)
    }
}

#[derive(Clone)]
struct SpeedupEntry {
    config: ConfigKey,
    seq_time: std::time::Duration,
    par_time: std::time::Duration,
    speedup: f64,
    percent_faster: f64,
    efficiency: f64,
    thread_count: usize,
}

#[derive(Clone)]
pub struct AccelerationTab {
    summary: Option<BenchmarkSummary>,
    localization: Localization,
    selected_configs: HashSet<ConfigKey>,
    all_configs: Vec<ConfigKey>,
    show_all: bool,
}

impl AccelerationTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            summary: None,
            localization,
            selected_configs: HashSet::new(),
            all_configs: Vec::new(),
            show_all: true,
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        let mut configs: HashSet<ConfigKey> = HashSet::new();
        for (params, _) in &summary.total_stats {
            configs.insert(ConfigKey::from_params(params));
        }
        
        let mut configs_vec: Vec<_> = configs.into_iter().collect();
        configs_vec.sort_by(|a, b| {
            let decoder_cmp = a.decoder_type.cmp(&b.decoder_type);
            if decoder_cmp != Ordering::Equal {
                return decoder_cmp;
            }
            let rate_cmp = a.ldpc_rate.cmp(&b.ldpc_rate);
            if rate_cmp != Ordering::Equal {
                return rate_cmp;
            }
            a.c_value.cmp(&b.c_value)
        });
        
        self.all_configs = configs_vec;
        self.summary = Some(summary.clone());
        
        self.show_all = true;
        self.selected_configs.clear();
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        if self.summary.is_none() {
            return;
        }
        
        let summary = self.summary.clone().unwrap();
        
        let mut has_sequential = false;
        let mut has_parallel = false;
        for (params, _) in &summary.total_stats {
            match params.implementation {
                Implementation::Sequential => has_sequential = true,
                Implementation::Parallel => has_parallel = true,
            }
            if has_sequential && has_parallel {
                break;
            }
        }
        
        if !has_sequential || !has_parallel {
            ui.vertical_centered(|ui| {
                ui.add_space(constants::SECTION_SPACING);
                ui.label(RichText::new(self.localization.get("acceleration_no_comparison"))
                    .color(egui::Color32::LIGHT_YELLOW));
            });
            return;
        }
        
        let speedup_data = self.calculate_speedup_data(&summary);
        
        ScrollArea::both().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(180.0);
                    ui.set_max_width(200.0);
                    
                    ui.heading(RichText::new(self.localization.get("config_filter")).size(heading_size(ui)));
                    ui.add_space(constants::SMALL_SPACING);
                    
                    let all_label = self.localization.get("filter_all");
                    if ui.selectable_label(self.show_all, RichText::new(all_label).strong()).clicked() {
                        self.show_all = true;
                        self.selected_configs.clear();
                    }
                    
                    ui.add_space(constants::SMALL_SPACING);
                    ui.separator();
                    ui.add_space(constants::SMALL_SPACING);
                    
                    for config in &self.all_configs.clone() {
                        let is_selected = self.selected_configs.contains(config);
                        let label = format!("C{} {} {} {}", 
                            config.c_value,
                            config.ldpc_rate,
                            config.ldpc_info_size,
                            config.decoder_type);
                        
                        if ui.selectable_label(is_selected && !self.show_all, &label).clicked() {
                            self.show_all = false;
                            if is_selected {
                                self.selected_configs.remove(config);
                                if self.selected_configs.is_empty() {
                                    self.show_all = true;
                                }
                            } else {
                                self.selected_configs.insert(config.clone());
                            }
                        }
                    }
                });
                
                ui.add_space(constants::ITEM_SPACING);
                
                ui.vertical(|ui| {
                    ui.heading(RichText::new(self.localization.get("speedup_info_title")).size(heading_size(ui)));
                    ui.add_space(constants::SMALL_SPACING);
                    
                    let filtered_data: Vec<_> = if self.show_all {
                        speedup_data.clone()
                    } else {
                        speedup_data.iter()
                            .filter(|entry| self.selected_configs.contains(&entry.config))
                            .cloned()
                            .collect()
                    };
                    
                    if filtered_data.is_empty() {
                        ui.label(RichText::new(self.localization.get("no_data_selected"))
                            .color(egui::Color32::LIGHT_YELLOW));
                    } else {
                        self.show_comparison_table(ui, &filtered_data);
                    }
                });
            });
        });
    }
    
    fn calculate_speedup_data(&self, summary: &BenchmarkSummary) -> Vec<SpeedupEntry> {
        let mut speedup_data = Vec::new();
        let thread_count = rayon::current_num_threads();
        
        for (seq_params, seq_stats) in summary.total_stats.iter()
            .filter(|(p, _)| matches!(p.implementation, Implementation::Sequential)) 
        {
            for (par_params, par_stats) in summary.total_stats.iter()
                .filter(|(p, _)| matches!(p.implementation, Implementation::Parallel)) 
            {
                if seq_params.c_value == par_params.c_value &&
                   seq_params.decoder_type == par_params.decoder_type &&
                   seq_params.ldpc_info_size == par_params.ldpc_info_size &&
                   seq_params.ldpc_rate == par_params.ldpc_rate 
                {
                    let speedup = seq_stats.avg.as_secs_f64() / par_stats.avg.as_secs_f64();
                    let percent_faster = (speedup - 1.0) * 100.0;
                    let efficiency = speedup / thread_count as f64 * 100.0;
                    
                    speedup_data.push(SpeedupEntry {
                        config: ConfigKey::from_params(seq_params),
                        seq_time: seq_stats.avg,
                        par_time: par_stats.avg,
                        speedup,
                        percent_faster,
                        efficiency,
                        thread_count,
                    });
                    break;
                }
            }
        }
        
        speedup_data.sort_by(|a, b| {
            let decoder_cmp = a.config.decoder_type.cmp(&b.config.decoder_type);
            if decoder_cmp != Ordering::Equal {
                return decoder_cmp;
            }
            let rate_cmp = a.config.ldpc_rate.cmp(&b.config.ldpc_rate);
            if rate_cmp != Ordering::Equal {
                return rate_cmp;
            }
            a.config.c_value.cmp(&b.config.c_value)
        });
        
        speedup_data
    }
    
    fn show_comparison_table(&self, ui: &mut Ui, data: &[SpeedupEntry]) {
        let columns = vec![
            TableColumn::new(self.localization.get("col_config")).with_min_width(180.0),
            TableColumn::new(self.localization.get("label_sequential")).with_min_width(100.0),
            TableColumn::new(self.localization.get("label_parallel")).with_min_width(100.0),
            TableColumn::new(self.localization.get("label_speedup")).with_min_width(80.0),
            TableColumn::new(self.localization.get("col_percent_faster")).with_min_width(100.0),
            TableColumn::new(self.localization.get("parallel_efficiency")).with_min_width(100.0),
            TableColumn::new(self.localization.get("thread_count")).with_min_width(80.0),
        ];
        
        let data_clone = data.to_vec();
        
        ResultsTable::new("acceleration_comparison_table", columns)
            .show(ui, data_clone.len(), |row_idx, row| {
                let entry = &data_clone[row_idx];
                
                row.col(|ui| {
                    let label = format!("C{} {} {} {}", 
                        entry.config.c_value,
                        entry.config.ldpc_rate,
                        entry.config.ldpc_info_size,
                        entry.config.decoder_type);
                    ui.label(RichText::new(label).size(small_size(ui)));
                });
                
                row.col(|ui| {
                    ui.label(RichText::new(format_duration(entry.seq_time))
                        .color(constants::sequential_color()));
                });
                
                row.col(|ui| {
                    ui.label(RichText::new(format_duration(entry.par_time))
                        .color(constants::parallel_color()));
                });
                
                row.col(|ui| {
                    let speedup_color = constants::speedup_color(ui, entry.speedup);
                    ui.label(RichText::new(format!("{:.2}x", entry.speedup))
                        .color(speedup_color)
                        .strong());
                });
                
                row.col(|ui| {
                    let speedup_color = constants::speedup_color(ui, entry.speedup);
                    ui.label(RichText::new(format!("{:.0}%", entry.percent_faster))
                        .color(speedup_color));
                });
                
                row.col(|ui| {
                    let efficiency_color = constants::efficiency_color(ui, entry.efficiency);
                    ui.label(RichText::new(format!("{:.1}%", entry.efficiency))
                        .color(efficiency_color));
                });
                
                row.col(|ui| {
                    ui.label(format!("{}", entry.thread_count));
                });
            });
    }
}
