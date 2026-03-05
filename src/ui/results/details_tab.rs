use eframe::egui::{self, RichText, ScrollArea, Ui};
use std::collections::HashMap;
use std::time::Duration;
use crate::benchmark::{BenchmarkSummary, BenchmarkParams, BenchmarkStats};
use crate::ui::localization::Localization;
use crate::ui::constants::{self, heading_size};
use super::utils::{format_duration, compare_benchmark_params};
use super::table_builder::{ResultsTable, phase_detail_columns};

fn draw_duration_with_bar(ui: &mut Ui, duration: Duration, min_duration: Duration, max_duration: Duration) {
    let text = format_duration(duration);
    
    let range = max_duration.as_secs_f64() - min_duration.as_secs_f64();
    let percentage = if range > 0.0 {
        ((duration.as_secs_f64() - min_duration.as_secs_f64()) / range).clamp(0.0, 1.0)
    } else {
        0.5
    };
    
    let desired_size = egui::vec2(constants::DATA_BAR_WIDTH, constants::DATA_BAR_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        
        painter.rect_filled(rect, constants::DATA_BAR_CORNER_RADIUS, constants::data_bar_bg(ui));
        
        let bar_width = rect.width() * percentage as f32;
        if bar_width > 0.5 {
            let bar_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(bar_width, rect.height())
            );
            painter.rect_filled(bar_rect, constants::DATA_BAR_CORNER_RADIUS, constants::data_bar_gradient(ui, percentage));
        }
        
        painter.rect_stroke(rect, constants::DATA_BAR_CORNER_RADIUS, constants::data_bar_stroke(ui), egui::StrokeKind::Outside);
        
        let text_pos = rect.right_center() - egui::vec2(constants::DATA_BAR_TEXT_PADDING, 0.0);
        let font_id = egui::FontId::new(
            constants::small_size(ui),
            egui::FontFamily::Monospace
        );
        
        let shadow_color = if ui.visuals().dark_mode {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200)
        };
        
        for offset in [
            egui::vec2(-1.0, 0.0),
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, -1.0),
            egui::vec2(0.0, 1.0),
        ] {
            painter.text(
                text_pos + offset,
                egui::Align2::RIGHT_CENTER,
                &text,
                font_id.clone(),
                shadow_color,
            );
        }
        
        let text_color = if ui.visuals().dark_mode {
            egui::Color32::WHITE
        } else {
            egui::Color32::BLACK
        };
        painter.text(
            text_pos,
            egui::Align2::RIGHT_CENTER,
            &text,
            font_id,
            text_color,
        );
    }
    
    response.on_hover_text(format!(
        "{} ({:.1}%)",
        text,
        percentage * 100.0
    ));
}

#[derive(Clone)]
pub struct DetailsTab {
    summary: Option<BenchmarkSummary>,
    localization: Localization,
}

impl DetailsTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            summary: None,
            localization,
        }
    }
    
    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        self.summary = Some(summary.clone());
    }
    
    pub fn show(&self, ui: &mut Ui) {
        ScrollArea::both().show(ui, |ui| {
            if let Some(summary) = &self.summary {
                ui.push_id("setup_times_section", |ui| {
                    self.show_section(
                        ui,
                        self.localization.get("setup_time_title"),
                        &summary.setup_stats,
                        "setup",
                    );
                });
                
                ui.add_space(constants::SECTION_SPACING);
                
                ui.push_id("deal_times_section", |ui| {
                    self.show_section(
                        ui,
                        self.localization.get("deal_time_title"),
                        &summary.deal_stats,
                        "deal",
                    );
                });
                
                ui.add_space(constants::SECTION_SPACING);
                
                ui.push_id("reconstruct_times_section", |ui| {
                    self.show_section(
                        ui,
                        self.localization.get("reconstruct_time_title"),
                        &summary.reconstruct_stats,
                        "reconstruct",
                    );
                });
            }
        });
    }
    
    fn show_section(
        &self,
        ui: &mut Ui,
        title: &str,
        stats: &HashMap<BenchmarkParams, BenchmarkStats>,
        section_id: &str,
    ) {
        ui.heading(RichText::new(title).size(heading_size(ui)));
        ui.add_space(constants::ITEM_SPACING);
        self.show_phase_table(ui, stats, section_id);
    }
    
    fn show_phase_table(&self, ui: &mut Ui, stats: &HashMap<BenchmarkParams, BenchmarkStats>, section_id: &str) {
        if stats.is_empty() {
            ui.label(RichText::new("-").weak());
            return;
        }
        
        let mut entries: Vec<_> = stats.iter().collect();
        entries.sort_by(|a, b| compare_benchmark_params(a.0, b.0));
        
        let dur_range = |get: fn(&BenchmarkStats) -> Duration| -> (Duration, Duration) {
            entries.iter()
                .map(|(_, s)| get(s))
                .fold((Duration::MAX, Duration::ZERO), |(lo, hi), d| (lo.min(d), hi.max(d)))
        };
        
        let (avg_min, avg_max)    = dur_range(|s| s.avg);
        let (min_min, min_max)    = dur_range(|s| s.min);
        let (max_min, max_max)    = dur_range(|s| s.max);
        let (median_min, median_max) = dur_range(|s| s.median);
        let (std_min, std_max)    = dur_range(|s| s.std_dev);
        
        let entries_for_table: Vec<_> = entries.iter()
            .map(|(p, s)| ((*p).clone(), (*s).clone()))
            .collect();
        
        let columns = phase_detail_columns(
            self.localization.get("col_implementation"),
            self.localization.get("col_block_size"),
            self.localization.get("col_rate"),
            self.localization.get("col_decoder"),
            self.localization.get("col_avg_time"),
            self.localization.get("col_min_time"),
            self.localization.get("col_max_time"),
            self.localization.get("col_median_time"),
            self.localization.get("col_std_dev"),
        );
        
        ResultsTable::new(&format!("{}_phase_table", section_id), columns)
            .show(ui, entries_for_table.len(), |row_idx, row| {
                let (params, stat) = &entries_for_table[row_idx];
                
                row.col(|ui| { ui.label(format!("{}", params.implementation)); });
                row.col(|ui| { ui.label(format!("{}", params.c_value)); });
                row.col(|ui| { ui.label(format!("{:?}", params.ldpc_info_size)); });
                row.col(|ui| { ui.label(format!("{:?}", params.ldpc_rate)); });
                row.col(|ui| { ui.label(format!("{:?}", params.decoder_type)); });
                row.col(|ui| { draw_duration_with_bar(ui, stat.avg, avg_min, avg_max); });
                row.col(|ui| { draw_duration_with_bar(ui, stat.min, min_min, min_max); });
                row.col(|ui| { draw_duration_with_bar(ui, stat.max, max_min, max_max); });
                row.col(|ui| { draw_duration_with_bar(ui, stat.median, median_min, median_max); });
                row.col(|ui| { draw_duration_with_bar(ui, stat.std_dev, std_min, std_max); });
            });
    }
}
