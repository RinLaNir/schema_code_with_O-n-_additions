use eframe::egui::{self, Color32, RichText, ScrollArea, Ui};
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use crate::benchmark::Implementation;
use crate::ui::benchmark_config::BenchmarkConfig;
use crate::ui::components::DecoderSelector;
use crate::ui::localization::Localization;

pub enum ConfigureAction {
    RunBenchmark,
    StopBenchmark,
    ShowCommandLine(String),
}

pub struct ConfigureTab {
    config: BenchmarkConfig,
    localization: Localization,
    decoder_selector: DecoderSelector,
    
    selected_implementation: usize,
    selected_rates: Vec<bool>,
    selected_size: usize,
    
    c_value: String,
    runs_value: String,
    warmup_value: String,
    shares_to_remove_value: String,
    shares_to_remove_as_percentage: bool,
    llr_value: String,
    max_iterations_value: String,
    
    secret_value: String,
    secret_hex_mode: bool,
    secret_random: bool,
    secret_seed: String,
    secret_valid: bool,
    
    command_line_display: Option<String>,
}

impl ConfigureTab {
    pub fn new(localization: Localization, config: BenchmarkConfig) -> Self {
        Self {
            localization: localization.clone(),
            config: config.clone(),
            decoder_selector: DecoderSelector::new(localization),
            
            selected_implementation: 0,
            selected_rates: vec![false, false, true],
            selected_size: 0,
            
            c_value: config.c_values.first().map_or("10".to_string(), |c| c.to_string()),
            runs_value: config.runs_per_config.to_string(),
            warmup_value: config.warmup_runs.to_string(),
            shares_to_remove_value: config.shares_to_remove.first().map_or("100".to_string(), |s| s.abs().to_string()),
            shares_to_remove_as_percentage: config.shares_to_remove.first().map_or(false, |&s| s < 0),
            llr_value: config.llr_value.to_string(),
            max_iterations_value: config.max_iterations.to_string(),
            
            secret_value: config.secret_value.to_string(),
            secret_hex_mode: false,
            secret_random: config.secret_random,
            secret_seed: config.secret_seed.map_or(String::new(), |s| s.to_string()),
            secret_valid: true,
            
            command_line_display: None,
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
        self.decoder_selector.update(localization);
    }
    
    pub fn show_with_state(&mut self, ui: &mut Ui, is_running: bool) -> Option<ConfigureAction> {
        let mut action: Option<ConfigureAction> = None;
        
        ui.heading(self.localization.get("config_title"));
        ui.add_space(10.0);
        
        ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::group(ui.style())
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(150, 150, 180)))
                .rounding(8.0)
                .inner_margin(egui::style::Margin::same(12.0))
                .outer_margin(egui::style::Margin::symmetric(0.0, 4.0))
                .show(ui, |ui| {
                    ui.heading(RichText::new(self.localization.get("basic_params")).color(Color32::from_rgb(80, 150, 230)));
                    ui.add_space(5.0);
                    
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            ui.label(self.localization.get("c_value"));
                            ui.horizontal(|ui| {
                                let c_parsed = self.c_value.parse::<usize>().unwrap_or(10);
                                let mut c_val = c_parsed;
                                ui.add(egui::Slider::new(&mut c_val, 2..=50).text("C"));
                                if c_val != c_parsed {
                                    self.c_value = c_val.to_string();
                                    self.config.c_values = vec![c_val];
                                }
                                ui.add(egui::TextEdit::singleline(&mut self.c_value)
                                    .desired_width(80.0));
                            });
                            
                            ui.label(self.localization.get("particles_to_remove"));
                            ui.horizontal(|ui| {
                                let parsed_val = self.shares_to_remove_value.parse::<isize>().unwrap_or(100);
                                let mut value = parsed_val.abs();
                                
                                ui.add(egui::Slider::new(&mut value, 1..=1000).text(""));
                                if value != parsed_val.abs() {
                                    self.shares_to_remove_value = value.to_string();
                                }
                                ui.add(egui::TextEdit::singleline(&mut self.shares_to_remove_value)
                                    .desired_width(80.0));
                                ui.checkbox(&mut self.shares_to_remove_as_percentage, self.localization.get("as_percentage"));
                            });

                            if let Ok(mut value) = self.shares_to_remove_value.parse::<isize>() {
                                if self.shares_to_remove_as_percentage && value > 0 {
                                    value = -value;
                                } else if !self.shares_to_remove_as_percentage && value < 0 {
                                    value = -value;
                                }
                                self.config.shares_to_remove = vec![value];
                            }
                        });
                        
