use crate::ui::localization::Localization;
use crate::ui::log_viewer::LogViewer;
use crate::ui::logging::get_logger;
use eframe::egui::Ui;

pub struct ConsoleTab {
    log_viewer: LogViewer,
    localization: Localization,
}

impl ConsoleTab {
    pub fn new(localization: Localization) -> Self {
        let logger = get_logger();

        Self {
            log_viewer: LogViewer::new(logger),
            localization,
        }
    }

    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading(self.localization.get("console_title"));
        ui.add_space(5.0);

        self.log_viewer.ui(ui);
    }
}
