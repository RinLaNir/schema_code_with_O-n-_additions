use eframe::egui::{self, Color32, RichText, Ui};
use ldpc_toolbox::decoder::factory::DecoderImplementation;
use crate::types::decoder_variants;
use crate::ui::localization::Localization;

pub struct DecoderSelector {
    selected_decoders: Vec<bool>,
    localization: Localization,
}

impl DecoderSelector {
    pub fn new(localization: Localization) -> Self {
        let selected_decoders = vec![false; decoder_variants().len()];
        
        Self {
            selected_decoders,
            localization,
        }
    }
    
    pub fn update(&mut self, localization: &Localization) {
        self.localization = localization.clone();
    }
    
    pub fn get_selected_decoders(&self) -> Vec<DecoderImplementation> {
        let selected: Vec<DecoderImplementation> = decoder_variants()
            .iter()
            .enumerate()
            .filter_map(|(i, &(decoder, _))| {
                if *self.selected_decoders.get(i)? { Some(decoder) } else { None }
            })
            .collect();

        if selected.is_empty() {
            vec![DecoderImplementation::Aminstarf32] // default
        } else {
            selected
        }
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            
            if ui.button(RichText::new(self.localization.get("select_all"))
                .color(Color32::from_rgb(100, 200, 100))).clicked() {
                self.selected_decoders.fill(true);
            }

            if ui.button(RichText::new(self.localization.get("clear_selection"))
                .color(Color32::from_rgb(200, 100, 100))).clicked() {
                self.selected_decoders.fill(false);
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

                        let should_select_all = selected_in_family < indices.len();
                        let button_label = if should_select_all {
                            format!("Select all {}", name)
                        } else {
                            format!("Deselect all {}", name)
                        };

                        if ui.small_button(button_label).clicked() {
                            for &idx in indices {
                                if idx < self.selected_decoders.len() {
                                    self.selected_decoders[idx] = should_select_all;
                                }
                            }
                        }

                        ui.add_space(5.0);

                        let decoder_names: Vec<&str> = decoder_variants().iter().map(|(_, name)| *name).collect();
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            ui.set_min_width(160.0);

                            for &idx in indices {
                                if idx < decoder_names.len() {
                                    let checkbox = ui.checkbox(&mut self.selected_decoders[idx], decoder_names[idx]);
                                    checkbox.on_hover_text(format!("Select decoder {}", decoder_names[idx]));
                                }
                            }
                        });
                    });
                    ui.end_row();
                }
            });
    }
    
}