                        cols[1].vertical(|ui| {
                            ui.label(self.localization.get("llr_value"));
                            ui.horizontal(|ui| {
                                let llr_parsed = self.llr_value.parse::<f64>().unwrap_or(10.0);
                                let mut llr_val = llr_parsed;
                                ui.add(egui::Slider::new(&mut llr_val, 0.1..=100.0).text("LLR").logarithmic(true));
                                if (llr_val - llr_parsed).abs() > 0.001 {
                                    self.llr_value = format!("{:.2}", llr_val);
                                    self.config.llr_value = llr_val;
                                }
                                ui.add(egui::TextEdit::singleline(&mut self.llr_value)
                                    .desired_width(80.0));
                            });
                            
                            ui.label(self.localization.get("max_iterations"));
                            ui.horizontal(|ui| {
                                let iter_parsed = self.max_iterations_value.parse::<usize>().unwrap_or(500);
                                let mut iter_val = iter_parsed;
                                ui.add(egui::Slider::new(&mut iter_val, 1..=1000));
                                if iter_val != iter_parsed {
                                    self.max_iterations_value = iter_val.to_string();
                                    self.config.max_iterations = iter_val;
                                }
                                ui.add(egui::TextEdit::singleline(&mut self.max_iterations_value)
                                    .desired_width(80.0));
                            });
                        });
                    });
                    
                    ui.separator();
                    
                    ui.label(RichText::new(self.localization.get("secret_value")).strong());
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.secret_random, self.localization.get("secret_random"));
                        
                        if !self.secret_random {
                            ui.checkbox(&mut self.secret_hex_mode, self.localization.get("secret_hex"));
                            
                            let parse_result = if self.secret_hex_mode {
                                u128::from_str_radix(self.secret_value.trim_start_matches("0x"), 16)
                            } else {
                                self.secret_value.parse::<u128>()
                            };
                            
                            self.secret_valid = parse_result.is_ok();
                            
                            let text_edit = egui::TextEdit::singleline(&mut self.secret_value)
                                .desired_width(240.0)
                                .hint_text(if self.secret_hex_mode { "0x2A" } else { "42" });
                            
                            if !self.secret_valid {
                                ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::RED);
                                ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::RED);
                            }
                            ui.add(text_edit);
                            
                            if let Ok(val) = parse_result {
                                self.config.secret_value = val;
                            }
                        } else {
                            ui.label(self.localization.get("secret_seed"));
                            ui.add(egui::TextEdit::singleline(&mut self.secret_seed)
                                .desired_width(140.0)
                                .hint_text(self.localization.get("secret_seed_hint")));
                        }
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label(self.localization.get("runs_count"));
                        let runs_parsed = self.runs_value.parse::<usize>().unwrap_or(1);
                        let mut runs_val = runs_parsed;
                        ui.add(egui::Slider::new(&mut runs_val, 1..=50));
                        if runs_val != runs_parsed {
                            self.runs_value = runs_val.to_string();
                            self.config.runs_per_config = runs_val;
                        }
                        ui.add(egui::TextEdit::singleline(&mut self.runs_value)
                            .desired_width(80.0));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(self.localization.get("warmup_runs"));
                        let warmup_parsed = self.warmup_value.parse::<usize>().unwrap_or(1);
                        let mut warmup_val = warmup_parsed;
                        ui.add(egui::Slider::new(&mut warmup_val, 0..=5));
                        if warmup_val != warmup_parsed {
                            self.warmup_value = warmup_val.to_string();
                            self.config.warmup_runs = warmup_val;
                        }
                        ui.add(egui::TextEdit::singleline(&mut self.warmup_value)
                            .desired_width(80.0));
                    });
                    
                    ui.add_space(5.0);
                    ui.label(RichText::new(self.localization.get("implementation")).strong());
                    ui.horizontal(|ui| { 
                        let labels = [
                            self.localization.get("implementation_both"),
                            self.localization.get("implementation_sequential"),
                            self.localization.get("implementation_parallel"),
                        ];
                        
                        for (idx, label) in labels.iter().enumerate() {
                            let is_selected = self.selected_implementation == idx;
                            let response = ui.selectable_label(is_selected, *label);
                            if response.clicked() && !is_selected {
                                self.selected_implementation = idx;
                                self.config.implementations = match idx {
                                    0 => vec![Implementation::Sequential, Implementation::Parallel],
                                    1 => vec![Implementation::Sequential],
                                    2 => vec![Implementation::Parallel],
                                    _ => vec![Implementation::Sequential, Implementation::Parallel],
                                };
                            }
                        }
                    });
                });
            
            ui.add_space(10.0);
            
            egui::Frame::group(ui.style())
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(150, 150, 180)))
                .rounding(8.0)
                .inner_margin(egui::style::Margin::same(12.0))
                .outer_margin(egui::style::Margin::symmetric(0.0, 4.0))
                .show(ui, |ui| {
                    ui.heading(RichText::new(self.localization.get("code_params")).color(Color32::from_rgb(80, 150, 230)));
                    ui.add_space(5.0);
                    
                    let available_width = ui.available_width();
                    ui.set_min_width(f32::max(500.0, available_width));
                    
                    ui.label(RichText::new(self.localization.get("code_rate")).strong());
                    ui.horizontal_wrapped(|ui| {
                        let rates = [
                            ("R1_2", AR4JARate::R1_2), 
                            ("R2_3", AR4JARate::R2_3),
                            ("R4_5", AR4JARate::R4_5),
                        ];
                        
                        if self.selected_rates.len() < rates.len() {
                            self.selected_rates.resize(rates.len(), false);
                        }
                        
                        for (i, (name, _)) in rates.iter().enumerate() {
                            if ui.selectable_label(self.selected_rates[i], *name).clicked() {
                                self.selected_rates[i] = !self.selected_rates[i];
                            }
                        }
                        
                        self.config.ldpc_rates = rates.iter()
                            .enumerate()
                            .filter_map(|(i, (_, rate))| {
                                if i < self.selected_rates.len() && self.selected_rates[i] {
                                    Some(*rate)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        
                        if self.config.ldpc_rates.is_empty() {
                            self.config.ldpc_rates = vec![AR4JARate::R4_5];
                        }
                    });
                    
                    ui.label(RichText::new(self.localization.get("info_block_size")).strong());
                    ui.horizontal(|ui| {
                        let sizes = [
                            ("K1024", AR4JAInfoSize::K1024), 
                            ("K4096", AR4JAInfoSize::K4096),
                            ("K16384", AR4JAInfoSize::K16384),
                        ];
                        
                        for (i, (name, size)) in sizes.iter().enumerate() {
                            if ui.selectable_label(self.selected_size == i, *name).clicked() {
                                self.selected_size = i;
                                self.config.ldpc_info_sizes = vec![*size];
                            }
                        }
                    });
                    
                    ui.collapsing(
                        RichText::new(self.localization.get("select_decoders")).strong(),
                        |ui| {
                            self.decoder_selector.show(ui);
                            self.config.decoder_types = self.decoder_selector.get_selected_decoders();
                        }
                    );
                });
            
            ui.add_space(10.0);
            
            egui::Frame::group(ui.style())
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(150, 150, 180)))
                .rounding(8.0)
                .inner_margin(egui::style::Margin::same(12.0))
                .outer_margin(egui::style::Margin::symmetric(0.0, 4.0))
                .show(ui, |ui| {
                    let available_width = ui.available_width();
                    ui.set_min_width(f32::max(500.0, available_width));
                    
                    ui.heading(RichText::new(self.localization.get("output_settings")).color(Color32::from_rgb(80, 150, 230)));
                    ui.add_space(5.0);
                    
                    ui.checkbox(&mut self.config.show_detail, self.localization.get("show_details"));
                    ui.checkbox(&mut self.config.verbose, self.localization.get("verbose_logging"));
                    
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.config.save_results, self.localization.get("save_json"));
                        if self.config.save_results {
                            ui.add(egui::TextEdit::singleline(&mut self.config.output_filename)
                                .desired_width(ui.available_width() - 10.0)
                                .hint_text(self.localization.get("filename_auto")));
                        }
                    });
                });
            
            ui.add_space(20.0);
            
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                if is_running {
                    let stop_button = egui::Button::new(
                        RichText::new(self.localization.get("stop_benchmark"))
                            .text_style(egui::TextStyle::Heading)
                            .color(Color32::WHITE))
                        .fill(Color32::from_rgb(220, 80, 80))
                        .min_size(egui::vec2(280.0, 40.0))
                        .rounding(8.0);
                        
                    if ui.add(stop_button).clicked() {
                        action = Some(ConfigureAction::StopBenchmark);
                    }
                } else {
                    let run_button = egui::Button::new(
                        RichText::new(self.localization.get("run_benchmark"))
                            .text_style(egui::TextStyle::Heading)
                            .color(Color32::WHITE))
                        .fill(Color32::from_rgb(50, 150, 230))
                        .min_size(egui::vec2(280.0, 40.0))
                        .rounding(8.0);
                        
                    if ui.add(run_button).clicked() {
                        self.update_config_from_ui_values();
                        action = Some(ConfigureAction::RunBenchmark);
                    }
                }
                
                if !is_running {
                    ui.add_space(10.0);
                    if ui.button(self.localization.get("show_command")).clicked() {
                        self.update_config_from_ui_values();
                        let args = self.config.to_arg_strings();
                        let cmd = format!("cargo run -- benchmark {}", args.join(" "));
                        if self.command_line_display.as_ref() == Some(&cmd) {
                            self.command_line_display = None;
                        } else {
                            self.command_line_display = Some(cmd.clone());
                            action = Some(ConfigureAction::ShowCommandLine(cmd));
                        }
                    }
                }
                
                if let Some(cmd) = &self.command_line_display {
                    ui.add_space(8.0);
                    
                    egui::Frame::none()
                        .fill(Color32::from_rgb(40, 40, 50))
                        .rounding(4.0)
                        .inner_margin(egui::style::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(self.localization.get("command_line_label"))
                                    .size(14.0)
                                    .strong()
                                    .color(Color32::from_rgb(180, 180, 200)));
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("📋").on_hover_text(self.localization.get("copy_command")).clicked() {
                                        ui.output_mut(|o| o.copied_text = cmd.clone());
                                    }
                                });
                            });
                            
                            ui.add_space(4.0);
                            
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::Label::new(
                                    RichText::new(cmd)
                                        .monospace()
                                        .size(12.0)
                                        .color(Color32::from_rgb(180, 220, 180))
                                ).wrap(true));
                            });
                        });
                }
            });
        });
        
        action
    }
    
    fn update_config_from_ui_values(&mut self) {
        if let Ok(c_val) = self.c_value.parse::<usize>() {
            self.config.c_values = vec![c_val];
        }
        
        if let Ok(runs) = self.runs_value.parse::<usize>() {
            self.config.runs_per_config = runs;
        }
        
        if let Ok(llr) = self.llr_value.parse::<f64>() {
            self.config.llr_value = llr;
        }
        
        if let Ok(max_iter) = self.max_iterations_value.parse::<usize>() {
            self.config.max_iterations = max_iter;
        }
        
        if let Ok(mut value) = self.shares_to_remove_value.parse::<isize>() {
            if self.shares_to_remove_as_percentage && value > 0 {
                value = -value;
            } else if !self.shares_to_remove_as_percentage && value < 0 {
                value = -value;
            }
            self.config.shares_to_remove = vec![value];
        }
        
        self.config.secret_random = self.secret_random;
        if self.secret_random {
            self.config.secret_seed = self.secret_seed.parse::<u64>().ok();
            if let Some(seed) = self.config.secret_seed {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                seed.hash(&mut hasher);
                self.config.secret_value = hasher.finish() as u128;
            } else {
                use std::time::{SystemTime, UNIX_EPOCH};
                let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
                self.config.secret_value = duration.as_nanos() as u128;
            }
        } else {
            let parse_result = if self.secret_hex_mode {
                u128::from_str_radix(self.secret_value.trim_start_matches("0x"), 16)
            } else {
                self.secret_value.parse::<u128>()
            };
            if let Ok(val) = parse_result {
                self.config.secret_value = val;
            }
            self.config.secret_seed = None;
        }
        
        self.config.decoder_types = self.decoder_selector.get_selected_decoders();
    }
    
    pub fn get_config(&self) -> BenchmarkConfig {
        self.config.clone()
    }
}
