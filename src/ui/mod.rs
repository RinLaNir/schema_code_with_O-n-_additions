mod app;
mod benchmark_config;
mod results_viewer;
mod localization;
pub mod logging;
mod log_viewer;
pub mod constants;

pub mod components;
pub mod tabs;
pub mod results;

pub use app::BenchmarkApp;
pub use logging::init_logger;

use eframe::egui;

pub fn launch_ui() -> Result<(), eframe::Error> {
    init_logger(5000);
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Schema Code Benchmark",
        options,
        Box::new(|cc| Box::new(BenchmarkApp::new(cc)))
    )
}