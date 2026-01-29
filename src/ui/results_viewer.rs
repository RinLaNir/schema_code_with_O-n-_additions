use eframe::egui::{self, RichText, Ui};
use crate::benchmark::{BenchmarkSummary, BenchmarkParams, Implementation, import_from_json};
use crate::ui::localization::Localization; 

use crate::ui::results::{ResultsTab, SummaryTab, DetailsTab, PhasesTab, VisualizationTab, AccelerationTab};
use std::collections::HashMap;
use std::cmp::Ordering; 

#[derive(Clone)]
pub struct ResultsViewer {
    has_results: bool,
    selected_tab: ResultsTab,
    localization: Localization,

    summary_tab: SummaryTab,
    details_tab: DetailsTab,
    phases_tab: PhasesTab,
    visualization_tab: VisualizationTab,
    acceleration_tab: AccelerationTab,

    import_error: Option<String>,
    import_success: bool,
}

impl ResultsViewer {
    pub fn new(localization: Localization) -> Self {
        Self {
            has_results: false,
            selected_tab: ResultsTab::Summary,
            localization: localization.clone(),
            
            summary_tab: SummaryTab::new(localization.clone()),
            details_tab: DetailsTab::new(localization.clone()),
            phases_tab: PhasesTab::new(localization.clone()),
            visualization_tab: VisualizationTab::new(localization.clone()),
            acceleration_tab: AccelerationTab::new(localization.clone()),
            
            import_error: None,
            import_success: false,
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
        self.summary_tab.update_localization(localization);
        self.details_tab.update_localization(localization);
        self.phases_tab.update_localization(localization);
        self.visualization_tab.update_localization(localization);
        self.acceleration_tab.update_localization(localization);
    }
    
    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        let sorted_summary = self.sort_benchmark_summary(summary);
        
        self.summary_tab.update_with_summary(&sorted_summary);
        self.details_tab.update_with_summary(&sorted_summary);
        self.phases_tab.update_with_summary(&sorted_summary);
        self.visualization_tab.update_with_summary(&sorted_summary);
        self.acceleration_tab.update_with_summary(&sorted_summary);
        self.has_results = true;
    }
    
    fn sort_benchmark_summary(&self, original_summary: &BenchmarkSummary) -> BenchmarkSummary {
        fn sort_stats<T: Clone>(stats: &HashMap<BenchmarkParams, T>) -> HashMap<BenchmarkParams, T> {
            let mut entries: Vec<_> = stats.iter().collect();
            
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
            
            let mut sorted_map = HashMap::new();
            for (k, v) in entries {
                sorted_map.insert(k.clone(), v.clone());
            }
            sorted_map
        }
        
        BenchmarkSummary {
            setup_stats: sort_stats(&original_summary.setup_stats),
            deal_stats: sort_stats(&original_summary.deal_stats),
            reconstruct_stats: sort_stats(&original_summary.reconstruct_stats),
            total_stats: sort_stats(&original_summary.total_stats),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.localization.get("results_title"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.localization.get("import_results")).clicked() {
                    self.import_error = None;
                    self.import_success = false;
                    
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        match import_from_json(&path) {
                            Ok(summary) => {
                                self.update_with_summary(&summary);
                                self.import_success = true;
                            }
                            Err(e) => {
                                self.import_error = Some(e);
                            }
                        }
                    }
                }
            });
        });
        ui.add_space(5.0);
        
        if let Some(ref error) = self.import_error {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", self.localization.get("import_error"), error))
                    .color(egui::Color32::LIGHT_RED));
            });
            ui.add_space(5.0);
        }
        if self.import_success {
            ui.horizontal(|ui| {
                ui.label(RichText::new(self.localization.get("import_success"))
                    .color(egui::Color32::LIGHT_GREEN));
            });
            ui.add_space(5.0);
        }
        
        if !self.has_results {
            ui.label(RichText::new(self.localization.get("no_results"))
                .color(egui::Color32::LIGHT_YELLOW));
            return;
        }
        
        let tab_summary = self.localization.get("tab_summary").to_string();
        let tab_details = self.localization.get("tab_details").to_string();
        let tab_phases = self.localization.get("tab_phases").to_string();
        let tab_visualization = self.localization.get("tab_visualization").to_string();
        let tab_acceleration = self.localization.get("tab_acceleration").to_string();
        
        ui.columns(5, |columns| {
            columns[0].vertical_centered(|ui| {
                if ui.selectable_label(self.selected_tab == ResultsTab::Summary, &tab_summary).clicked() {
                    self.selected_tab = ResultsTab::Summary;
                }
            });
            columns[1].vertical_centered(|ui| {
                if ui.selectable_label(self.selected_tab == ResultsTab::Visualization, &tab_visualization).clicked() {
                    self.selected_tab = ResultsTab::Visualization;
                }
            });
            columns[2].vertical_centered(|ui| {
                if ui.selectable_label(self.selected_tab == ResultsTab::Acceleration, &tab_acceleration).clicked() {
                    self.selected_tab = ResultsTab::Acceleration;
                }
            });
            columns[3].vertical_centered(|ui| {
                if ui.selectable_label(self.selected_tab == ResultsTab::Details, &tab_details).clicked() {
                    self.selected_tab = ResultsTab::Details;
                }
            });
            columns[4].vertical_centered(|ui| {
                if ui.selectable_label(self.selected_tab == ResultsTab::Phases, &tab_phases).clicked() {
                    self.selected_tab = ResultsTab::Phases;
                }
            });
        });
        
        ui.separator();
        
        match self.selected_tab {
            ResultsTab::Summary => self.summary_tab.show(ui),
            ResultsTab::Visualization => self.visualization_tab.show(ui),
            ResultsTab::Acceleration => self.acceleration_tab.show(ui),
            ResultsTab::Details => self.details_tab.show(ui),
            ResultsTab::Phases => self.phases_tab.show(ui),
        }
    }
}