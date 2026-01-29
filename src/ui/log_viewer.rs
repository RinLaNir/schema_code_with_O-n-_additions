use eframe::egui::{self, Color32, RichText, ScrollArea, TextStyle};
use crate::ui::logging::{Logger, LogLevel, LogMessage};
use std::sync::Arc;

pub struct LogViewer {
    logger: Arc<Logger>,
    filter_text: String,
    info_enabled: bool,
    warning_enabled: bool,
    error_enabled: bool,
    success_enabled: bool,
    progress_enabled: bool,
    autoscroll: bool,
}

impl LogViewer {
    pub fn new(logger: Arc<Logger>) -> Self {
        Self {
            logger,
            filter_text: String::new(),
            info_enabled: true,
            warning_enabled: true,
            error_enabled: true,
            success_enabled: true,
            progress_enabled: true,
            autoscroll: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.filter_text);

            ui.separator();

            ui.checkbox(&mut self.info_enabled, "Info");
            ui.checkbox(&mut self.warning_enabled, "Warning");
            ui.checkbox(&mut self.error_enabled, "Error");
            ui.checkbox(&mut self.success_enabled, "Success");
            ui.checkbox(&mut self.progress_enabled, "Progress");
            
            ui.separator();
            
            ui.checkbox(&mut self.autoscroll, "Auto-scroll");
            
            if ui.button("Clear").clicked() {
                self.logger.clear();
            }
        });

        ui.separator();

        self.log_area(ui);
    }

    fn log_area(&mut self, ui: &mut egui::Ui) {
        let text_style = TextStyle::Body;
        let row_height = ui.text_style_height(&text_style) + 4.0;
        
        let messages = self.logger.get_messages();
        
        let filtered_messages: Vec<&LogMessage> = messages
            .iter()
            .filter(|msg| {
                let level_match = match msg.level {
                    LogLevel::Info => self.info_enabled,
                    LogLevel::Warning => self.warning_enabled,
                    LogLevel::Error => self.error_enabled,
                    LogLevel::Success => self.success_enabled,
                    LogLevel::Progress => self.progress_enabled,
                };
                
                let text_match = if self.filter_text.is_empty() {
                    true
                } else {
                    msg.message.to_lowercase().contains(&self.filter_text.to_lowercase())
                };
                
                level_match && text_match
            })
            .collect();
        
        self.show_messages_filtered(ui, &filtered_messages, row_height);
    }

    fn show_messages_filtered(&mut self, ui: &mut egui::Ui, messages: &[&LogMessage], height: f32) {
        let scroll_to_bottom = self.autoscroll && !messages.is_empty();
        
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(scroll_to_bottom)
            .show_rows(
                ui,
                height,
                messages.len(),
                |ui, row_range| {
                    for row_idx in row_range {
                        if let Some(msg) = messages.get(row_idx) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("[{}]", msg.formatted_timestamp()))
                                    .color(Color32::GRAY));
                                
                                let (level_tag, level_color) = match msg.level {
                                    LogLevel::Info => ("[INFO]", Color32::LIGHT_BLUE),
                                    LogLevel::Warning => ("[WARN]", Color32::GOLD),
                                    LogLevel::Error => ("[ERROR]", Color32::RED),
                                    LogLevel::Success => ("[SUCCESS]", Color32::GREEN),
                                    LogLevel::Progress => ("[PROGRESS]", Color32::LIGHT_GREEN),
                                };
                                
                                ui.label(RichText::new(level_tag).color(level_color));
                                
                                let message_color = match msg.level {
                                    LogLevel::Error => Color32::LIGHT_RED,
                                    LogLevel::Warning => Color32::LIGHT_YELLOW,
                                    LogLevel::Success => Color32::LIGHT_GREEN,
                                    _ => Color32::WHITE,
                                };
                                
                                ui.label(RichText::new(&msg.message).color(message_color));
                            });
                        }
                    }
                },
            );
        
        if scroll_to_bottom {
            ui.ctx().request_repaint();
        }
    }

    #[allow(dead_code)]
    fn show_messages(&mut self, ui: &mut egui::Ui, messages: &[LogMessage], height: f32) {
        let scroll_to_bottom = self.autoscroll && !messages.is_empty();
        
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(scroll_to_bottom)
            .show_rows(
                ui,
                height,
                messages.len(),
                |ui, row_range| {
                    for row_idx in row_range {
                        if let Some(msg) = messages.get(row_idx) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("[{}]", msg.formatted_timestamp()))
                                    .color(Color32::GRAY));
                                
                                let (level_tag, level_color) = match msg.level {
                                    LogLevel::Info => ("[INFO]", Color32::LIGHT_BLUE),
                                    LogLevel::Warning => ("[WARN]", Color32::GOLD),
                                    LogLevel::Error => ("[ERROR]", Color32::RED),
                                    LogLevel::Success => ("[SUCCESS]", Color32::GREEN),
                                    LogLevel::Progress => ("[PROGRESS]", Color32::LIGHT_GREEN),
                                };
                                
                                ui.label(RichText::new(level_tag).color(level_color));
                                
                                let message_color = match msg.level {
                                    LogLevel::Error => Color32::LIGHT_RED,
                                    LogLevel::Warning => Color32::LIGHT_YELLOW,
                                    LogLevel::Success => Color32::LIGHT_GREEN,
                                    _ => Color32::WHITE,
                                };
                                
                                ui.label(RichText::new(&msg.message).color(message_color));
                            });
                        }
                    }
                },
            );
        
        if scroll_to_bottom {
            ui.ctx().request_repaint();
        }
    }
}