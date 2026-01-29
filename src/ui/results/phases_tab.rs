use eframe::egui::{self, RichText, ScrollArea, Ui};
use crate::benchmark::{BenchmarkSummary, Implementation};
use crate::ui::localization::Localization;
use crate::ui::constants::{self, heading_size, small_size};
use super::utils::{format_duration, show_phase_pie_chart};
use super::table_builder::{ResultsTable, phase_breakdown_columns};
use ldpc_toolbox::codes::ccsds::{AR4JARate, AR4JAInfoSize};
use ldpc_toolbox::decoder::factory::DecoderImplementation;

#[derive(Clone)]
pub struct PhasesTab {
    summary: Option<BenchmarkSummary>,
    localization: Localization,
    all_expanded: bool,
}

impl PhasesTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            summary: None,
            localization,
            all_expanded: false,
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        self.summary = Some(summary.clone());
    }
    
    fn format_info_size(&self, size: &AR4JAInfoSize) -> String {
        match format!("{:?}", size).as_str() {
            "K1024" => self.localization.get("info_size_k1024").to_string(),
            "K4096" => self.localization.get("info_size_k4096").to_string(),
            "K16384" => self.localization.get("info_size_k16384").to_string(),
            other => other.to_string(),
        }
    }
    
    fn format_rate(&self, rate: &AR4JARate) -> String {
        match format!("{:?}", rate).as_str() {
            "R1_2" => self.localization.get("rate_r1_2").to_string(),
            "R2_3" => self.localization.get("rate_r2_3").to_string(),
            "R4_5" => self.localization.get("rate_r4_5").to_string(),
            other => other.to_string(),
        }
    }
    
    fn format_decoder(&self, decoder: &DecoderImplementation) -> String {
        match format!("{:?}", decoder).as_str() {
            "Aminstarf32" => self.localization.get("decoder_aminstarf32").to_string(),
            "Aminstarf64" => self.localization.get("decoder_aminstarf64").to_string(),
            "Phif32" => self.localization.get("decoder_phif32").to_string(),
            "Phif64" => self.localization.get("decoder_phif64").to_string(),
            "Tanhf32" => self.localization.get("decoder_tanhf32").to_string(),
            "Tanhf64" => self.localization.get("decoder_tanhf64").to_string(),
            other => other.to_string(),
        }
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        let col_phase = self.localization.get("col_phase").to_string();
        let col_avg_time = self.localization.get("col_avg_time").to_string();
        let col_min_time = self.localization.get("col_min_time").to_string();
        let col_max_time = self.localization.get("col_max_time").to_string();
        let col_percent_total = self.localization.get("col_percent_total").to_string();
        let phase_distribution = self.localization.get("phase_distribution").to_string();
        let expand_all_text = self.localization.get("expand_all").to_string();
        let collapse_all_text = self.localization.get("collapse_all").to_string();
        
        ScrollArea::vertical().show(ui, |ui| {
            if let Some(summary) = self.summary.clone() {
                ui.horizontal(|ui| {
                    if ui.button(&expand_all_text).clicked() {
                        self.all_expanded = true;
                    }
                    if ui.button(&collapse_all_text).clicked() {
                        self.all_expanded = false;
                    }
                });
                ui.add_space(constants::ITEM_SPACING);
                
                if !summary.deal_stats.is_empty() {
                    ui.heading(RichText::new(self.localization.get("deal_phases_title")).size(heading_size(ui)));
                    ui.add_space(constants::SMALL_SPACING);
                    
                    let mut deal_entries: Vec<_> = summary.deal_stats.iter().collect();
                    deal_entries.sort_by(|a, b| {
                        let decoder_cmp = format!("{:?}", a.0.decoder_type).cmp(&format!("{:?}", b.0.decoder_type));
                        if decoder_cmp != std::cmp::Ordering::Equal { return decoder_cmp; }
                        let rate_cmp = format!("{:?}", a.0.ldpc_rate).cmp(&format!("{:?}", b.0.ldpc_rate));
                        if rate_cmp != std::cmp::Ordering::Equal { return rate_cmp; }
                        match (a.0.implementation, b.0.implementation) {
                            (Implementation::Sequential, Implementation::Parallel) => std::cmp::Ordering::Less,
                            (Implementation::Parallel, Implementation::Sequential) => std::cmp::Ordering::Greater,
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    
                    for (params, stats) in deal_entries {
                        if let Some(phase_metrics) = &stats.phase_metrics {
                            let section_id = format!("deal_{:?}_{:?}_{:?}_{:?}_{:?}", 
                                params.implementation,
                                params.c_value,
                                params.ldpc_info_size,
                                params.ldpc_rate,
                                params.decoder_type);
                            

                            let header = format!("{} • C{} • {} • {} • {}", 
                                params.implementation,
                                params.c_value,
                                self.format_info_size(&params.ldpc_info_size),
                                self.format_rate(&params.ldpc_rate),
                                self.format_decoder(&params.decoder_type));
                            
                            ui.push_id(format!("deal_section_{}", &section_id), |ui| {
                                let header_state = egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    ui.make_persistent_id(format!("deal_collapse_{}", &section_id)),
                                    self.all_expanded,
                                );
                                
                                header_state.show_header(ui, |ui| {
                                    ui.label(RichText::new(&header).size(constants::scaled_size(ui, constants::SUBHEADING_SCALE)));
                                })
                                .body(|ui| {
                                    self.show_phase_details(
                                        ui,
                                        phase_metrics,
                                        &section_id,
                                        &col_phase,
                                        &col_avg_time,
                                        &col_min_time,
                                        &col_max_time,
                                        &col_percent_total,
                                        &phase_distribution,
                                    );
                                });
                            });
                        }
                    }
                }
                
                ui.add_space(constants::SECTION_SPACING);
                
                if !summary.reconstruct_stats.is_empty() {
                    ui.heading(RichText::new(self.localization.get("reconstruct_phases_title")).size(heading_size(ui)));
                    ui.add_space(constants::SMALL_SPACING);
                    
                    let mut reconstruct_entries: Vec<_> = summary.reconstruct_stats.iter().collect();
                    reconstruct_entries.sort_by(|a, b| {
                        let decoder_cmp = format!("{:?}", a.0.decoder_type).cmp(&format!("{:?}", b.0.decoder_type));
                        if decoder_cmp != std::cmp::Ordering::Equal { return decoder_cmp; }
                        let rate_cmp = format!("{:?}", a.0.ldpc_rate).cmp(&format!("{:?}", b.0.ldpc_rate));
                        if rate_cmp != std::cmp::Ordering::Equal { return rate_cmp; }
                        match (a.0.implementation, b.0.implementation) {
                            (Implementation::Sequential, Implementation::Parallel) => std::cmp::Ordering::Less,
                            (Implementation::Parallel, Implementation::Sequential) => std::cmp::Ordering::Greater,
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    
                    for (params, stats) in reconstruct_entries {
                        if let Some(phase_metrics) = &stats.phase_metrics {
                            let section_id = format!("reconstruct_{:?}_{:?}_{:?}_{:?}_{:?}", 
                                params.implementation,
                                params.c_value,
                                params.ldpc_info_size,
                                params.ldpc_rate,
                                params.decoder_type);
                            
                            let header = format!("{} • C{} • {} • {} • {}", 
                                params.implementation,
                                params.c_value,
                                self.format_info_size(&params.ldpc_info_size),
                                self.format_rate(&params.ldpc_rate),
                                self.format_decoder(&params.decoder_type));
                            
                            ui.push_id(format!("reconstruct_section_{}", &section_id), |ui| {
                                let header_state = egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    ui.make_persistent_id(format!("reconstruct_collapse_{}", &section_id)),
                                    self.all_expanded,
                                );
                                
                                header_state.show_header(ui, |ui| {
                                    ui.label(RichText::new(&header).size(constants::scaled_size(ui, constants::SUBHEADING_SCALE)));
                                })
                                .body(|ui| {
                                    self.show_phase_details(
                                        ui,
                                        phase_metrics,
                                        &section_id,
                                        &col_phase,
                                        &col_avg_time,
                                        &col_min_time,
                                        &col_max_time,
                                        &col_percent_total,
                                        &phase_distribution,
                                    );
                                    
                                    if let Some(decoding_stats) = &stats.decoding_stats {
                                        ui.add_space(constants::ITEM_SPACING);
                                        ui.separator();
                                        ui.add_space(constants::SMALL_SPACING);
                                        ui.label(RichText::new(self.localization.get("decoding_stats_title")).strong().size(small_size(ui) * 1.1));
                                        ui.add_space(constants::SMALL_SPACING);
                                        
                                        egui::Grid::new(format!("decoding_stats_{}", section_id))
                                            .spacing([10.0, 4.0])
                                            .show(ui, |ui| {
                                                ui.label(self.localization.get("total_rows"));
                                                ui.label(format!("{}", decoding_stats.total_rows));
                                                ui.end_row();
                                                
                                                ui.label(self.localization.get("successful_rows"));
                                                let success_color = constants::rate_color(ui, decoding_stats.success_rate());
                                                ui.label(RichText::new(format!("{} ({:.1}%)", 
                                                    decoding_stats.successful_rows, 
                                                    decoding_stats.success_rate() * 100.0))
                                                    .color(success_color));
                                                ui.end_row();
                                                
                                                if decoding_stats.failed_rows > 0 {
                                                    ui.label(self.localization.get("failed_rows"));
                                                    ui.label(RichText::new(format!("{}", decoding_stats.failed_rows))
                                                        .color(constants::error_color(ui)));
                                                    ui.end_row();
                                                }
                                                
                                                ui.label(self.localization.get("avg_iterations"));
                                                ui.label(format!("{:.2}", decoding_stats.avg_iterations));
                                                ui.end_row();
                                                
                                                if decoding_stats.max_iterations_hit > 0 {
                                                    ui.label(self.localization.get("max_iter_hit"));
                                                    let hit_rate = decoding_stats.max_iterations_hit as f64 / decoding_stats.total_rows as f64 * 100.0;
                                                    ui.label(RichText::new(format!("{} ({:.1}%)", 
                                                        decoding_stats.max_iterations_hit, hit_rate))
                                                        .color(constants::warning_color(ui)));
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                });
                            });
                        }
                    }
                }
            }
        });
    }
    
    fn show_phase_details(
        &self,
        ui: &mut Ui,
        phase_metrics: &std::collections::HashMap<String, crate::benchmark::PhaseStats>,
        section_id: &str,
        col_phase: &str,
        col_avg_time: &str,
        col_min_time: &str,
        col_max_time: &str,
        col_percent_total: &str,
        phase_distribution: &str,
    ) {
        let mut phases: Vec<_> = phase_metrics.iter().collect();
        phases.sort_by(|(_, a), (_, b)| 
            b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());
        
        let phases_for_table: Vec<_> = phases.iter()
            .map(|(name, stat)| (name.to_string(), (*stat).clone()))
            .collect();
        
        let columns = phase_breakdown_columns(
            col_phase,
            col_avg_time,
            col_min_time,
            col_max_time,
            col_percent_total,
        );
        
        ResultsTable::new(&format!("{}_phases_table", section_id), columns)
            .show(ui, phases_for_table.len(), |row_idx, row| {
                let (name, phase_stat) = &phases_for_table[row_idx];
                
                row.col(|ui| { ui.label(name); });
                row.col(|ui| { ui.label(format_duration(phase_stat.avg_duration)); });
                row.col(|ui| { ui.label(format_duration(phase_stat.min_duration)); });
                row.col(|ui| { ui.label(format_duration(phase_stat.max_duration)); });
                row.col(|ui| { ui.label(format!("{:.2}%", phase_stat.avg_percentage)); });
            });
        
        show_phase_pie_chart(ui, phase_metrics, phase_distribution);
    }
}
