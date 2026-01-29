use eframe::egui::{self, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use crate::ui::constants::{self, TABLE_ROW_HEIGHT};

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
    
    #[allow(dead_code)]
    pub fn with_initial_width(mut self, width: f32) -> Self {
        self.initial_width = Some(width);
        self
    }
    
    pub fn fixed(mut self) -> Self {
        self.resizable = false;
        self
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
    
    #[allow(dead_code)]
    pub fn with_row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }
    
    #[allow(dead_code)]
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
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
                let column = if i == num_columns - 1 {
                    Column::remainder().at_least(col.min_width)
                } else if let Some(initial) = col.initial_width {
                    if col.resizable {
                        Column::initial(initial).at_least(col.min_width).resizable(true)
                    } else {
                        Column::exact(initial)
                    }
                } else {
                    let estimated_width = (available_width / num_columns as f32).max(col.min_width);
                    if col.resizable {
                        Column::initial(estimated_width).at_least(col.min_width).resizable(true)
                    } else {
                        Column::auto().at_least(col.min_width)
                    }
                };
                builder = builder.column(column);
            }
            
            let mut row_idx = 0usize;
            
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

#[allow(dead_code)]
pub fn standard_benchmark_columns(
    col_impl: &str,
    col_block_size: &str,
    col_rate: &str,
    col_decoder: &str,
    col_avg: &str,
    col_min: &str,
    col_max: &str,
    col_median: &str,
    col_std_dev: &str,
    col_throughput: Option<&str>,
    col_success_rate: Option<&str>,
) -> Vec<TableColumn> {
    let mut columns = vec![
        TableColumn::new(col_impl).with_min_width(80.0),
        TableColumn::new("C").with_min_width(40.0).fixed(),
        TableColumn::new(col_block_size).with_min_width(80.0),
        TableColumn::new(col_rate).with_min_width(60.0),
        TableColumn::new(col_decoder).with_min_width(100.0),
        TableColumn::new(col_avg).with_min_width(80.0),
        TableColumn::new(col_min).with_min_width(80.0),
        TableColumn::new(col_max).with_min_width(80.0),
        TableColumn::new(col_median).with_min_width(70.0),
        TableColumn::new(col_std_dev).with_min_width(70.0),
    ];
    
    if let Some(throughput) = col_throughput {
        columns.push(TableColumn::new(throughput).with_min_width(80.0));
    }
    
    if let Some(success) = col_success_rate {
        columns.push(TableColumn::new(success).with_min_width(70.0));
    }
    
    columns
}

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
