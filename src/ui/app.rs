use eframe::egui::{self, Context, RichText};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::benchmark::{run_comprehensive_benchmark_for_ui, BenchmarkSummary};
use crate::log_info;
use crate::types::F2PowElement;
use crate::ui::benchmark_config::BenchmarkConfig;
use crate::ui::components::{BenchmarkState, Header, StatusBar};
use crate::ui::constants::{MAX_CONTENT_WIDTH, SIDEBAR_BREAKPOINT, SIDEBAR_WIDTH};
use crate::ui::localization::{Language, Localization};
use crate::ui::tabs::{AboutTab, ConfigureAction, ConfigureTab, ConsoleTab, ResultsTab, Tab};

pub struct BenchmarkApp {
    tab: Tab,
    localization: Localization,

    configure_tab: ConfigureTab,
    results_tab: ResultsTab,
    console_tab: ConsoleTab,
    about_tab: AboutTab,

    header: Header,
    status_bar: StatusBar,

    state: BenchmarkState,
    benchmark_thread: Option<std::thread::JoinHandle<()>>,
    cancel_flag: Arc<AtomicBool>,

    /// Shared status message updated by worker thread, polled by UI.
    benchmark_status: Arc<Mutex<Option<String>>>,
    /// Shared result set by worker thread on completion.
    benchmark_result: Arc<Mutex<Option<BenchmarkSummary>>>,
    /// Signals that the worker thread has finished.
    benchmark_finished: Arc<AtomicBool>,
}

impl BenchmarkApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = BenchmarkConfig::default();

        log_info!("Schema Code Benchmarking UI initialized");

        let localization = Localization::default();

        Self {
            tab: Tab::Configure,
            localization: localization.clone(),

            configure_tab: ConfigureTab::new(localization.clone(), config.clone()),
            results_tab: ResultsTab::new(localization.clone()),
            console_tab: ConsoleTab::new(localization.clone()),
            about_tab: AboutTab::new(localization.clone()),

            header: Header::new(localization.clone()),
            status_bar: StatusBar::new(localization.clone()),

            state: BenchmarkState::Idle,
            benchmark_thread: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            benchmark_status: Arc::new(Mutex::new(None)),
            benchmark_result: Arc::new(Mutex::new(None)),
            benchmark_finished: Arc::new(AtomicBool::new(false)),
        }
    }

    fn run_benchmark(&mut self) {
        let config = self.configure_tab.get_config();
        let secret = match build_secret(&config) {
            Ok(secret) => secret,
            Err(err) => {
                self.status_bar.set_message(Some(err));
                return;
            }
        };

        self.cancel_flag.store(false, Ordering::SeqCst);
        self.benchmark_finished.store(false, Ordering::SeqCst);
        *self
            .benchmark_status
            .lock()
            .expect("Failed to lock status mutex") = None;
        *self
            .benchmark_result
            .lock()
            .expect("Failed to lock result mutex") = None;

        crate::ui::logging::set_verbose(config.verbose);

        self.state = BenchmarkState::Running;
        self.status_bar.set_state(BenchmarkState::Running);
        self.status_bar
            .set_message(Some(self.localization.get("status_running").to_string()));

        self.tab = Tab::Console;

        log_info!(
            "Starting benchmark: ell={}, impl={:?}, decoder={:?}, rate={:?}, size={:?}, runs={}",
            config.secret_bits,
            config.implementations,
            config.decoder_types,
            config.ldpc_rates,
            config.ldpc_info_sizes,
            config.runs_per_config
        );

        let status = self.benchmark_status.clone();
        let result = self.benchmark_result.clone();
        let finished = self.benchmark_finished.clone();
        let cancel_flag = self.cancel_flag.clone();
        let preparing_msg = self.localization.get("status_preparing").to_string();
        let completed_msg = self.localization.get("status_completed").to_string();

        let handle = thread::spawn(move || {
            *status.lock().expect("Failed to lock status mutex") = Some(preparing_msg);

            let summary = run_comprehensive_benchmark_for_ui(
                &config.shares_to_remove,
                &config.decoder_types,
                &config.ldpc_rates,
                &config.ldpc_info_sizes,
                &config.implementations,
                config.runs_per_config,
                config.cache_setup,
                config.show_detail,
                config
                    .save_results
                    .then_some(config.output_filename.as_str()),
                |status_message| {
                    *status.lock().expect("Failed to lock status mutex") = Some(status_message);
                },
                &secret,
                config.max_iterations,
                config.llr_value,
                cancel_flag,
                config.removal_seed,
            );

            *result.lock().expect("Failed to lock result mutex") = Some(summary);
            *status.lock().expect("Failed to lock status mutex") = Some(completed_msg);
            finished.store(true, Ordering::SeqCst);
        });

        self.benchmark_thread = Some(handle);
    }
}

