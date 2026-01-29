use eframe::egui::{self, RichText, Ui};
use crate::ui::localization::{Localization, Language};

#[allow(dead_code)]
pub struct LanguageSelector {
    localization: Localization,
}

#[allow(dead_code)]
impl LanguageSelector {
    pub fn new(localization: Localization) -> Self {
        Self {
            localization,
        }
    }
    
    pub fn update(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn show(&self, ui: &mut Ui) -> Option<Language> {
        let mut selected_language = None;
        
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
            
        selected_language
    }
}
