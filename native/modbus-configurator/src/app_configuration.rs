//! Configuration selection, local persistence, and programming preflight.
use super::*;

impl Configurator {
    /// Validate a TSV before replacing the selection; report failures in the UI.
    pub(super) fn load_configuration(&mut self, path: &std::path::Path) -> bool {
        use modbus_configurator::config_file as files;
        match files::load(path) {
            Ok(table) => {
                let saved = self
                    .storage_root
                    .clone()
                    .map_err(std::io::Error::other)
                    .and_then(|folder| {
                        std::fs::create_dir_all(&folder)?;
                        files::save(&folder.join("Selected.tsv"), &table)
                    });
                self.catalog_view.selected = None;
                self.config_source = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into();
                self.loaded_config = Some(table);
                self.file_message = match saved {
                    Ok(()) => "Configuration loaded and remembered.".into(),
                    Err(e) => format!("Loaded for this session; could not remember selection: {e}"),
                };
            }
            Err(e) => {
                self.file_message = format!("Configuration was not loaded: {e}");
                return false;
            }
        }
        true
    }
    /// Render configuration actions and require a verified target before programming.
    pub(super) fn configuration_files(&mut self, ui: &mut egui::Ui) {
        use modbus_configurator::config_file as files;
        ui.heading("Configure E5 bridge");
        ui.weak("Sensor point tables").on_hover_text("TSV programming and backups cover the sensor point table. Serial settings, radio settings and credentials are separate.");
        if self.programming_blocked && self.active.is_none() {
            ui.label("Polling is paused until the console and current point table can be checked.");
            if ui.button("Check recovered console").clicked() && self.select_bridge_route() {
                self.hardware(Operation::BridgeExport);
            }
        }
        let source = self.bridge_source.clone().filter(|old| {
            self.ports
                .iter()
                .any(|p| modbus_configurator::adapter::same_route(old, p))
        });
        if let Some(p) = source.as_ref() {
            ui.label(format!(
                "Target: {} · USB {}",
                p.port,
                p.serial_number.as_deref().unwrap_or("serial unavailable")
            ));
        } else {
            egui::Frame::default().fill(crate::brand::ORANGE.gamma_multiply(0.15))
                .stroke(egui::Stroke::new(1.0, crate::brand::ORANGE)).inner_margin(12.0).show(ui, |ui| {
                    ui.set_width((ui.available_width() - 24.0).max(0.0));
                    ui.strong("E5 bridge unavailable — programming and backup disabled");
                    ui.label("Connect and switch on the E5 bridge to verify the target. You can still review or save configurations.");
                });
        }
        ui.add_space(12.0);
        ui.strong("1. Choose a point table");
        ui.horizontal_wrapped(|ui| {
            ui.label("Sensor type");
            for index in 0..self.profiles.len() {
                if ui
                    .selectable_label(
                        self.catalog_view.selected == Some(index),
                        &self.profiles[index].info.model,
                    )
                    .clicked()
                {
                    self.catalog_view.selected = Some(index);
                    self.loaded_config = Some(self.profiles[index].native_tsv.clone());
                    self.config_source = self.profiles[index].info.model.clone();
                    self.remember_selection();
                }
            }
            if ui.button("Open TSV file…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("E5 bridge point table", &["tsv"])
                    .pick_file()
            {
                self.load_configuration(&path);
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    source.is_some() && !self.foreground_busy(),
                    egui::Button::new("Back up now"),
                )
                .clicked()
            {
                self.handle_action(technician_view::Action::BackupBridge);
            }
            ui.menu_button("Backups", |ui| {
                if let Some(port) = &source {
                    match self
                        .storage_root
                        .clone()
                        .map_err(std::io::Error::other)
                        .and_then(|root| files::history(&root, port))
                    {
                        Ok(paths) if paths.is_empty() => {
                            ui.label("No saved point tables for this E5 bridge.");
                        }
                        Ok(paths) => {
                            for (index, path) in paths.into_iter().enumerate() {
                                let details = files::load(&path)
                                    .map(|table| {
                                        format!("{} points", table.lines().skip(1).count())
                                    })
                                    .unwrap_or_else(|_| "Unreadable table".into());
                                let label = if index == 0 {
                                    format!("Latest backup · {details}")
                                } else {
                                    format!("Earlier backup {index} · {details}")
                                };
                                if ui
                                    .button(&label)
                                    .on_hover_text(path.display().to_string())
                                    .clicked()
                                {
                                    if self.load_configuration(&path) {
                                        self.config_source =
                                            label.split(" · ").next().unwrap_or("Backup").into();
                                    }
                                    ui.close();
                                }
                            }
                        }
                        Err(e) => {
                            ui.label(format!("Cannot read backups: {e}"));
                        }
                    }
                } else {
                    ui.label("Connect the target E5 bridge to view its backups.");
                }
            });
        });
        if let Some(table) = self.loaded_config.clone() {
            ui.separator();
            ui.strong("2. Review changes");
            ui.label(format!(
                "{} · {} points",
                self.config_source,
                table.lines().skip(1).count()
            ));
            let ids: std::collections::BTreeSet<_> = table
                .lines()
                .skip(1)
                .filter_map(|r| r.split('\t').nth(1))
                .collect();
            ui.label(format!(
                "Slave IDs: {}",
                ids.into_iter().collect::<Vec<_>>().join(", ")
            ));
            if let Some(index) = self.catalog_view.selected {
                ui.collapsing("Connection requirements", |ui| {
                    ui.label(&self.profiles[index].info.serial);
                    ui.label(self.profiles[index].info.status.label());
                });
            } else {
                ui.weak("TSV files do not specify baud rate or parity; confirm the instrument and E5 bridge serial settings agree.");
            }
            if let Some(current) = &self.result {
                let rows = |s: &str| -> std::collections::BTreeMap<u8, String> {
                    s.lines()
                        .skip(1)
                        .filter_map(|r| {
                            r.split_once('\t')
                                .and_then(|(k, _)| k.parse().ok().map(|k| (k, r.into())))
                        })
                        .collect()
                };
                let old = rows(&current.native_tsv);
                let new = rows(&table);
                let removed: Vec<_> = old
                    .keys()
                    .filter(|k| !new.contains_key(*k))
                    .cloned()
                    .collect();
                let changed = new.iter().filter(|(k, v)| old.get(*k) != Some(*v)).count();
                if changed == 0 && removed.is_empty() {
                    ui.label("Matches the E5 bridge’s current point table.");
                } else {
                    ui.label(format!(
                        "{changed} added or changed points; {} removed points",
                        removed.len()
                    ));
                }
                if !removed.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(165, 65, 40),
                        format!(
                            "Remove points: {}",
                            removed
                                .iter()
                                .map(u8::to_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                }
                if changed > 0 || !removed.is_empty() {
                    ui.collapsing("Review point changes", |ui| {
                        for key in old
                            .keys()
                            .chain(new.keys())
                            .collect::<std::collections::BTreeSet<_>>()
                        {
                            if old.get(key) != new.get(key) {
                                ui.strong(format!("Point {key}"));
                                ui.monospace(format!(
                                    "Current: {}",
                                    old.get(key).map_or("Absent", String::as_str)
                                ));
                                ui.monospace(format!(
                                    "Selected: {}",
                                    new.get(key).map_or("Remove", String::as_str)
                                ));
                            }
                        }
                    });
                }
            }
            ui.add_space(12.0);
            ui.strong("3. Apply or save");
            ui.horizontal_wrapped(|ui| {
                let ready = source.is_some()
                    && self.result.is_some()
                    && !self.foreground_busy()
                    && !self.programming_blocked;
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(
                            egui::RichText::new("Program E5 bridge").color(egui::Color32::WHITE),
                        )
                        .fill(brand::DARK_BLUE),
                    )
                    .clicked()
                {
                    self.selected = source.as_ref().unwrap().port.clone();
                    self.hardware(Operation::BridgeProgram {
                        target: table.clone(),
                        reviewed: self.result.as_ref().unwrap().native_tsv.clone(),
                    });
                }
                if ui.button("Save TSV file…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("E5 bridge point table", &["tsv"])
                        .set_file_name("E5 bridge-configuration.tsv")
                        .save_file()
                {
                    self.file_message = match files::save(&path, &table) {
                        Ok(()) => "TSV file saved.".into(),
                        Err(e) => format!("Save failed: {e}"),
                    };
                }
                if ui.button("Copy TSV").clicked() {
                    ui.ctx().copy_text(table.clone());
                }
            });
            ui.weak("Program E5 bridge saves a backup, replaces the point table, verifies an export, then resumes live readings.");
            ui.collapsing("Selected TSV", |ui| {
                ui.monospace(&table);
            });
            if self.catalog_view.selected.is_some() {
                ui.collapsing("Sensor reference", |ui| {
                    self.catalog_view
                        .show(ui, &self.profiles, self.result.as_ref())
                });
            }
        }
    }
    /// Persist the selected configuration without discarding it on a storage failure.
    pub(super) fn remember_selection(&mut self) {
        if let Some(table) = &self.loaded_config {
            self.file_message = match self
                .storage_root
                .clone()
                .map_err(std::io::Error::other)
                .and_then(|folder| {
                    std::fs::create_dir_all(&folder)?;
                    modbus_configurator::config_file::save(&folder.join("Selected.tsv"), table)
                }) {
                Ok(()) => String::new(),
                Err(e) => format!("Could not remember selection: {e}"),
            };
        }
    }
}
