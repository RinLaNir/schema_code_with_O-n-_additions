use eframe::egui::{self, RichText, Ui};
use crate::ui::localization::{Localization, Language};
use crate::ui::tabs::Tab;

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
                if let Some(language) = self.show_language_selector(ui) {
                    selected_language = Some(language);
                }
            });
        });
        
        selected_language
    }
    
    pub fn show(&self, ui: &mut Ui, current_tab: &mut Tab) -> Option<Language> {
        let mut selected_language = None;
        
        ui.horizontal(|ui| {
            ui.heading(self.localization.get("app_title"));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(language) = self.show_language_selector(ui) {
                    selected_language = Some(language);
                }
                
                if ui.button(RichText::new(self.localization.get("tab_about")).text_style(egui::TextStyle::Body))
                    .clicked() {
                    *current_tab = Tab::About;
                }
                
                if ui.button(RichText::new(self.localization.get("tab_console")).text_style(egui::TextStyle::Body))
                    .clicked() {
                    *current_tab = Tab::Console;
                }
                
                if ui.button(RichText::new(self.localization.get("tab_results")).text_style(egui::TextStyle::Body))
                    .clicked() {
                    *current_tab = Tab::Results;
                }
                
                if ui.button(RichText::new(self.localization.get("tab_config")).text_style(egui::TextStyle::Body))
                    .clicked() {
                    *current_tab = Tab::Configure;
                }
            });
        });
        
        selected_language
    }
    
    fn show_language_selector(&self, ui: &mut Ui) -> Option<Language> {
        let mut selected_language = None;
        
        ui.horizontal(|ui| {
            egui::Frame::none()
                .fill(ui.visuals().extreme_bg_color)
                .rounding(5.0)
                .inner_margin(egui::style::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("EN").text_style(egui::TextStyle::Body))
                            .clicked() {
                            selected_language = Some(Language::English);
                        }
                        
                        if ui.button(RichText::new("UA").text_style(egui::TextStyle::Body))
                            .clicked() {
                            selected_language = Some(Language::Ukrainian);
                        }
                    });
                });
        });
        
        selected_language
    }
}
