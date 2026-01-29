use eframe::egui::Ui;
use crate::ui::localization::Localization;

pub struct AboutTab {
    localization: Localization,
}

impl AboutTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            localization,
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn show(&self, ui: &mut Ui) {
        ui.heading(self.localization.get("about_title"));
        ui.add_space(10.0);
        
        ui.label(self.localization.get("about_description"));
        
        ui.add_space(10.0);
        
        ui.heading(self.localization.get("user_instructions"));
        ui.label(self.localization.get("instruction_1"));
        ui.label(self.localization.get("instruction_2"));
        ui.label(self.localization.get("instruction_3"));
        ui.label(self.localization.get("instruction_4"));
        
        ui.add_space(10.0);
        
        ui.heading(self.localization.get("benchmark_params_heading"));
        ui.label(self.localization.get("param_c_desc"));
        ui.label(self.localization.get("param_runs_desc"));
        ui.label(self.localization.get("param_impl_desc"));
        ui.label(self.localization.get("param_decoder_desc"));
        ui.label(self.localization.get("param_rate_desc"));
        ui.label(self.localization.get("param_size_desc"));
        
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.label("© 2025");
            ui.hyperlink_to("Schema Code Project", "https://github.com/schema-code/schema-code");
        });
    }
}
