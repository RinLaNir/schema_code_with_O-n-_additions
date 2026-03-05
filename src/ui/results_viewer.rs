use eframe::egui::{self, RichText, Ui};
use crate::benchmark::{BenchmarkSummary, import_from_json};
use crate::ui::localization::Localization; 

use crate::ui::results::{ResultsTab, SummaryTab, DetailsTab, PhasesTab, VisualizationTab, AccelerationTab};

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
            acceleration_tab: AccelerationTab::new(localization),
            
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
        self.summary_tab.update_with_summary(summary);
        self.details_tab.update_with_summary(summary);
        self.phases_tab.update_with_summary(summary);
        self.visualization_tab.update_with_summary(summary);
        self.acceleration_tab.update_with_summary(summary);
        self.has_results = true;
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
        
        let tabs = [
            (ResultsTab::Summary,       "tab_summary"),
            (ResultsTab::Visualization, "tab_visualization"),
            (ResultsTab::Acceleration,  "tab_acceleration"),
            (ResultsTab::Details,       "tab_details"),
            (ResultsTab::Phases,        "tab_phases"),
        ];

        ui.horizontal(|ui| {
            for (tab, key) in &tabs {
                if ui.selectable_label(self.selected_tab == *tab, self.localization.get(key)).clicked() {
                    self.selected_tab = tab.clone();
                }
            }
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