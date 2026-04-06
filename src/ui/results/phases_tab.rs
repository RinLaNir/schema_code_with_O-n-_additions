use eframe::egui::{self, RichText, ScrollArea, Ui};
use ldpc_toolbox::codes::ccsds::{AR4JAInfoSize, AR4JARate};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use std::collections::HashMap;

use crate::benchmark::{BenchmarkParams, BenchmarkStats, BenchmarkSummary, PhaseStats};
use crate::types::DecodingStats;
use crate::ui::constants::{self, heading_size, small_size};
use crate::ui::localization::Localization;

use super::table_builder::{phase_breakdown_columns, ResultsTable};
use super::utils::{compare_benchmark_params, format_duration, show_phase_pie_chart};

#[derive(Clone)]
pub struct PhasesTab {
    summary: Option<BenchmarkSummary>,
    localization: Localization,
    all_expanded: bool,
}

impl PhasesTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            summary: None,
            localization,
            all_expanded: false,
        }
    }

    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }

    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        self.summary = Some(summary.clone());
    }

    fn format_info_size(&self, size: &AR4JAInfoSize) -> String {
        let key = match size {
            AR4JAInfoSize::K1024 => "info_size_k1024",
            AR4JAInfoSize::K4096 => "info_size_k4096",
            AR4JAInfoSize::K16384 => "info_size_k16384",
        };
        self.localization.get(key).to_string()
    }

    fn format_rate(&self, rate: &AR4JARate) -> String {
        let key = match rate {
            AR4JARate::R1_2 => "rate_r1_2",
            AR4JARate::R2_3 => "rate_r2_3",
            AR4JARate::R4_5 => "rate_r4_5",
        };
        self.localization.get(key).to_string()
    }

    fn format_decoder(&self, decoder: &DecoderImplementation) -> String {
        let debug_name = format!("{:?}", decoder);
        let key = format!("decoder_{}", debug_name.to_lowercase());
        let localized = self.localization.get(&key);
        if localized == "[Unknown key]" {
            debug_name
        } else {
            localized.to_string()
        }
    }

    fn format_section_header(&self, params: &BenchmarkParams) -> String {
        format!(
            "{} | ell={} | {} | {} | {}",
            params.implementation,
            params.secret.bit_len,
            self.format_info_size(&params.ldpc_info_size),
            self.format_rate(&params.ldpc_rate),
            self.format_decoder(&params.decoder_type)
        )
    }

    fn make_section_id(prefix: &str, params: &BenchmarkParams) -> String {
        format!(
            "{}_{:?}_{:?}_{:?}_{:?}_{:?}",
            prefix,
            params.implementation,
            params.secret.bit_len,
            params.ldpc_info_size,
            params.ldpc_rate,
            params.decoder_type
        )
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let expand_all_text = self.localization.get("expand_all").to_string();
        let collapse_all_text = self.localization.get("collapse_all").to_string();

        ScrollArea::vertical().show(ui, |ui| {
            if let Some(ref summary) = self.summary {
                ui.horizontal(|ui| {
                    if ui.button(&expand_all_text).clicked() {
                        self.all_expanded = true;
                    }
                    if ui.button(&collapse_all_text).clicked() {
                        self.all_expanded = false;
                    }
                });
                ui.add_space(constants::ITEM_SPACING);

                if !summary.deal_stats.is_empty() {
                    self.show_stats_section(
                        ui,
                        &summary.deal_stats,
                        "deal",
                        "deal_phases_title",
                        false,
                    );
                }

                ui.add_space(constants::SECTION_SPACING);

                if !summary.reconstruct_stats.is_empty() {
                    self.show_stats_section(
                        ui,
                        &summary.reconstruct_stats,
                        "reconstruct",
                        "reconstruct_phases_title",
                        true,
                    );
                }
            }
        });
    }

    fn show_stats_section(
        &self,
        ui: &mut Ui,
        stats_map: &HashMap<BenchmarkParams, BenchmarkStats>,
        prefix: &str,
        heading_key: &str,
        show_decoding: bool,
    ) {
        ui.heading(RichText::new(self.localization.get(heading_key)).size(heading_size(ui)));
        ui.add_space(constants::SMALL_SPACING);

        let mut entries: Vec<_> = stats_map.iter().collect();
        entries.sort_by(|a, b| compare_benchmark_params(a.0, b.0));

        for (params, stats) in entries {
            let Some(phase_metrics) = &stats.phase_metrics else {
                continue;
            };

            let section_id = Self::make_section_id(prefix, params);
            let header = self.format_section_header(params);

            ui.push_id(format!("{}_section_{}", prefix, &section_id), |ui| {
                let header_state = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    ui.make_persistent_id(format!("{}_collapse_{}", prefix, &section_id)),
                    self.all_expanded,
                );

                header_state
                    .show_header(ui, |ui| {
                        ui.label(
                            RichText::new(&header)
                                .size(constants::scaled_size(ui, constants::SUBHEADING_SCALE)),
                        );
                    })
                    .body(|ui| {
                        self.show_phase_details(ui, phase_metrics, &section_id);

                        if show_decoding {
                            if let Some(decoding_stats) = &stats.decoding_stats {
                                self.show_decoding_stats(ui, decoding_stats, &section_id);
                            }
                        }
                    });
            });
        }
    }

    fn show_phase_details(
        &self,
        ui: &mut Ui,
        phase_metrics: &HashMap<String, PhaseStats>,
        section_id: &str,
    ) {
        let mut phases: Vec<_> = phase_metrics.iter().collect();
        phases.sort_by(|(_, a), (_, b)| b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());

        let phases_for_table: Vec<_> = phases
            .iter()
            .map(|(name, stat)| (name.to_string(), (*stat).clone()))
            .collect();

        let columns = phase_breakdown_columns(
            self.localization.get("col_phase"),
            self.localization.get("col_avg_time"),
            self.localization.get("col_min_time"),
            self.localization.get("col_max_time"),
            self.localization.get("col_percent_total"),
        );

        ResultsTable::new(&format!("{}_phases_table", section_id), columns).show(
            ui,
            phases_for_table.len(),
            |row_idx, row| {
                let (name, phase_stat) = &phases_for_table[row_idx];

                row.col(|ui| {
                    ui.label(name);
                });
                row.col(|ui| {
                    ui.label(format_duration(phase_stat.avg_duration));
                });
                row.col(|ui| {
                    ui.label(format_duration(phase_stat.min_duration));
                });
                row.col(|ui| {
                    ui.label(format_duration(phase_stat.max_duration));
                });
                row.col(|ui| {
                    ui.label(format!("{:.2}%", phase_stat.avg_percentage));
                });
            },
        );

        show_phase_pie_chart(
            ui,
            phase_metrics,
            self.localization.get("phase_distribution"),
        );
    }

    fn show_decoding_stats(&self, ui: &mut Ui, decoding_stats: &DecodingStats, section_id: &str) {
        ui.add_space(constants::ITEM_SPACING);
        ui.separator();
        ui.add_space(constants::SMALL_SPACING);
        ui.label(
            RichText::new(self.localization.get("decoding_stats_title"))
                .strong()
                .size(small_size(ui) * 1.1),
        );
        ui.add_space(constants::SMALL_SPACING);

        egui::Grid::new(format!("decoding_stats_{}", section_id))
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label(self.localization.get("total_rows"));
                ui.label(format!("{}", decoding_stats.total_rows));
                ui.end_row();

                ui.label(self.localization.get("successful_rows"));
                let success_color = constants::rate_color(ui, decoding_stats.success_rate());
                ui.label(
                    RichText::new(format!(
                        "{} ({:.1}%)",
                        decoding_stats.successful_rows,
                        decoding_stats.success_rate() * 100.0
                    ))
                    .color(success_color),
                );
                ui.end_row();

                if decoding_stats.failed_rows > 0 {
                    ui.label(self.localization.get("failed_rows"));
                    ui.label(
                        RichText::new(format!("{}", decoding_stats.failed_rows))
                            .color(constants::error_color(ui)),
                    );
                    ui.end_row();
                }

                ui.label(self.localization.get("avg_iterations"));
                ui.label(format!("{:.2}", decoding_stats.avg_iterations));
                ui.end_row();

                if decoding_stats.max_iterations_hit > 0 {
                    ui.label(self.localization.get("max_iter_hit"));
                    let hit_rate = decoding_stats.max_iterations_hit as f64
                        / decoding_stats.total_rows as f64
                        * 100.0;
                    ui.label(
                        RichText::new(format!(
                            "{} ({:.1}%)",
                            decoding_stats.max_iterations_hit, hit_rate
                        ))
                        .color(constants::warning_color(ui)),
                    );
                    ui.end_row();
                }
            });
    }
}
