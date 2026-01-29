use eframe::egui::{self, RichText, ScrollArea, Ui};
use std::collections::HashMap;
use std::time::Duration;
use crate::benchmark::{BenchmarkSummary, BenchmarkParams, BenchmarkStats, Implementation};
use crate::ui::localization::Localization;
use crate::ui::constants::{self, heading_size};
use super::utils::format_duration;
use super::table_builder::{ResultsTable, phase_detail_columns};

fn draw_duration_with_bar(ui: &mut Ui, duration: Duration, min_duration: Duration, max_duration: Duration) {
    let text = format_duration(duration);
    
    let range = max_duration.as_secs_f64() - min_duration.as_secs_f64();
    let percentage = if range > 0.0 {
        ((duration.as_secs_f64() - min_duration.as_secs_f64()) / range).clamp(0.0, 1.0)
    } else {
        0.5
    };
    
    let cell_width = constants::DATA_BAR_WIDTH;
    let cell_height = constants::DATA_BAR_HEIGHT;
    let corner_radius = constants::DATA_BAR_CORNER_RADIUS;
    
    let desired_size = egui::vec2(cell_width, cell_height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        
        painter.rect_filled(rect, corner_radius, constants::data_bar_bg(ui));
        
        let bar_width = rect.width() * percentage as f32;
        if bar_width > 0.5 {
            let bar_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(bar_width, rect.height())
            );
            let bar_color = constants::data_bar_gradient(ui, percentage);
            painter.rect_filled(bar_rect, corner_radius, bar_color);
        }
        
        painter.rect_stroke(rect, corner_radius, constants::data_bar_stroke(ui));
        
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
        entries.sort_by(|a, b| {
            let decoder_a = format!("{:?}", a.0.decoder_type);
            let decoder_b = format!("{:?}", b.0.decoder_type);
            let decoder_cmp = decoder_a.cmp(&decoder_b);
            if decoder_cmp != std::cmp::Ordering::Equal {
                return decoder_cmp;
            }
            
            let rate_a = format!("{:?}", a.0.ldpc_rate);
            let rate_b = format!("{:?}", b.0.ldpc_rate);
            let rate_cmp = rate_a.cmp(&rate_b);
            if rate_cmp != std::cmp::Ordering::Equal {
                return rate_cmp;
            }
            
            match (a.0.implementation, b.0.implementation) {
                (Implementation::Sequential, Implementation::Parallel) => std::cmp::Ordering::Less,
                (Implementation::Parallel, Implementation::Sequential) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });
        
        let (avg_min, avg_max) = entries.iter()
            .map(|(_, s)| s.avg)
            .fold((Duration::MAX, Duration::ZERO), |(min, max), d| (min.min(d), max.max(d)));
        let (min_min, min_max) = entries.iter()
            .map(|(_, s)| s.min)
            .fold((Duration::MAX, Duration::ZERO), |(min, max), d| (min.min(d), max.max(d)));
        let (max_min, max_max) = entries.iter()
            .map(|(_, s)| s.max)
            .fold((Duration::MAX, Duration::ZERO), |(min, max), d| (min.min(d), max.max(d)));
        let (median_min, median_max) = entries.iter()
            .map(|(_, s)| s.median)
            .fold((Duration::MAX, Duration::ZERO), |(min, max), d| (min.min(d), max.max(d)));
        let (std_min, std_max) = entries.iter()
            .map(|(_, s)| s.std_dev)
            .fold((Duration::MAX, Duration::ZERO), |(min, max), d| (min.min(d), max.max(d)));
        
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
