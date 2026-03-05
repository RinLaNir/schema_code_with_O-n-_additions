use eframe::egui::{self, RichText, Ui};
use crate::ui::localization::{Localization, Language};
use crate::ui::tabs::Tab;
use super::language_selector::LanguageSelector;

pub struct Header {
    localization: Localization,
}

impl Header {
    pub fn new(localization: Localization) -> Self {
        Self {
            localization,
        }
    }
    
    pub fn update(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn show_minimal(&self, ui: &mut Ui) -> Option<Language> {
        let mut selected_language = None;

        ui.horizontal(|ui| {
            ui.heading(self.localization.get("app_title"));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                selected_language = LanguageSelector::show(ui);
            });
        });

        selected_language
    }

    pub fn show(&self, ui: &mut Ui, current_tab: &mut Tab) -> Option<Language> {
        let mut selected_language = None;

        ui.horizontal(|ui| {
            ui.heading(self.localization.get("app_title"));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                selected_language = LanguageSelector::show(ui);

                let mut nav_button = |key: &str, tab: Tab| {
                    let label = RichText::new(self.localization.get(key)).text_style(egui::TextStyle::Body);
                    if ui.button(label).clicked() {
                        *current_tab = tab;
                    }
                };

                nav_button("tab_about", Tab::About);
                nav_button("tab_console", Tab::Console);
                nav_button("tab_results", Tab::Results);
                nav_button("tab_config", Tab::Configure);
            });
        });

        selected_language
    }
}