fn build_secret(config: &BenchmarkConfig) -> Result<F2PowElement, String> {
    if config.secret_random {
        if let Some(seed) = config.secret_seed {
            let mut rng = StdRng::seed_from_u64(seed);
            Ok(F2PowElement::random(config.secret_bits, &mut rng))
        } else {
            let mut rng = rand::rng();
            Ok(F2PowElement::random(config.secret_bits, &mut rng))
        }
    } else {
        F2PowElement::from_hex(&config.secret_hex, config.secret_bits)
            .map_err(|err| format!("Invalid secret: {}", err))
    }
}

impl eframe::App for BenchmarkApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_benchmark_progress();

        let screen_width = ctx.viewport_rect().width();
        let use_sidebar = screen_width >= SIDEBAR_BREAKPOINT;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if use_sidebar {
                if let Some(language) = self.header.show_minimal(ui) {
                    self.update_language(language);
                }
            } else if let Some(language) = self.header.show(ui, &mut self.tab) {
                self.update_language(language);
            }
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.status_bar.show(ui);
        });

        if use_sidebar {
            egui::SidePanel::left("nav_sidebar")
                .resizable(false)
                .exact_width(SIDEBAR_WIDTH)
                .show(ctx, |ui| {
                    ui.add_space(15.0);
                    ui.vertical(|ui| {
                        self.show_nav_item(ui, "⚙", "tab_config", Tab::Configure);
                        self.show_nav_item(ui, "📊", "tab_results", Tab::Results);
                        self.show_nav_item(ui, "🖥", "tab_console", Tab::Console);
                        self.show_nav_item(ui, "ℹ", "tab_about", Tab::About);
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let available_width = ui.available_width();
            let content_width = available_width.min(MAX_CONTENT_WIDTH);
            let horizontal_padding = ((available_width - content_width) / 2.0).max(0.0);

            let available_rect = ui.available_rect_before_wrap();
            let content_rect = egui::Rect::from_min_size(
                egui::pos2(
                    available_rect.left() + horizontal_padding,
                    available_rect.top(),
                ),
                egui::vec2(content_width, available_rect.height()),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                self.show_tab_content(ui);
            });
        });

        if matches!(self.state, BenchmarkState::Running) {
            ctx.request_repaint();
        }
    }
}

impl BenchmarkApp {
    fn poll_benchmark_progress(&mut self) {
        if !matches!(self.state, BenchmarkState::Running) {
            return;
        }

        if let Ok(status) = self.benchmark_status.lock() {
            if let Some(msg) = status.as_ref() {
                self.status_bar.set_message(Some(msg.clone()));
            }
        }

        if self.benchmark_finished.load(Ordering::SeqCst) {
            if let Some(handle) = self.benchmark_thread.take() {
                let _ = handle.join();
            }

            self.state = BenchmarkState::Finished;
            self.status_bar.set_state(BenchmarkState::Finished);

            if let Ok(result) = self.benchmark_result.lock() {
                if let Some(summary) = result.as_ref() {
                    self.results_tab.update_with_summary(summary);
                    self.tab = Tab::Results;
                }
            }
        }
    }

    fn update_language(&mut self, language: Language) {
        self.localization.set_language(language);

        self.header.update(&self.localization);
        let message_clone = self.status_bar.get_message().map(str::to_owned);
        self.status_bar
            .update(self.state.clone(), message_clone, &self.localization);
        self.configure_tab.update_localization(&self.localization);
        self.results_tab.update_localization(&self.localization);
        self.console_tab.update_localization(&self.localization);
        self.about_tab.update_localization(&self.localization);
    }

    fn show_nav_item(&mut self, ui: &mut egui::Ui, icon: &str, key: &str, tab: Tab) {
        let is_selected = self.tab == tab;
        let text = format!("{} {}", icon, self.localization.get(key));

        let response = ui.selectable_label(is_selected, RichText::new(text).size(15.0));

        if response.clicked() {
            self.tab = tab;
        }

        ui.add_space(5.0);
    }

    fn show_tab_content(&mut self, ui: &mut egui::Ui) {
        match self.tab {
            Tab::Configure => {
                let is_running = matches!(self.state, BenchmarkState::Running);
                if let Some(action) = self.configure_tab.show_with_state(ui, is_running) {
                    match action {
                        ConfigureAction::RunBenchmark => {
                            self.run_benchmark();
                        }
                        ConfigureAction::StopBenchmark => {
                            self.stop_benchmark();
                        }
                        ConfigureAction::ShowCommandLine(cmd) => {
                            self.status_bar.set_command_line(Some(cmd));
                            self.status_bar.toggle_command_line();
                        }
                    }
                }
            }
            Tab::Results => self.results_tab.show(ui),
            Tab::Console => self.console_tab.show(ui),
            Tab::About => self.about_tab.show(ui),
        }
    }

    fn stop_benchmark(&mut self) {
        log_info!("Stopping benchmark...");
        self.cancel_flag.store(true, Ordering::SeqCst);
        self.status_bar.set_message(Some(
            self.localization.get("stopping_benchmark").to_string(),
        ));
    }
}
