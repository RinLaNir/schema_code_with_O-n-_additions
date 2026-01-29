use eframe::egui::{Color32, TextStyle, Ui};

pub const HEADING_SCALE: f32 = 1.4;
pub const SUBHEADING_SCALE: f32 = 1.2;
#[allow(dead_code)]
pub const BODY_SCALE: f32 = 1.0;
pub const SMALL_SCALE: f32 = 0.85;
#[allow(dead_code)]
pub const TINY_SCALE: f32 = 0.75;

pub const SECTION_SPACING: f32 = 20.0;
pub const ITEM_SPACING: f32 = 10.0;
pub const SMALL_SPACING: f32 = 5.0;

pub const TABLE_ROW_HEIGHT: f32 = 24.0;
pub const MIN_COLUMN_WIDTH: f32 = 60.0;

pub fn scaled_size(ui: &Ui, scale: f32) -> f32 {
    ui.text_style_height(&TextStyle::Body) * scale
}

pub fn heading_size(ui: &Ui) -> f32 {
    scaled_size(ui, HEADING_SCALE)
}

#[allow(dead_code)]
pub fn subheading_size(ui: &Ui) -> f32 {
    scaled_size(ui, SUBHEADING_SCALE)
}

pub fn small_size(ui: &Ui) -> f32 {
    scaled_size(ui, SMALL_SCALE)
}

pub fn success_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(80, 200, 80)
    } else {
        Color32::from_rgb(20, 140, 20)
    }
}

pub fn warning_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(230, 200, 60)
    } else {
        Color32::from_rgb(180, 140, 0)
    }
}

pub fn error_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(230, 80, 80)
    } else {
        Color32::from_rgb(180, 40, 40)
    }
}

#[allow(dead_code)]
pub fn primary_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(100, 160, 230)
    } else {
        Color32::from_rgb(40, 100, 180)
    }
}

pub fn secondary_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(230, 160, 100)
    } else {
        Color32::from_rgb(180, 100, 40)
    }
}

pub fn rate_color(ui: &Ui, rate: f64) -> Color32 {
    if rate >= 0.99 {
        success_color(ui)
    } else if rate >= 0.8 {
        warning_color(ui)
    } else {
        error_color(ui)
    }
}

pub fn speedup_color(ui: &Ui, speedup: f64) -> Color32 {
    if speedup >= 2.5 {
        success_color(ui)
    } else if speedup >= 1.5 {
        warning_color(ui)
    } else {
        secondary_color(ui)
    }
}

pub fn efficiency_color(ui: &Ui, efficiency: f64) -> Color32 {
    if efficiency >= 70.0 {
        success_color(ui)
    } else if efficiency >= 40.0 {
        warning_color(ui)
    } else {
        secondary_color(ui)
    }
}

#[allow(dead_code)]
pub fn setup_section_bg(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(70, 130, 200, 15)
    } else {
        Color32::from_rgba_unmultiplied(70, 130, 200, 25)
    }
}

#[allow(dead_code)]
pub fn deal_section_bg(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(70, 180, 100, 15)
    } else {
        Color32::from_rgba_unmultiplied(70, 180, 100, 25)
    }
}

#[allow(dead_code)]
pub fn reconstruct_section_bg(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(200, 140, 70, 15)
    } else {
        Color32::from_rgba_unmultiplied(200, 140, 70, 25)
    }
}

#[allow(dead_code)]
pub fn setup_border_color(_ui: &Ui) -> Color32 {
    Color32::from_rgb(70, 130, 200)
}

#[allow(dead_code)]
pub fn deal_border_color(_ui: &Ui) -> Color32 {
    Color32::from_rgb(70, 180, 100)
}

#[allow(dead_code)]
pub fn reconstruct_border_color(_ui: &Ui) -> Color32 {
    Color32::from_rgb(200, 140, 70)
}

#[allow(dead_code)]
pub fn chart_colors() -> Vec<Color32> {
    vec![
        Color32::from_rgb(235, 64, 52),
        Color32::from_rgb(66, 135, 245),
        Color32::from_rgb(252, 186, 3),
        Color32::from_rgb(50, 168, 82),
        Color32::from_rgb(142, 36, 170),
        Color32::from_rgb(240, 128, 60),
        Color32::from_rgb(66, 189, 168),
        Color32::from_rgb(194, 24, 91),
        Color32::from_rgb(97, 97, 97),
    ]
}

pub fn sequential_color() -> Color32 {
    Color32::from_rgb(70, 130, 200)
}

pub fn parallel_color() -> Color32 {
    Color32::from_rgb(200, 120, 70)
}

#[allow(dead_code)]
pub fn striped_row_bg(ui: &Ui, row_index: usize) -> Option<Color32> {
    if row_index % 2 == 1 {
        Some(if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(255, 255, 255, 8)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 8)
        })
    } else {
        None
    }
}

pub const DATA_BAR_HEIGHT: f32 = 18.0;
pub const DATA_BAR_WIDTH: f32 = 110.0;
pub const DATA_BAR_CORNER_RADIUS: f32 = 3.0;
pub const DATA_BAR_TEXT_PADDING: f32 = 6.0;

pub fn data_bar_bg(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(60, 60, 70, 50)
    } else {
        Color32::from_rgba_unmultiplied(180, 180, 190, 50)
    }
}

pub fn data_bar_stroke(ui: &Ui) -> eframe::egui::Stroke {
    eframe::egui::Stroke::new(
        1.0,
        if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(90, 90, 100, 80)
        } else {
            Color32::from_rgba_unmultiplied(140, 140, 150, 100)
        },
    )
}

#[allow(dead_code)]
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

pub fn data_bar_gradient(ui: &Ui, percentage: f64) -> Color32 {
    let alpha: u8 = if ui.visuals().dark_mode { 200 } else { 180 };
    
    let (r, g, b) = if percentage < 0.5 {
        let t = percentage * 2.0;
        (
            (80.0 + t * 150.0) as u8,
            (200.0 - t * 20.0) as u8,
            (80.0 - t * 20.0) as u8,
        )
    } else {
        let t = (percentage - 0.5) * 2.0;
        (
            230,
            (180.0 - t * 100.0) as u8,
            (60.0 + t * 20.0) as u8,
        )
    };
    
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}
