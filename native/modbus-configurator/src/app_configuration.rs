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
                if self.multi_device {
                    let recognized = self.loaded_config.as_deref().and_then(|table| {
                        modbus_configurator::network::recognize(table, &self.profiles)
                    });
                    self.network_unrecognized = recognized.is_none();
                    self.network_devices = recognized.unwrap_or_default();
                }
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
        ui.weak("Sensor point tables").on_hover_text("Point-table programming and backups cover the sensor point table. Serial settings, radio settings and credentials are separate.");
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
            brand::attention(
                ui,
                "E5 bridge unavailable",
                "Connect and switch on the E5 bridge to enable programming and backup. Configuration review and saving remain available.",
            );
        }
        ui.add_space(12.0);
        self.line_configuration(ui, source.as_ref());
        ui.add_space(12.0);
        ui.strong("1. Choose a point table");
        self.configuration_mode(ui);
        let network_valid = if self.multi_device {
            self.network_configuration(ui)
        } else {
            true
        };
        if !self.multi_device {
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
                if ui.button("Open configuration file…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("E5 bridge point table", &["tsv"])
                        .pick_file()
                {
                    self.load_configuration(&path);
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Backup name");
            ui.add(
                egui::TextEdit::singleline(&mut self.backup_name)
                    .hint_text("Optional, e.g. Before sensor change")
                    .desired_width(250.0)
                    .char_limit(64),
            );
            if ui
                .add_enabled(
                    source.is_some() && !self.foreground_busy(),
                    egui::Button::new("Back up E5 bridge"),
                )
                .clicked()
            {
                let name = self.backup_name.trim().to_owned();
                if name.is_empty() {
                    self.handle_action(technician_view::Action::BackupBridge);
                } else if let Err(error) = files::validate_backup_name(&name) {
                    self.file_message = error.to_string();
                } else if self.select_bridge_route() {
                    self.hardware(Operation::BridgeNamedBackup { name });
                }
            }
            ui.menu_button("Load backup", |ui| {
                ui.weak("Load a point table for review, then use Program E5 bridge to restore it.");
                if ui.button("Choose backup file…").clicked() {
                    let mut dialog =
                        rfd::FileDialog::new().add_filter("E5 bridge point table", &["tsv"]);
                    if let Ok(root) = &self.storage_root {
                        dialog = dialog.set_directory(root.join("Backups"));
                    }
                    if let Some(path) = dialog.pick_file() {
                        self.load_configuration(&path);
                    }
                    ui.close();
                }
                ui.separator();
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
                                let label = if let Some(name) = files::backup_name(&path) {
                                    let version = if index == 0 {
                                        "Latest".to_owned()
                                    } else {
                                        format!("Earlier backup {index}")
                                    };
                                    format!("{name} · {details} · {version}")
                                } else if index == 0 {
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
        if network_valid && let Some(mut table) = self.loaded_config.clone() {
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
                .filter_map(|r| r.split('\t').nth(1)?.parse::<u8>().ok())
                .collect();
            if !self.multi_device {
                for from in ids.iter().copied() {
                    ui.horizontal(|ui| {
                        ui.label("Sensor slave address");
                        let mut to = from;
                        if ui
                            .add(egui::DragValue::new(&mut to).range(1..=247))
                            .changed()
                        {
                            if to != from && ids.contains(&to) {
                                self.file_message =
                                    "Each device needs a different slave address.".into();
                                return;
                            }
                            match files::remap_slave(&table, from, to) {
                                Ok(updated) => {
                                    table = updated;
                                    self.loaded_config = Some(table.clone());
                                    self.remember_selection();
                                }
                                Err(error) => self.file_message = error.to_string(),
                            }
                        }
                    });
                }
                ui.weak(
                "Used by this point table. Set the physical sensor to the same address separately.",
            );
            }
            if let Some(index) = self.catalog_view.selected {
                crate::brand::collapsing(ui, "Connection requirements", |ui| {
                    ui.label(&self.profiles[index].info.serial);
                    ui.label(self.profiles[index].info.status.label());
                });
            } else {
                ui.weak("Point-table files do not specify baud rate or parity; confirm the instrument and E5 bridge serial settings agree.");
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
                    brand::attention(
                        ui,
                        "Points will be removed",
                        &format!(
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
                    crate::brand::collapsing(ui, "Review point changes", |ui| {
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
            ui.strong("3. Program E5 bridge");
            ui.label("A backup is saved before programming. The new point table is verified before readings resume.");
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
            });
            ui.add_space(16.0);
            crate::brand::collapsing(ui, "File tools", |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Save configuration file…").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("E5 bridge point table", &["tsv"])
                            .set_file_name("E5 bridge-configuration.tsv")
                            .save_file()
                    {
                        self.file_message = match files::save(&path, &table) {
                            Ok(()) => "Configuration file saved.".into(),
                            Err(e) => format!("Save failed: {e}"),
                        };
                    }
                    if ui.button("Copy configuration table").clicked() {
                        ui.ctx().copy_text(table.clone());
                    }
                });
            });
            crate::brand::collapsing(ui, "Selected configuration table", |ui| {
                ui.monospace(&table);
            });
            if self.catalog_view.selected.is_some() {
                crate::brand::collapsing(ui, "Sensor reference", |ui| {
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
