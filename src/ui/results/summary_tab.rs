use super::table_builder::TableColumn;
use super::utils::{compare_benchmark_params, format_duration};
use crate::benchmark::{BenchmarkParams, BenchmarkStats, BenchmarkSummary};
use crate::ui::constants::{self, heading_size, TABLE_ROW_HEIGHT};
use crate::ui::localization::Localization;
use eframe::egui::{RichText, ScrollArea, Sense, Ui};
use egui_extras::{Column, TableBuilder};

#[derive(Clone, Copy, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    fn arrow(&self) -> &'static str {
        match self {
            SortDirection::Ascending => "▲",
            SortDirection::Descending => "▼",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortColumn {
    Implementation,
    SecretBits,
    BlockSize,
    Rate,
    Decoder,
    AvgTime,
    MinTime,
    MaxTime,
    Median,
    StdDev,
    Throughput,
    SuccessRate,
}

#[derive(Clone)]
pub struct SummaryTab {
    summary: Option<BenchmarkSummary>,
    localization: Localization,
    sort_column: Option<SortColumn>,
    sort_direction: SortDirection,
}

impl SummaryTab {
    pub fn new(localization: Localization) -> Self {
        Self {
            summary: None,
            localization,
            sort_column: None,
            sort_direction: SortDirection::Ascending,
        }
    }

    pub fn update_localization(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }

    pub fn update_with_summary(&mut self, summary: &BenchmarkSummary) {
        self.summary = Some(summary.clone());
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let mut clicked_column: Option<SortColumn> = None;

        ScrollArea::both().show(ui, |ui| {
            if let Some(ref summary) = self.summary {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new(self.localization.get("total_execution_time"))
                            .size(heading_size(ui)),
                    );
                    ui.add_space(constants::ITEM_SPACING);
                    if self.sort_column.is_some()
                        && ui.button(self.localization.get("reset_sort")).clicked()
                    {
                        self.sort_column = None;
                    }
                });
                ui.add_space(constants::SMALL_SPACING);

                let mut entries: Vec<_> = summary
                    .total_stats
                    .iter()
                    .map(|(p, s)| (p.clone(), s.clone()))
                    .collect();
                Self::apply_sort(self.sort_column, self.sort_direction, &mut entries, summary);

                let columns = self.build_columns();

                ui.push_id("summary_section", |ui| {
                    clicked_column = Self::show_sortable_table(
                        self.sort_column,
                        self.sort_direction,
                        ui,
                        &entries,
                        &columns,
                        summary,
                    );
                });
            }
        });

        if let Some(col) = clicked_column {
            if self.sort_column == Some(col) {
                self.sort_direction = self.sort_direction.toggle();
            } else {
                self.sort_column = Some(col);
                self.sort_direction = SortDirection::Ascending;
            }
        }
    }

    fn build_columns(&self) -> Vec<(TableColumn, SortColumn)> {
        vec![
            (
                TableColumn::new(self.localization.get("col_implementation")).with_min_width(80.0),
                SortColumn::Implementation,
            ),
            (
                TableColumn::new("ell").with_min_width(50.0).fixed(),
                SortColumn::SecretBits,
            ),
            (
                TableColumn::new(self.localization.get("col_block_size")).with_min_width(80.0),
                SortColumn::BlockSize,
            ),
            (
                TableColumn::new(self.localization.get("col_rate")).with_min_width(60.0),
                SortColumn::Rate,
            ),
            (
                TableColumn::new(self.localization.get("col_decoder")).with_min_width(100.0),
                SortColumn::Decoder,
            ),
            (
                TableColumn::new(self.localization.get("col_avg_time")).with_min_width(80.0),
                SortColumn::AvgTime,
            ),
            (
                TableColumn::new(self.localization.get("col_min_time")).with_min_width(80.0),
                SortColumn::MinTime,
            ),
            (
                TableColumn::new(self.localization.get("col_max_time")).with_min_width(80.0),
                SortColumn::MaxTime,
            ),
            (
                TableColumn::new(self.localization.get("col_median_time")).with_min_width(70.0),
                SortColumn::Median,
            ),
            (
                TableColumn::new(self.localization.get("col_std_dev")).with_min_width(70.0),
                SortColumn::StdDev,
            ),
            (
                TableColumn::new(self.localization.get("col_throughput")).with_min_width(80.0),
                SortColumn::Throughput,
            ),
            (
                TableColumn::new(self.localization.get("col_success_rate")).with_min_width(70.0),
                SortColumn::SuccessRate,
            ),
        ]
    }

    fn show_sortable_table(
        sort_column: Option<SortColumn>,
        sort_direction: SortDirection,
        ui: &mut Ui,
        entries: &[(BenchmarkParams, BenchmarkStats)],
        columns: &[(TableColumn, SortColumn)],
        summary: &BenchmarkSummary,
    ) -> Option<SortColumn> {
        let available_width = ui.available_width();
        let num_columns = columns.len();

        let mut builder = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(eframe::egui::Layout::left_to_right(
                eframe::egui::Align::Center,
            ))
            .min_scrolled_height(0.0)
            .vscroll(false);

        for (i, (col, _)) in columns.iter().enumerate() {
            let column = if i == num_columns - 1 {
                Column::remainder().at_least(col.min_width)
            } else {
                let estimated_width = (available_width / num_columns as f32).max(col.min_width);
                if col.resizable {
                    Column::initial(estimated_width)
                        .at_least(col.min_width)
                        .resizable(true)
                } else {
                    Column::auto().at_least(col.min_width)
                }
            };
            builder = builder.column(column);
        }

        let mut clicked_column: Option<SortColumn> = None;

        builder
            .header(TABLE_ROW_HEIGHT, |mut header| {
                for (col, sort_col) in columns {
                    header.col(|ui| {
                        let is_sorted = sort_column == Some(*sort_col);
                        let header_text = if is_sorted {
                            format!("{} {}", col.header, sort_direction.arrow())
                        } else {
                            col.header.clone()
                        };

                        let response = ui.add(
                            eframe::egui::Label::new(RichText::new(&header_text).strong())
                                .sense(Sense::click()),
                        );

                        if response.clicked() {
                            clicked_column = Some(*sort_col);
                        }

                        if response.hovered() {
                            ui.ctx()
                                .set_cursor_icon(eframe::egui::CursorIcon::PointingHand);
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(TABLE_ROW_HEIGHT, entries.len(), |mut row| {
                    let row_idx = row.index();
                    let (params, stats) = &entries[row_idx];

                    row.col(|ui| {
                        ui.label(params.implementation.to_string());
                    });
                    row.col(|ui| {
                        ui.label(format!("{}", params.secret.bit_len));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:?}", params.ldpc_info_size));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:?}", params.ldpc_rate));
                    });
                    row.col(|ui| {
                        ui.label(format!("{:?}", params.decoder_type));
                    });
                    row.col(|ui| {
                        ui.label(format_duration(stats.avg));
                    });
                    row.col(|ui| {
                        ui.label(format_duration(stats.min));
                    });
                    row.col(|ui| {
                        ui.label(format_duration(stats.max));
                    });
                    row.col(|ui| {
                        ui.label(format_duration(stats.median));
                    });
                    row.col(|ui| {
                        ui.label(format_duration(stats.std_dev));
                    });

                    row.col(|ui| {
                        let throughput_text =
                            if let Some(deal_stats) = summary.deal_stats.get(params) {
                                if let Some(throughput) = &deal_stats.throughput {
                                    format!("{:.1} sh/s", throughput.shares_per_second)
                                } else {
                                    "-".to_string()
                                }
                            } else {
                                "-".to_string()
                            };
                        ui.label(throughput_text);
                    });

                    row.col(|ui| {
                        let success_text = format!("{:.0}%", stats.success_rate * 100.0);
                        let success_color = constants::rate_color(ui, stats.success_rate);
                        ui.label(RichText::new(success_text).color(success_color));
                    });
                });
            });

        clicked_column
    }

    fn apply_sort(
        sort_column: Option<SortColumn>,
        sort_direction: SortDirection,
        entries: &mut [(BenchmarkParams, BenchmarkStats)],
        summary: &BenchmarkSummary,
    ) {
        let sort_col = match sort_column {
            Some(col) => col,
            None => {
                entries.sort_by(|a, b| compare_benchmark_params(&a.0, &b.0));
                return;
            }
        };

        entries.sort_by(|a, b| {
            let cmp = match sort_col {
                SortColumn::Implementation => {
                    a.0.implementation
                        .to_string()
                        .cmp(&b.0.implementation.to_string())
                }
                SortColumn::SecretBits => a.0.secret.bit_len.cmp(&b.0.secret.bit_len),
                SortColumn::BlockSize => {
                    format!("{:?}", a.0.ldpc_info_size).cmp(&format!("{:?}", b.0.ldpc_info_size))
                }
                SortColumn::Rate => {
                    format!("{:?}", a.0.ldpc_rate).cmp(&format!("{:?}", b.0.ldpc_rate))
                }
                SortColumn::Decoder => {
                    format!("{:?}", a.0.decoder_type).cmp(&format!("{:?}", b.0.decoder_type))
                }
                SortColumn::AvgTime => a.1.avg.cmp(&b.1.avg),
                SortColumn::MinTime => a.1.min.cmp(&b.1.min),
                SortColumn::MaxTime => a.1.max.cmp(&b.1.max),
                SortColumn::Median => a.1.median.cmp(&b.1.median),
                SortColumn::StdDev => a.1.std_dev.cmp(&b.1.std_dev),
                SortColumn::Throughput => {
                    let throughput = |p: &BenchmarkParams| {
                        summary
                            .deal_stats
                            .get(p)
                            .and_then(|s| s.throughput.as_ref())
                            .map_or(0.0, |t| t.shares_per_second)
                    };
                    throughput(&a.0)
                        .partial_cmp(&throughput(&b.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                SortColumn::SuccessRate => {
                    a.1.success_rate
                        .partial_cmp(&b.1.success_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            };

            match sort_direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
    }
}
