//! One device-list editor for individual sensors, networks and imported custom tables.
use super::*;
use modbus_configurator::network::{self, Device};

impl Configurator {
    /// Rebuild the editor only when a different configuration is loaded.
    fn sync_network_editor(&mut self) -> Result<(), String> {
        if self.network_initialized && self.network_table == self.loaded_config {
            return Ok(());
        }
        self.network_devices = if let Some(table) = &self.loaded_config {
            network::editable(table, &self.profiles)?
        } else {
            vec![Device::new(
                self.catalog_view.selected.unwrap_or(0),
                1,
                &self.profiles,
            )]
        };
        if self.network_devices.is_empty() {
            self.network_devices.push(Device::new(0, 1, &self.profiles));
        }
        self.network_table = self.loaded_config.clone();
        self.network_initialized = true;
        Ok(())
    }

    /// Render a 32-device plan and publish a TSV only after complete validation.
    pub(super) fn network_configuration(&mut self, ui: &mut egui::Ui) -> bool {
        if let Err(error) = self.sync_network_editor() {
            brand::attention(ui, "Configuration could not be loaded", &error);
            return false;
        }
        let before = self.network_devices.clone();
        let can_remove = self.network_devices.len() > 1;
        ui.weak("Choose the configured model and slave address for each device. Models are not detected or verified from successful reads. Set each physical sensor to its assigned address; all devices must use the same RS-485 line settings.");
        let count: usize = self.network_devices.iter().map(|d| d.points.len()).sum();
        ui.strong(format!(
            "{count} / 32 register entries · {} / 32 devices",
            self.network_devices.len()
        ));
        ui.weak("A multi-word value counts as one entry. Selections use the reviewed register encodings.");
        let mut remove = None;
        for (index, device) in self.network_devices.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("Device {}", index + 1));
                        let old_profile = device.profile;
                        egui::ComboBox::from_id_salt("model")
                            .selected_text(self.profiles.get(device.profile).map_or("Custom register set", |p| p.info.model.as_str()))
                            .show_ui(ui, |ui| {
                                for (key, profile) in self.profiles.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut device.profile,
                                        key,
                                        &profile.info.model,
                                    );
                                }
                            });
                        if old_profile != device.profile {
                            *device = Device::new(device.profile, device.slave, &self.profiles);
                        }
                        ui.label(egui::RichText::new("Sensor slave address").strong().color(brand::ORANGE));
                        ui.add(egui::DragValue::new(&mut device.slave).range(1..=247));
                        if ui.add_enabled(can_remove, egui::Button::new("Remove device")).clicked() {
                            remove = Some(index);
                        }
                        ui.vertical(|ui| {
                            ui.set_width(270.0);
                            if let Some(profile) = self.profiles.get(device.profile) {
                                crate::catalog_view::compatibility(ui, profile);
                            }
                        });
                    });
                    let profile = self.profiles.get(device.profile);
                    let rows = if device.custom_rows.is_empty() {
                        &profile.expect("catalog selection").rows
                    } else { &device.custom_rows };
                    crate::brand::collapsing_id(
                        ui,
                        "register-entries",
                        format!("Register entries · {} selected", device.points.len()),
                        |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Select all").clicked() {
                                    device.points = rows.keys().copied().collect();
                                }
                                if ui.button("Clear selection").clicked() {
                                    device.points.clear();
                                }
                            });
                            for (point, row) in rows {
                                let fields: Vec<_> = row.split('\t').collect();
                                let mut selected = device.points.contains(point);
                                let label = profile.and_then(|p| p.point_register(*point)).map_or_else(
                                    || format!("Register {} · {} · {} {} · multiplier {}", fields[3], fields[2], fields[4], fields[5], fields[6]),
                                    |register| format!("{} · {} · register {} · {}", register.name, register.units, register.manual, fields[4]));
                                if ui.checkbox(&mut selected, label).changed() {
                                    if selected {
                                        device.points.insert(*point);
                                    } else {
                                        device.points.remove(point);
                                    }
                                }
                            }
                            if let Some(profile) = profile {
                                ui.weak(&profile.info.serial);
                                ui.weak(if !profile.info.confirmed_variants.is_empty() && profile.info.status == modbus_configurator::catalog::Validation::ToTest { "Model confirmed working · full profile verification not recorded" } else { profile.info.status.label() });
                            }
                        },
                    );
                });
            });
        }
        if let Some(index) = remove {
            self.network_devices.remove(index);
        }
        if ui
            .add_enabled(
                self.network_devices.len() < 32,
                egui::Button::new("Add device"),
            )
            .clicked()
        {
            let slave = (1..=247)
                .find(|id| self.network_devices.iter().all(|d| d.slave != *id))
                .unwrap();
            self.network_devices
                .push(Device::new(0, slave, &self.profiles));
        }
        match network::compose(&self.network_devices, &self.profiles) {
            Ok(table) => {
                if before != self.network_devices || self.loaded_config.is_none() {
                    self.loaded_config = Some(table);
                    self.config_source =
                        format!("Network · {} devices", self.network_devices.len());
                    self.remember_selection();
                }
                self.network_table = self.loaded_config.clone();
                self.catalog_view.selected = if self.network_devices.len() == 1 {
                    self.profiles
                        .get(self.network_devices[0].profile)
                        .map(|_| self.network_devices[0].profile)
                } else {
                    None
                };
                self.technician.network_draft = self
                    .loaded_config
                    .clone()
                    .map(|table| (table, self.network_devices.clone()));
                true
            }
            Err(error) => {
                brand::attention(ui, "Network needs attention", &error);
                false
            }
        }
    }
}
