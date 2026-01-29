use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use ark_bls12_381::Fr;
use eframe::egui::{self, Context, RichText};

use crate::benchmark::{run_comprehensive_benchmark_for_ui, BenchmarkSummary};
use crate::log_info;
use crate::ui::benchmark_config::BenchmarkConfig;
use crate::ui::tabs::{Tab, ConfigureTab, ConfigureAction, ResultsTab, ConsoleTab, AboutTab};
use crate::ui::components::{Header, StatusBar, BenchmarkState};
use crate::ui::localization::Localization;
use crate::ui::logging::get_logger;

const SIDEBAR_BREAKPOINT: f32 = 900.0;
const SIDEBAR_WIDTH: f32 = 180.0;
const MAX_CONTENT_WIDTH: f32 = 1200.0;

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
    benchmark_thread: Option<std::thread::JoinHandle<(BenchmarkState, Option<String>, Arc<Mutex<Option<BenchmarkSummary>>>)>>,
    cancel_flag: Arc<AtomicBool>,
}

impl BenchmarkApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = BenchmarkConfig::default();
        
        let _logger = get_logger();
        
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
        }
    }
    
    fn run_benchmark(&mut self) {
        let config = self.configure_tab.get_config();
        
        self.cancel_flag.store(false, Ordering::SeqCst);
        
        crate::ui::logging::set_verbose(config.verbose);
        
        self.state = BenchmarkState::Running;
        self.status_bar.set_state(BenchmarkState::Running);
        self.status_bar.set_message(Some(self.localization.get("status_running").to_string()));
        
        self.tab = Tab::Console;
        
        log_info!("Starting benchmark: C={:?}, impl={:?}, decoder={:?}, rate={:?}, size={:?}, runs={}", 
            config.c_values, config.implementations, config.decoder_types,
            config.ldpc_rates, config.ldpc_info_sizes, config.runs_per_config);
        
        let (tx, rx) = std::sync::mpsc::channel();
        
        let result_data = Arc::new(Mutex::new(None));
        let result_data_clone = result_data.clone();
        
        let cancel_flag = self.cancel_flag.clone();
        
        thread::spawn(move || {
            let _ = tx.send(("status", "Підготовка середовища для бенчмаркінгу...".to_string()));
            
            let summary = run_comprehensive_benchmark_for_ui::<Fr>(
                &config.c_values,
                &config.shares_to_remove,
                &config.decoder_types,
                &config.ldpc_rates,
                &config.ldpc_info_sizes,
                &config.implementations,
                config.runs_per_config,
                config.show_detail,
                if config.save_results {
                    if config.output_filename.is_empty() {
                        Some("")
                    } else {
                        Some(&config.output_filename)
                    }
                } else {
                    None
                },
                |status_message| {
                    let _ = tx.send(("progress", status_message));
                },
                config.secret_value,
                config.max_iterations,
                config.llr_value,
                cancel_flag,
            );
            
            *result_data_clone.lock().expect("Failed to lock result data mutex") = Some(summary);
            
            let _ = tx.send(("status", "Benchmarking completed successfully!".to_string()));
            let _ = tx.send(("complete", "".to_string()));
        });
        
        let state = Arc::new(Mutex::new(self.state.clone())); 
        let status = Arc::new(Mutex::new(None::<String>));
        let results_data_for_ui = result_data.clone();
        
        let handle = std::thread::spawn(move || {
            while let Ok((msg_type, content)) = rx.recv() {
                match msg_type {
                    "status" => {
                        *status.lock().expect("Failed to lock status mutex") = Some(content);
                    },
                    "progress" => {
                        *status.lock().expect("Failed to lock status mutex") = Some(content);
                    },
                    "complete" => {
                        *state.lock().expect("Failed to lock state mutex") = BenchmarkState::Finished;
                        break;
                    },
                    _ => {}
                }
            }
            
            (
                (*state.lock().expect("Failed to lock state mutex")).clone(),
                status.lock().expect("Failed to lock status mutex").clone(),
                results_data_for_ui
            )
        });
        
        self.benchmark_thread = Some(handle);
    }
}

impl eframe::App for BenchmarkApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(handle) = &self.benchmark_thread {
            if handle.is_finished() {
                if let Some(handle) = self.benchmark_thread.take() {
                    if let Ok((state, status_message, result_data)) = handle.join() {
                        self.state = state.clone();
                        self.status_bar.set_state(state);
                        
                        if let Some(msg) = status_message {
                            self.status_bar.set_message(Some(msg));
                        }
                        
                        if let Ok(data) = result_data.lock() {
                            if let Some(summary) = &*data {
                                self.results_tab.update_with_summary(summary);
                                
                                self.tab = Tab::Results;
                            }
                        }
                    }
                }
            }
        }

        let screen_width = ctx.screen_rect().width();
        let use_sidebar = screen_width >= SIDEBAR_BREAKPOINT;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if use_sidebar {
                if let Some(language) = self.header.show_minimal(ui) {
                    self.update_language(language);
                }
            } else {
                if let Some(language) = self.header.show(ui, &mut self.tab) {
                    self.update_language(language);
                }
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
                egui::pos2(available_rect.left() + horizontal_padding, available_rect.top()),
                egui::vec2(content_width, available_rect.height())
            );
            
            ui.allocate_ui_at_rect(content_rect, |ui| {
                self.show_tab_content(ui);
            });
        });
        
        if let BenchmarkState::Running = self.state {
            ctx.request_repaint();
        }
    }
}

impl BenchmarkApp {
    fn update_language(&mut self, language: crate::ui::localization::Language) {
        self.localization.set_language(language);
        
        self.header.update(&self.localization);
        let message_clone = self.status_bar.get_message().as_ref().cloned();
        self.status_bar.update(self.state.clone(), message_clone, &self.localization);
        self.configure_tab.update_localization(&self.localization);
        self.results_tab.update_localization(&self.localization);
        self.console_tab.update_localization(&self.localization);
        self.about_tab.update_localization(&self.localization);
    }
    
    fn show_nav_item(&mut self, ui: &mut egui::Ui, icon: &str, key: &str, tab: Tab) {
        let is_selected = self.tab == tab;
        let text = format!("{} {}", icon, self.localization.get(key));
        
        let response = ui.selectable_label(
            is_selected, 
            RichText::new(text).size(15.0)
        );
        
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
                        },
                        ConfigureAction::StopBenchmark => {
                            self.stop_benchmark();
                        },
                        ConfigureAction::ShowCommandLine(cmd) => {
                            self.status_bar.set_command_line(Some(cmd));
                            self.status_bar.toggle_command_line();
                        },
                    }
                }
            },
            Tab::Results => self.results_tab.show(ui),
            Tab::Console => self.console_tab.show(ui),
            Tab::About => self.about_tab.show(ui),
        }
    }
    
    fn stop_benchmark(&mut self) {
        log_info!("Stopping benchmark...");
        self.cancel_flag.store(true, Ordering::SeqCst);
        self.status_bar.set_message(Some(self.localization.get("stopping_benchmark").to_string()));
    }
}