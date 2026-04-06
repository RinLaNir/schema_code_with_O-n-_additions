use crate::ui::constants::{self, TABLE_ROW_HEIGHT};
use eframe::egui::{self, RichText, Ui};
use egui_extras::{Column, TableBuilder};

#[derive(Clone)]
pub struct TableColumn {
    pub header: String,
    pub min_width: f32,
    pub resizable: bool,
    pub initial_width: Option<f32>,
}

impl TableColumn {
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            min_width: constants::MIN_COLUMN_WIDTH,
            resizable: true,
            initial_width: None,
        }
    }

    pub fn with_min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    pub fn fixed(mut self) -> Self {
        self.resizable = false;
        self
    }

    fn to_egui_column(&self, is_last: bool, available_width: f32, num_columns: usize) -> Column {
        if is_last {
            return Column::remainder().at_least(self.min_width);
        }
        if let Some(initial) = self.initial_width {
            return if self.resizable {
                Column::initial(initial)
                    .at_least(self.min_width)
                    .resizable(true)
            } else {
                Column::exact(initial)
            };
        }
        let estimated_width = (available_width / num_columns as f32).max(self.min_width);
        if self.resizable {
            Column::initial(estimated_width)
                .at_least(self.min_width)
                .resizable(true)
        } else {
            Column::auto().at_least(self.min_width)
        }
    }
}

pub struct ResultsTable<'a> {
    id: &'a str,
    columns: Vec<TableColumn>,
    striped: bool,
    row_height: f32,
}

impl<'a> ResultsTable<'a> {
    pub fn new(id: &'a str, columns: Vec<TableColumn>) -> Self {
        Self {
            id,
            columns,
            striped: true,
            row_height: TABLE_ROW_HEIGHT,
        }
    }

    pub fn show<F>(self, ui: &mut Ui, row_count: usize, mut body_fn: F)
    where
        F: FnMut(usize, &mut egui_extras::TableRow),
    {
        let available_width = ui.available_width();

        ui.push_id(self.id, |ui| {
            let mut builder = TableBuilder::new(ui)
                .striped(self.striped)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .min_scrolled_height(0.0)
                .vscroll(false);

            let num_columns = self.columns.len();
            for (i, col) in self.columns.iter().enumerate() {
                builder = builder.column(col.to_egui_column(
                    i == num_columns - 1,
                    available_width,
                    num_columns,
                ));
            }

            let mut row_idx = 0;

            builder
                .header(self.row_height, |mut header| {
                    for col in &self.columns {
                        header.col(|ui| {
                            ui.label(RichText::new(&col.header).strong());
                        });
                    }
                })
                .body(|body| {
                    body.rows(self.row_height, row_count, |mut row| {
                        body_fn(row_idx, &mut row);
                        row_idx += 1;
                    });
                });
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn phase_detail_columns(
    col_impl: &str,
    col_block_size: &str,
    col_rate: &str,
    col_decoder: &str,
    col_avg: &str,
    col_min: &str,
    col_max: &str,
    col_median: &str,
    col_std_dev: &str,
) -> Vec<TableColumn> {
    vec![
        TableColumn::new(col_impl).with_min_width(85.0),
        TableColumn::new("C").with_min_width(35.0).fixed(),
        TableColumn::new(col_block_size).with_min_width(75.0),
        TableColumn::new(col_rate).with_min_width(55.0),
        TableColumn::new(col_decoder).with_min_width(85.0),
        TableColumn::new(col_avg).with_min_width(115.0),
        TableColumn::new(col_min).with_min_width(115.0),
        TableColumn::new(col_max).with_min_width(115.0),
        TableColumn::new(col_median).with_min_width(115.0),
        TableColumn::new(col_std_dev).with_min_width(115.0),
    ]
}

pub fn phase_breakdown_columns(
    col_phase: &str,
    col_avg: &str,
    col_min: &str,
    col_max: &str,
    col_percent: &str,
) -> Vec<TableColumn> {
    vec![
        TableColumn::new(col_phase).with_min_width(150.0),
        TableColumn::new(col_avg).with_min_width(80.0),
        TableColumn::new(col_min).with_min_width(80.0),
        TableColumn::new(col_max).with_min_width(80.0),
        TableColumn::new(col_percent).with_min_width(70.0),
    ]
}
