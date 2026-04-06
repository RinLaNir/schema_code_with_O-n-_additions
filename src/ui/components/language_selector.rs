use crate::ui::localization::Language;
use eframe::egui::{self, RichText, Ui};

/// Renders language switcher buttons (EN / UA) and returns the chosen language.
pub struct LanguageSelector;

impl LanguageSelector {
    pub fn show(ui: &mut Ui) -> Option<Language> {
        let mut selected = None;

        egui::Frame::NONE
            .fill(ui.visuals().extreme_bg_color)
            .corner_radius(5.0)
            .inner_margin(4)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("EN").text_style(egui::TextStyle::Body))
                        .clicked()
                    {
                        selected = Some(Language::English);
                    }
                    if ui
                        .button(RichText::new("UA").text_style(egui::TextStyle::Body))
                        .clicked()
                    {
                        selected = Some(Language::Ukrainian);
                    }
                });
            });

        selected
    }
}
