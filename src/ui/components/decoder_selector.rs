use eframe::egui::{self, Color32, RichText, Ui};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use crate::ui::localization::Localization;

pub struct DecoderSelector {
    selected_decoders: Vec<bool>,
    localization: Localization,
}

impl DecoderSelector {
    pub fn new(localization: Localization) -> Self {
        let selected_decoders = vec![false; 36];
        
        Self {
            selected_decoders,
            localization,
        }
    }
    
    pub fn update(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn get_selected_decoders(&self) -> Vec<DecoderImplementation> {
        let all_decoders = self.get_all_decoders();
        
        let selected_decoders: Vec<DecoderImplementation> = self.selected_decoders.iter()
            .enumerate()
            .filter_map(|(i, &selected)| {
                if selected && i < all_decoders.len() {
                    Some(all_decoders[i])
                } else {
                    None
                }
            })
            .collect();
            
        if selected_decoders.is_empty() {
            vec![DecoderImplementation::Aminstarf32] // default
        } else {
            selected_decoders
        }
    }
    
    #[allow(dead_code)]
    pub fn set_selected_decoders(&mut self, decoders: &[DecoderImplementation]) {
        let all_decoders = self.get_all_decoders();
        
        for selected in &mut self.selected_decoders {
            *selected = false;
        }
        
        for &decoder in decoders {
            if let Some(index) = all_decoders.iter().position(|&d| d == decoder) {
                if index < self.selected_decoders.len() {
                    self.selected_decoders[index] = true;
                }
            }
        }
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            
            if ui.button(RichText::new(self.localization.get("select_all"))
                .color(Color32::from_rgb(100, 200, 100))).clicked() {
                for i in 0..self.selected_decoders.len() {
                    self.selected_decoders[i] = true;
                }
            }
            
            if ui.button(RichText::new(self.localization.get("clear_selection"))
                .color(Color32::from_rgb(200, 100, 100))).clicked() {
                for i in 0..self.selected_decoders.len() {
                    self.selected_decoders[i] = false;
                }
            }
        });
        
        ui.separator();
        
        let families = [
            ("Phi Family", vec![0, 1, 24, 25]),
            ("Tanh Family", vec![2, 3, 26, 27]),
            ("Minstarapprox Family", vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 28, 29, 30, 31]),
            ("Aminstar Family", vec![14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 32, 33, 34, 35]),
        ];
        
        ui.spacing_mut().item_spacing.y = 8.0;
        
        egui::Grid::new("decoder_families")
            .num_columns(2)
            .spacing([40.0, 12.0])
            .min_col_width(180.0)
            .show(ui, |ui| {
                for (name, indices) in families.iter() {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.heading(RichText::new(*name).size(18.0));
                        
                        let selected_in_family = indices.iter()
                            .filter(|&&idx| idx < self.selected_decoders.len() && self.selected_decoders[idx])
                            .count();
                            
                        let select_all = selected_in_family < indices.len();
                        let text = if select_all {
                            format!("Select all {}", name)
                        } else {
                            format!("Deselect all {}", name)
                        };
                        
                        if ui.small_button(text).clicked() {
                            for &idx in indices {
                                if idx < self.selected_decoders.len() {
                                    self.selected_decoders[idx] = select_all;
                                }
                            }
                        }
                        
                        ui.add_space(5.0);
                        
                        let all_decoders = self.get_all_decoders_names();
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            ui.set_min_width(160.0);
                            
                            for &idx in indices {
                                if idx < all_decoders.len() {
                                    let checkbox = ui.checkbox(&mut self.selected_decoders[idx], all_decoders[idx]);
                                    checkbox.on_hover_text(format!("Вибрати декодер {}", all_decoders[idx]));
                                }
                            }
                        });
                    });
                    ui.end_row();
                }
            });
    }
    
    fn get_all_decoders(&self) -> Vec<DecoderImplementation> {
        vec![
            DecoderImplementation::Phif64,
            DecoderImplementation::Phif32,
            DecoderImplementation::Tanhf64,
            DecoderImplementation::Tanhf32,
            DecoderImplementation::Minstarapproxf64,
            DecoderImplementation::Minstarapproxf32,
            DecoderImplementation::Minstarapproxi8,
            DecoderImplementation::Minstarapproxi8Jones,
            DecoderImplementation::Minstarapproxi8PartialHardLimit,
            DecoderImplementation::Minstarapproxi8JonesPartialHardLimit,
            DecoderImplementation::Minstarapproxi8Deg1Clip,
            DecoderImplementation::Minstarapproxi8JonesDeg1Clip,
            DecoderImplementation::Minstarapproxi8PartialHardLimitDeg1Clip,
            DecoderImplementation::Minstarapproxi8JonesPartialHardLimitDeg1Clip,
            DecoderImplementation::Aminstarf64,
            DecoderImplementation::Aminstarf32,
            DecoderImplementation::Aminstari8,
            DecoderImplementation::Aminstari8Jones,
            DecoderImplementation::Aminstari8PartialHardLimit,
            DecoderImplementation::Aminstari8JonesPartialHardLimit,
            DecoderImplementation::Aminstari8Deg1Clip,
            DecoderImplementation::Aminstari8JonesDeg1Clip,
            DecoderImplementation::Aminstari8PartialHardLimitDeg1Clip,
            DecoderImplementation::Aminstari8JonesPartialHardLimitDeg1Clip,
            DecoderImplementation::HLPhif64,
            DecoderImplementation::HLPhif32,
            DecoderImplementation::HLTanhf64,
            DecoderImplementation::HLTanhf32,
            DecoderImplementation::HLMinstarapproxf64,
            DecoderImplementation::HLMinstarapproxf32,
            DecoderImplementation::HLMinstarapproxi8,
            DecoderImplementation::HLMinstarapproxi8PartialHardLimit,
            DecoderImplementation::HLAminstarf64,
            DecoderImplementation::HLAminstarf32,
            DecoderImplementation::HLAminstari8,
            DecoderImplementation::HLAminstari8PartialHardLimit,
        ]
    }
    
    fn get_all_decoders_names(&self) -> Vec<&'static str> {
        vec![
            "Phif64",
            "Phif32",
            "Tanhf64",
            "Tanhf32",
            "Minstarapproxf64",
            "Minstarapproxf32",
            "Minstarapproxi8",
            "Minstarapproxi8Jones",
            "Minstarapproxi8PartialHardLimit",
            "Minstarapproxi8JonesPartialHardLimit",
            "Minstarapproxi8Deg1Clip",
            "Minstarapproxi8JonesDeg1Clip",
            "Minstarapproxi8PartialHardLimitDeg1Clip",
            "Minstarapproxi8JonesPartialHardLimitDeg1Clip",
            "Aminstarf64",
            "Aminstarf32",
            "Aminstari8",
            "Aminstari8Jones",
            "Aminstari8PartialHardLimit",
            "Aminstari8JonesPartialHardLimit",
            "Aminstari8Deg1Clip",
            "Aminstari8JonesDeg1Clip",
            "Aminstari8PartialHardLimitDeg1Clip",
            "Aminstari8JonesPartialHardLimitDeg1Clip",
            "HLPhif64",
            "HLPhif32",
            "HLTanhf64",
            "HLTanhf32",
            "HLMinstarapproxf64",
            "HLMinstarapproxf32",
            "HLMinstarapproxi8",
            "HLMinstarapproxi8PartialHardLimit",
            "HLAminstarf64",
            "HLAminstarf32",
            "HLAminstari8",
            "HLAminstari8PartialHardLimit",
        ]
    }
}
