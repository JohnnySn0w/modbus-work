//! Configuration profile details and recorded working hardware variants.
use eframe::egui;
use modbus_configurator::{
    bridge::BridgeResult,
    catalog::{Profile, Validation},
};

#[derive(Default)]
pub struct CatalogView {
    pub selected: Option<usize>,
    query: String,
    only_points: bool,
}

/// Show recorded compatibility without treating unknown versions as universal support.
pub fn compatibility(ui: &mut egui::Ui, profile: &Profile) {
    ui.strong("Confirmed working variants");
    if profile.info.confirmed_variants.is_empty() {
        ui.label("No confirmed variants recorded");
    }
    for variant in &profile.info.confirmed_variants {
        ui.label(&variant.model);
        ui.label(format!(
            "Hardware revision: {}",
            variant
                .hardware_revision
                .as_deref()
                .unwrap_or("not recorded")
        ));
        ui.label(format!(
            "Firmware: {}",
            variant.firmware.as_deref().unwrap_or("not recorded")
        ));
        ui.weak(&variant.evidence);
    }
}

impl CatalogView {
    pub fn show(&mut self, ui: &mut egui::Ui, profiles: &[Profile], result: Option<&BridgeResult>) {
        let Some(profile) = self.selected.and_then(|i| profiles.get(i)) else {
            return;
        };
        ui.separator();
        ui.heading(format!(
            "{} {}",
            profile.info.manufacturer, profile.info.model
        ));
        let color = match profile.info.status {
            Validation::Validated => crate::brand::DARK_BLUE,
            Validation::ToTest => crate::brand::DARK_GREY,
        };
        ui.colored_label(
            color,
            if !profile.info.confirmed_variants.is_empty()
                && profile.info.status == Validation::ToTest
            {
                "Model confirmed working · full profile verification not recorded"
            } else {
                profile.info.status.label()
            },
        );
        ui.label(&profile.info.serial);
        crate::brand::collapsing(ui, "Setup and troubleshooting", |ui| {
            for help in &profile.info.help {
                ui.label(format!("• {help}"));
            }
            ui.label(format!("Configuration source: {}", profile.info.source));
        });

        ui.add_space(8.0);
        let matched = result.is_some_and(|r| profile.matches(r));
        if result.is_some() {
            ui.weak(if matched {
                "Matches last Modbus Bridge backup"
            } else {
                "Differs from last Modbus Bridge backup"
            });
        }
        ui.add_space(8.0);
        crate::brand::collapsing(ui, "Register reference", |ui| {
            ui.horizontal(|ui| {
                ui.label("Find");
                ui.text_edit_singleline(&mut self.query);
                ui.checkbox(
                    &mut self.only_points,
                    "Configured Modbus Bridge points only",
                );
            });
            let query = self.query.trim().to_ascii_lowercase();
            egui::ScrollArea::horizontal()
                .id_salt("register-scroll")
                .show(ui, |ui| {
                    egui::Grid::new(("register-table", &profile.info.id))
                        .striped(true)
                        .min_col_width(65.0)
                        .show(ui, |ui| {
                            for title in [
                                "Manual",
                                "Transmitted address",
                                "Register",
                                "Point",
                                "Modbus Bridge data type / word order",
                                "Last reading",
                                "Units",
                            ] {
                                ui.strong(title);
                            }
                            ui.end_row();
                            for (index, register) in profile.registers.iter().enumerate() {
                                let point = profile.register_point(index);
                                if self.only_points && point.is_none() {
                                    continue;
                                }
                                if !query.is_empty()
                                    && !format!(
                                        "{} {} {} {}",
                                        register.name,
                                        register.manual,
                                        register.pdu_display(),
                                        register.units
                                    )
                                    .to_ascii_lowercase()
                                    .contains(&query)
                                {
                                    continue;
                                }
                                ui.label(&register.manual);
                                ui.label(register.pdu_display());
                                ui.label(&register.name).on_hover_ui(|ui| {
                                    for (label, value) in [
                                        ("Interpretation", &register.interpretation),
                                        ("Decode", &register.decode),
                                        ("Documented defaults", &register.defaults),
                                    ] {
                                        if !value.is_empty() {
                                            ui.strong(label);
                                            ui.label(value);
                                        }
                                    }
                                });
                                ui.label(
                                    point
                                        .as_ref()
                                        .map_or("—".into(), |(item, _)| item.to_string()),
                                );
                                ui.label(
                                    point.as_ref().map_or("Reference only", |(_, encoding)| {
                                        encoding.as_str()
                                    }),
                                );
                                let reading = if matched {
                                    point.as_ref().and_then(|(item, _)| {
                                        result.and_then(|r| {
                                            r.readings.iter().find(|r| r.item == *item)
                                        })
                                    })
                                } else {
                                    None
                                };
                                ui.label(reading.map_or("—".into(), |r| r.value.to_string()));
                                ui.label(&register.units);
                                ui.end_row();
                            }
                        });
                });
            ui.label(format!(
                "{} registers · {} configured points",
                profile.registers.len(),
                profile.rows.len()
            ));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_pages_render_headlessly_at_two_window_sizes() {
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        for size in [egui::vec2(960.0, 760.0), egui::vec2(640.0, 480.0)] {
            let context = egui::Context::default();
            let mut view = CatalogView::default();
            for (index, profile) in profiles.iter().enumerate() {
                view.selected = Some(index);
                let result = BridgeResult {
                    dev_eui: None,
                    identity: modbus_configurator::contract::Identity {
                        model: "ENL-MOD-32".into(),
                        firmware: "3.6".into(),
                    },
                    native_tsv: profile.native_tsv.clone(),
                    readings: vec![],
                    successful_reads: None,
                    exceptions: vec![],
                };
                let output = context.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            egui::ScrollArea::vertical()
                                .show(ui, |ui| view.show(ui, &profiles, Some(&result)));
                        });
                    },
                );
                assert!(!output.shapes.is_empty());
            }
        }
    }
}
