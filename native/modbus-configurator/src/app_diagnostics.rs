//! Diagnostic sections and evidence export controls.
use super::*;

impl Configurator {
    /// Render diagnostics without initiating implicit hardware actions.
    pub(super) fn diagnostics_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Diagnostics");
        self.communication_timeline(ui);
        self.diagnostic_assessment(ui);
        self.diagnostic_interfaces(ui);
        self.diagnostic_console(ui);
        ui.separator();
        self.diagnostic_activity(ui);
        crate::brand::collapsing(ui, "Offline diagnostics", |ui| {
            ui.weak(
                "Checks recorded console responses without communicating with connected equipment.",
            );
            if ui
                .add_enabled(
                    self.active.is_none(),
                    egui::Button::new("Run offline prompt replay"),
                )
                .clicked()
            {
                self.replay_log.clear();
                let fixture: Command =
                    serde_json::from_str(include_str!("../tests/fixtures/navigation.json"))
                        .expect("embedded fixture");
                self.request(fixture.operation, false);
            }
            ui.weak("Replay results appear in Activity log.");
        });
        ui.label("Point-table programming and backups are in Configuration. Firmware updates are not implemented.");
    }
    /// Show bounded serial metadata without changing polling status.
    fn communication_timeline(&mut self, ui: &mut egui::Ui) {
        crate::brand::collapsing(ui, "Live communication timeline", |ui| {
            ui.label("Recent serial waits, byte counts, parser states, and recovery attempts. Included in Copy log, Save log, and Export diagnostics. Configure time limits in Settings.");
            egui::ScrollArea::vertical()
                .id_salt("communication-timeline")
                .max_height(220.0)
                .show(ui, |ui| {
                    for line in self.communication_log.iter().rev() {
                        ui.label(line);
                    }
                });
            if self.communication_log.is_empty() {
                ui.weak("No communication events yet.");
            }
            if let Some(request_id) = self.active.filter(|_| !self.programming)
                && ui.button("Cancel current read").clicked()
            {
                self.auto_paused = true;
                self.request(Operation::Cancel { request_id }, false);
                self.status = "Cancelling the read; automatic polling paused.".into();
            }
        });
    }

    /// Show the last response assessment with its source and timestamp.
    fn diagnostic_assessment(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Communication and data checks");
            ui.label("Read-only assessment of the last response. Plausible values do not prove correct indexing, word order, device identity, or sensor accuracy. Zero is not automatically an error.");
            if let Some((port, at, findings)) = &self.diagnostic_report {
                ui.strong(format!("{} items to review · {port} · {at}", findings.iter().filter(|f| f.warning).count()));
                ui.weak("Snapshot from that request; not a live connection indicator. Retained readings are excluded from these checks.");
                for finding in findings {
                    ui.separator();
                    if finding.warning { ui.colored_label(crate::brand::ORANGE, format!("Review · {}", finding.subject)); }
                    else { ui.strong(&finding.subject); }
                    ui.label(&finding.detail);
                }
            } else { ui.label("No response assessed yet. Select an interface and request readings; no extra bus probes are performed by these checks."); }
        });
    }

    /// List available routes and allow an explicit manual selection.
    fn diagnostic_interfaces(&mut self, ui: &mut egui::Ui) {
        ui.strong("USB interfaces");
        ui.label("Choose the USB interface to use. Modbus Bridge console actions are available for a verified Modbus Bridge.");
        if self.ports.is_empty() {
            ui.label("No serial interfaces reported.");
        }
        for port in &self.ports {
            ui.group(|ui| {
                ui.strong(format!("{} — {}", port.port, port.description));
                if let (Some(vid), Some(pid)) = (port.usb_vid, port.usb_pid) {
                    ui.label(format!("USB {vid:04X}:{pid:04X}"));
                }
                ui.label(format!(
                    "USB serial: {}",
                    port.serial_number.as_deref().unwrap_or("unavailable")
                ));
                let candidate = modbus_configurator::adapter::is_bridge(port)
                    || modbus_configurator::adapter::is_adapter(port);
                ui.label(if port.busy {
                    "Console operation in progress"
                } else {
                    "Product identity checked when an action starts"
                });
                if ui
                    .add_enabled(
                        candidate && self.active.is_none(),
                        egui::Button::selectable(self.selected == port.port, "Use this interface"),
                    )
                    .clicked()
                {
                    self.selected = port.port.clone();
                    self.preferred_route = Some(port.clone());
                    self.preferences.automatic_polling = false;
                    self.status = "Automatic polling is off. Manual interface selected.".into();
                }
            });
        }
    }

    /// Present verified console readings and read-only actions.
    fn diagnostic_console(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        if !self.preferences.automatic_polling {
            ui.weak(
                "Automatic polling is off. Enable it in Settings to resume automatic readings.",
            );
        }
        ui.heading(format!(
            "Modbus Bridge console{}",
            if self.selected.is_empty() {
                String::new()
            } else {
                format!(" — {}", self.selected)
            }
        ));
        ui.label("Read now asks the Modbus Bridge to poll its configured instruments. Backups are available in Configuration.");
        ui.horizontal(|ui| {
            let ready = !self.selected.is_empty()
                && !self.foreground_busy()
                && self
                    .ports
                    .iter()
                    .any(|p| p.port == self.selected && modbus_configurator::adapter::is_bridge(p));
            if ui
                .add_enabled(ready, egui::Button::new("Read now"))
                .clicked()
            {
                self.hardware(Operation::BridgeReadAll);
            }
            if ui
                .add_enabled(ready, egui::Button::new("Read configuration only"))
                .clicked()
            {
                self.hardware(Operation::BridgeExport);
            }
            if ui
                .add_enabled(ready, egui::Button::new("Release console"))
                .clicked()
            {
                self.hardware(Operation::ClosePort);
            }
        });
        if let Some(result) = &self.result {
            ui.label(format!(
                "Verified console: {} firmware {}",
                result.identity.model, result.identity.firmware
            ));
            for exception in &result.exceptions {
                ui.label(format!(
                    "Point {}: {} (exception {})",
                    exception.item, exception.message, exception.code
                ));
            }
            if !result.readings.is_empty() {
                let profile = self
                    .profiles
                    .iter()
                    .find(|profile| profile.contains_points(result));
                ui.label("Last completed Read All:");
                if let Some(profile) = profile {
                    ui.label(format!("Names and units from the matching {} table; attached instrument identity is not established by this match.", profile.info.model));
                } else {
                    ui.label("Register names are unavailable for this point table.");
                }
                egui::Grid::new("readings").striped(true).show(ui, |ui| {
                    ui.strong("Point");
                    ui.strong("Register");
                    ui.strong("Value");
                    ui.strong("Units");
                    ui.end_row();
                    for reading in &result.readings {
                        let register = profile.and_then(|p| p.point_register(reading.item));
                        ui.label(reading.item.to_string());
                        ui.label(register.map_or("Unmapped", |r| r.name.as_str()));
                        ui.label(reading.value.to_string());
                        ui.label(register.map_or("—", |r| r.units.as_str()));
                        ui.end_row();
                    }
                });
            }
            crate::brand::collapsing(ui, "Native point table", |ui| {
                ui.monospace(&result.native_tsv);
            });
        }
    }

    /// Copy, export, and display the retained diagnostic evidence.
    fn diagnostic_activity(&mut self, ui: &mut egui::Ui) {
        crate::brand::collapsing(ui, "Activity log", |ui| {
            ui.weak("Timestamped actions, results, and connection changes. Automatic polling appears only when it fails.");
            ui.horizontal(|ui| {
                if ui.button("Export diagnostics…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Text log", &["txt"])
                        .set_file_name("polygon-diagnostics.txt")
                        .save_file()
                {
                    self.status = match crate::diagnostic_log::export(&path) {
                        Ok(()) => "Diagnostic logs exported.".into(),
                        Err(error) => format!("Could not export diagnostics: {error}"),
                    };
                }
                if ui
                    .add_enabled(
                        !self.replay_log.is_empty()
                            || !self.communication_log.is_empty()
                            || self.diagnostic_report.is_some(),
                        egui::Button::new("Copy log"),
                    )
                    .clicked()
                {
                    let text = self.log_text();
                    self.status = match arboard::Clipboard::new()
                        .and_then(|mut clipboard| clipboard.set_text(text))
                    {
                        Ok(()) => "Activity log copied to clipboard.".into(),
                        Err(error) => {
                            format!("Could not copy activity log: {error}. Use Save log instead.")
                        }
                    };
                }
                if ui
                    .add_enabled(
                        !self.replay_log.is_empty()
                            || !self.communication_log.is_empty()
                            || self.diagnostic_report.is_some(),
                        egui::Button::new("Save log…"),
                    )
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Text log", &["txt"])
                        .set_file_name("activity-log.txt")
                        .save_file()
                {
                    self.status = match std::fs::write(path, self.log_text()) {
                        Ok(()) => "Activity log saved.".into(),
                        Err(error) => format!("Could not save activity log: {error}"),
                    };
                }
                if ui
                    .add_enabled(!self.replay_log.is_empty(), egui::Button::new("Clear log"))
                    .clicked()
                {
                    self.replay_log.clear();
                }
            });
            if self.replay_log.is_empty() {
                ui.label("No activity recorded yet.");
            } else {
                let mut rows: Vec<(&str, usize)> = Vec::new();
                for message in &self.replay_log {
                    if let Some((last, count)) = rows.last_mut()
                        && *last == message
                    {
                        *count += 1;
                    } else {
                        rows.push((message.as_str(), 1));
                    }
                }
                ui.weak("Newest first · last 64 events");
                egui::ScrollArea::vertical()
                    .id_salt("diagnostic-activity")
                    .max_height(280.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (message, count) in rows.into_iter().rev() {
                            ui.horizontal_top(|ui| {
                                ui.add_sized(
                                    [45.0, 20.0],
                                    egui::Label::new(if count > 1 {
                                        format!("{count} times")
                                    } else {
                                        String::new()
                                    }),
                                );
                                ui.add(egui::Label::new(message).wrap());
                            });
                            ui.separator();
                        }
                    });
            }
        });
    }
}
