use crate::ui::localization::Localization;
use eframe::egui::{self, Color32, RichText, Ui};

#[derive(PartialEq, Clone)]
pub enum BenchmarkState {
    Idle,
    Running,
    Finished,
    /// Error state - currently unused but kept for future error handling
    #[allow(dead_code)]
    Error(String),
}

pub struct StatusBar {
    state: BenchmarkState,
    status_message: Option<String>,
    localization: Localization,
    command_line: Option<String>,
    showing_command_line: bool,
}

impl StatusBar {
    pub fn new(localization: Localization) -> Self {
        Self {
            state: BenchmarkState::Idle,
            status_message: None,
            localization,
            command_line: None,
            showing_command_line: false,
        }
    }

    pub fn update(
        &mut self,
        state: BenchmarkState,
        message: Option<String>,
        localization: &Localization,
    ) {
        self.state = state;
        self.status_message = message;
        self.localization = localization.clone();
    }

    pub fn set_state(&mut self, state: BenchmarkState) {
        self.state = state;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.status_message = message;
    }

    pub fn get_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn set_command_line(&mut self, command: Option<String>) {
        self.command_line = command;
    }

    pub fn toggle_command_line(&mut self) {
        self.showing_command_line = !self.showing_command_line;
    }

    fn state_color(&self) -> Color32 {
        match &self.state {
            BenchmarkState::Running => Color32::from_rgb(0, 128, 255),
            BenchmarkState::Finished => Color32::from_rgb(0, 180, 0),
            BenchmarkState::Error(_) => Color32::from_rgb(255, 50, 50),
            _ => Color32::GRAY,
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            match (self.showing_command_line, &self.command_line) {
                (true, Some(cmd)) => {
                    ui.label(RichText::new(cmd.as_str()).color(Color32::from_rgb(0, 160, 0)));
                }
                _ => match &self.status_message {
                    Some(message) => {
                        ui.label(RichText::new(message).color(self.state_color()));
                    }
                    None => {
                        ui.label(
                            RichText::new(self.localization.get("status_ready"))
                                .color(Color32::GRAY),
                        );
                    }
                },
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let BenchmarkState::Running = self.state {
                    ui.spinner();
                }
            });
        });
    }
}
