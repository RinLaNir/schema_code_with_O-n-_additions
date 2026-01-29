use eframe::egui::Ui;
use crate::ui::localization::Localization;
use crate::ui::results_viewer::ResultsViewer;
use crate::benchmark::BenchmarkSummary;

pub struct ResultsTab {
    localization: Localization,
    results_viewer: ResultsViewer,
}

impl ResultsTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            localization: localization.clone(),
            results_viewer: ResultsViewer::new(localization),
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
        self.results_viewer.update_localization(localization);
    }
    
    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        self.results_viewer.update_with_summary(summary);
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        self.results_viewer.ui(ui);
    }
}