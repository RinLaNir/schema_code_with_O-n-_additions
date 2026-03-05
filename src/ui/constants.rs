use eframe::egui::{Color32, Stroke, TextStyle, Ui};

// --- Text scaling ---

const HEADING_SCALE: f32 = 1.4;
pub const SUBHEADING_SCALE: f32 = 1.2;
const SMALL_SCALE: f32 = 0.85;

pub fn scaled_size(ui: &Ui, scale: f32) -> f32 {
    ui.text_style_height(&TextStyle::Body) * scale
}

pub fn heading_size(ui: &Ui) -> f32 {
    scaled_size(ui, HEADING_SCALE)
}

pub fn small_size(ui: &Ui) -> f32 {
    scaled_size(ui, SMALL_SCALE)
}

// --- Layout spacing & dimensions ---

pub const SECTION_SPACING: f32 = 20.0;
pub const ITEM_SPACING: f32 = 10.0;
pub const SMALL_SPACING: f32 = 5.0;

pub const TABLE_ROW_HEIGHT: f32 = 24.0;
pub const MIN_COLUMN_WIDTH: f32 = 60.0;

pub const SIDEBAR_BREAKPOINT: f32 = 900.0;
pub const SIDEBAR_WIDTH: f32 = 180.0;
pub const MAX_CONTENT_WIDTH: f32 = 1200.0;

// --- Theme-aware colors ---

fn themed_color(ui: &Ui, dark: Color32, light: Color32) -> Color32 {
    if ui.visuals().dark_mode { dark } else { light }
}

fn success_color(ui: &Ui) -> Color32 {
    themed_color(ui, Color32::from_rgb(80, 200, 80), Color32::from_rgb(20, 140, 20))
}

pub fn warning_color(ui: &Ui) -> Color32 {
    themed_color(ui, Color32::from_rgb(230, 200, 60), Color32::from_rgb(180, 140, 0))
}

pub fn error_color(ui: &Ui) -> Color32 {
    themed_color(ui, Color32::from_rgb(230, 80, 80), Color32::from_rgb(180, 40, 40))
}

fn secondary_color(ui: &Ui) -> Color32 {
    themed_color(ui, Color32::from_rgb(230, 160, 100), Color32::from_rgb(180, 100, 40))
}

// --- Threshold-based color selectors ---

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

// --- Implementation-specific colors ---

pub fn sequential_color() -> Color32 {
    Color32::from_rgb(70, 130, 200)
}

pub fn parallel_color() -> Color32 {
    Color32::from_rgb(200, 120, 70)
}

// --- Data bar styling ---

pub const DATA_BAR_HEIGHT: f32 = 18.0;
pub const DATA_BAR_WIDTH: f32 = 110.0;
pub const DATA_BAR_CORNER_RADIUS: f32 = 3.0;
pub const DATA_BAR_TEXT_PADDING: f32 = 6.0;

pub fn data_bar_bg(ui: &Ui) -> Color32 {
    themed_color(
        ui,
        Color32::from_rgba_unmultiplied(60, 60, 70, 50),
        Color32::from_rgba_unmultiplied(180, 180, 190, 50),
    )
}

pub fn data_bar_stroke(ui: &Ui) -> Stroke {
    Stroke::new(
        1.0,
        themed_color(
            ui,
            Color32::from_rgba_unmultiplied(90, 90, 100, 80),
            Color32::from_rgba_unmultiplied(140, 140, 150, 100),
        ),
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
