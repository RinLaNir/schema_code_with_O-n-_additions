use std::time::Duration;
use std::cmp::Ordering;
use eframe::egui::{self, Color32, Ui};
use std::collections::HashMap;
use crate::benchmark::{BenchmarkParams, PhaseStats, Implementation};

/// Standard comparison for BenchmarkParams entries.
///
/// Sorts by decoder type, then rate, then implementation (sequential first).
/// Uses Debug representation for enum ordering because external ldpc-toolbox
/// enums (`DecoderImplementation`, `AR4JARate`) don't implement `Ord`.
pub fn compare_benchmark_params(a: &BenchmarkParams, b: &BenchmarkParams) -> Ordering {
    format!("{:?}", a.decoder_type)
        .cmp(&format!("{:?}", b.decoder_type))
        .then_with(|| format!("{:?}", a.ldpc_rate).cmp(&format!("{:?}", b.ldpc_rate)))
        .then_with(|| match (a.implementation, b.implementation) {
            (Implementation::Sequential, Implementation::Parallel) => Ordering::Less,
            (Implementation::Parallel, Implementation::Sequential) => Ordering::Greater,
            _ => Ordering::Equal,
        })
}

pub fn format_duration(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let total_us = duration.as_micros();

    if total_ms < 1 {
        format!("{} μs", total_us)
    } else if total_ms < 1000 {
        format!("{}.{:03} ms", total_ms, total_us % 1000)
    } else {
        format!("{}.{:03} s", total_ms / 1000, total_ms % 1000)
    }
}

pub fn show_phase_pie_chart(ui: &mut Ui, phase_metrics: &HashMap<String, PhaseStats>, phase_distribution_label: &str) {
    ui.add_space(10.0);
    ui.label(phase_distribution_label);
    
    let mut phases: Vec<_> = phase_metrics.iter().collect();
    phases.sort_by(|(_, a), (_, b)| b.avg_percentage.partial_cmp(&a.avg_percentage).unwrap());
    
    let colors = [
        Color32::from_rgb(235, 64, 52),   // Red
        Color32::from_rgb(66, 135, 245),  // Blue
        Color32::from_rgb(252, 186, 3),   // Yellow
        Color32::from_rgb(50, 168, 82),   // Green
        Color32::from_rgb(142, 36, 170),  // Purple
        Color32::from_rgb(240, 128, 60),  // Orange
        Color32::from_rgb(66, 189, 168),  // Teal
        Color32::from_rgb(194, 24, 91),   // Pink
        Color32::from_rgb(97, 97, 97),    // Gray
    ];
    
    ui.horizontal_wrapped(|ui| {
        for (i, (name, phase_stat)) in phases.iter().enumerate() {
            let color = colors[i % colors.len()];
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(16.0, 16.0), 
                        egui::Sense::hover()
                    );
                    ui.painter().rect_filled(rect, 2.0, color);
                    
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(*name).strong());
                        ui.label(format!("{:.1}% ({:.2}ms)",
                            phase_stat.avg_percentage,
                            phase_stat.avg_duration.as_secs_f64() * 1000.0));
                    });
                });
            });
        }
    });
    
    let total_height = 24.0;
    let total_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_width, total_height), egui::Sense::hover());
    
    let mut current_x = rect.left();
    for (i, (_, phase_stat)) in phases.iter().enumerate() {
        let width = (phase_stat.avg_percentage / 100.0 * total_width as f64) as f32;
        if width > 1.0 {
            let segment_rect = egui::Rect::from_min_size(
                egui::pos2(current_x, rect.top()),
                egui::vec2(width, total_height)
            );
            ui.painter().rect_filled(segment_rect, 0.0, colors[i % colors.len()]);
            current_x += width;
        }
    }
}