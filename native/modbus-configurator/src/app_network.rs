//! Single-device mode stays compact; network mode edits each sensor independently.
use super::*;
use modbus_configurator::network::{self, Device};

impl Configurator {
    /// Preserve the single-device draft while editing a combined network table.
    pub(super) fn configuration_mode(&mut self, ui: &mut egui::Ui) {
        let previous = self.multi_device;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.multi_device, false, "Single device");
            ui.selectable_value(&mut self.multi_device, true, "Multi-device");
        });
        if previous == self.multi_device {
            return;
        }
        if self.multi_device {
            self.single_selection = Some((
                self.loaded_config.clone(),
                self.config_source.clone(),
                self.catalog_view.selected,
            ));
            if self.network_devices.is_empty() {
                let recognized = self
                    .loaded_config
                    .as_deref()
                    .and_then(|table| network::recognize(table, &self.profiles));
                self.network_unrecognized = self.loaded_config.is_some() && recognized.is_none();
                self.network_devices = recognized.unwrap_or_default();
            }
        } else if let Some((table, source, selected)) = self.single_selection.take() {
            self.loaded_config = table;
            self.config_source = source;
            self.catalog_view.selected = selected;
            self.remember_selection();
        }
    }

    /// Render a four-device plan and publish a TSV only after complete validation.
    pub(super) fn network_configuration(&mut self, ui: &mut egui::Ui) -> bool {
        if self.network_unrecognized {
            brand::attention(
                ui,
                "Custom table",
                "This table cannot be mapped unambiguously to the sensor catalog. Use Single device mode to keep working with the original configuration table, or start a new network.",
            );
            if ui.button("Start new network").clicked() {
                self.network_unrecognized = false;
                self.network_devices.clear();
            }
            return false;
        }
        ui.weak("Choose the configured model and slave address for each device. Models are not detected or verified from successful reads. Set each physical sensor to its assigned address; all devices must use the same RS-485 line settings.");
        let count: usize = self.network_devices.iter().map(|d| d.points.len()).sum();
        ui.strong(format!(
            "{count} / 32 register entries · {} / 4 devices",
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
                            .selected_text(&self.profiles[device.profile].info.model)
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
                        ui.label("Slave address");
                        ui.add(egui::DragValue::new(&mut device.slave).range(1..=247));
                        if ui.button("Remove device").clicked() {
                            remove = Some(index);
                        }
                    });
                    let profile = &self.profiles[device.profile];
                    crate::brand::collapsing(
                        ui,
                        format!("Register entries · {} selected", device.points.len()),
                        |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Select all").clicked() {
                                    device.points = profile.rows.keys().copied().collect();
                                }
                                if ui.button("Clear selection").clicked() {
                                    device.points.clear();
                                }
                            });
                            for (point, row) in &profile.rows {
                                let register = profile
                                    .point_register(*point)
                                    .expect("validated catalog point");
                                let fields: Vec<_> = row.split('\t').collect();
                                let mut selected = device.points.contains(point);
                                let label = format!(
                                    "{} · {} · register {} · {}",
                                    register.name, register.units, register.manual, fields[4]
                                );
                                if ui.checkbox(&mut selected, label).changed() {
                                    if selected {
                                        device.points.insert(*point);
                                    } else {
                                        device.points.remove(point);
                                    }
                                }
                            }
                            ui.weak(&profile.info.serial);
                            ui.weak(profile.info.status.label());
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
                self.network_devices.len() < 4,
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
                self.technician.network_draft = Some((table.clone(), self.network_devices.clone()));
                self.catalog_view.selected = None;
                self.config_source = format!("Network · {} devices", self.network_devices.len());
                if self.loaded_config.as_ref() != Some(&table) {
                    self.loaded_config = Some(table);
                    self.remember_selection();
                }
                true
            }
            Err(error) => {
                brand::attention(ui, "Network needs attention", &error);
                false
            }
        }
    }
}
