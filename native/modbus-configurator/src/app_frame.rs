//! Window layout, status display, and automatic polling scheduling.
use super::*;

impl Configurator {
    /// Render the fixed status area without shifting page contents.
    pub(super) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("operation-status")
            .exact_height(64.0)
            .resizable(false)
            .frame(
                egui::Frame::default()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(24, 10)),
            )
            .show(ctx, |ui| {
                let busy = self.active.is_some() && !self.auto_request;
                let label = if let Some((operation, _)) = &self.queued {
                    Self::queued_label(operation).into()
                } else if !self.status.is_empty() {
                    self.status.clone()
                } else if busy {
                    "Reading device…".into()
                } else if self.offline {
                    "Offline · hardware access disabled".into()
                } else if !self.preferences.automatic_polling {
                    "Automatic polling off".into()
                } else if self.auto_paused {
                    "Polling paused".into()
                } else {
                    "Ready · automatic readings enabled".into()
                };
                ui.horizontal(|ui| {
                    // Reserve the same action width in every state. Text never wraps into the track.
                    let width = (ui.available_width() - 120.0).max(0.0);
                    ui.add_sized([width, 24.0], egui::Label::new(&label).truncate())
                        .on_hover_text(&label);
                    if self.queued.is_some() {
                        if ui.button("Cancel queued").clicked() {
                            self.queued = None;
                        }
                    } else if let Some(request_id) = self.active.filter(|_| !self.auto_request) {
                        if ui.button("Cancel").clicked() {
                            self.auto_paused = true;
                            self.request(Operation::Cancel { request_id }, false);
                            self.status =
                                "Cancelling; waiting for the console session to close…".into();
                        }
                    } else if !self.status.is_empty() && ui.button("Dismiss").clicked() {
                        self.status.clear();
                    }
                });
                ui.add_space(4.0);
                if busy {
                    let progress = transfer_progress(&self.status);
                    ui.add(
                        egui::ProgressBar::new(progress.unwrap_or(0.0))
                            .animate(progress.is_none())
                            .desired_width(ui.available_width())
                            .desired_height(6.0),
                    );
                } else {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 6.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 3.0, brand::LOGO_GREY);
                }
            });
    }
    /// Process events, schedule background work, and render the selected page.
    pub(super) fn frame(&mut self, ctx: &egui::Context) {
        self.apply_theme(ctx);
        while let Ok(event) = self.service.events.try_recv() {
            self.handle_event(event);
        }
        self.start_queued();
        if self.scan_pending.is_none() && self.last_scan.elapsed() >= Duration::from_millis(1500) {
            self.last_scan = Instant::now();
            self.scan_pending = self.request(Operation::Inventory, false);
        }
        if self.active.is_none()
            && self.preferences.automatic_polling
            && !self.auto_paused
            && !self.programming_blocked
            && Instant::now().saturating_duration_since(self.last_fetch) >= Duration::from_secs(5)
        {
            use modbus_configurator::adapter::{choose_route, is_adapter, is_bridge};
            let candidate = if self.ports.iter().any(is_bridge) {
                is_bridge as fn(&PortInfo) -> bool
            } else {
                is_adapter as fn(&PortInfo) -> bool
            };
            if let Some(route) = choose_route(
                &self.ports,
                self.preferred_route.as_ref().filter(|p| candidate(p)),
                candidate,
            )
            .cloned()
            {
                self.selected = route.port.clone();
                self.preferred_route = Some(route.clone());
                let previous_status = self.status.clone();
                self.hardware_with_activity(
                    if is_bridge(&route) {
                        Operation::BridgeReadAll
                    } else {
                        Operation::AdapterRead
                    },
                    true,
                );
                self.auto_request = true;
                self.status = previous_status;
            } else if self.ports.iter().filter(|p| candidate(p)).count() > 1 {
                self.status =
                    "Multiple USB devices found. Choose an interface in Diagnostics.".into();
            }
        }

        if let Some(capture) = &mut self.capture {
            let ready = self.active.is_none()
                && (self.offline
                    || self
                        .result
                        .as_ref()
                        .is_some_and(|r| r.successful_reads.is_some())
                    || self.adapter.as_ref().is_some_and(|r| !r.values.is_empty()));
            if !capture.started && ready {
                self.capture_restore = Some((
                    self.loaded_config.clone(),
                    self.config_source.clone(),
                    self.catalog_view.selected,
                    self.auto_paused,
                ));
            }
            if let Some((page, profile)) = capture.update(ctx, ready) {
                self.technician.page = page;
                self.technician.reference_context = capture.reference_context;
                self.library_open = false;
                if let Some(index) = profile {
                    self.catalog_view.selected = Some(index);
                    self.loaded_config = Some(self.profiles[index].native_tsv.clone());
                    self.config_source = self.profiles[index].info.model.clone();
                }
            }
            if capture.started {
                self.auto_paused = !capture.done;
            }
        }
        if self.capture.as_ref().is_some_and(|c| c.done) {
            if self.offline && std::env::args_os().any(|arg| arg == "--exit-after-capture") {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            self.capture = None;
            if let Some((table, source, selected, paused)) = self.capture_restore.take() {
                self.loaded_config = table;
                self.config_source = source;
                self.catalog_view.selected = selected;
                self.auto_paused = paused;
            }
        }
        self.technician.bridge_connected = self.bridge_source.as_ref().is_some_and(|source| {
            self.ports
                .iter()
                .any(|p| modbus_configurator::adapter::same_route(p, source))
        });
        self.status_bar(ctx);
        egui::CentralPanel::default().frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin { left: 24, right: 6, top: 24, bottom: 24 })).show(ctx, |ui| {
            egui::Frame::default().inner_margin(egui::Margin { right: 18, ..Default::default() }).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Polygon Device Configurator");
                    ui.add_space(20.0);
                    let reference_page = self.technician.page == technician_view::Page::References
                        || self.technician.page == technician_view::Page::Radio
                        || (self.technician.reference_context && matches!(self.technician.page, technician_view::Page::Detail(_) | technician_view::Page::Registers(_)));
                    if ui.selectable_label(!reference_page && matches!(self.technician.page, technician_view::Page::Overview | technician_view::Page::Detail(_) | technician_view::Page::Registers(_)), "Devices").clicked() {
                        self.technician.page = technician_view::Page::Overview; self.technician.reference_context = false; self.library_open = false;
                    }
                    if ui.selectable_label(self.technician.page == technician_view::Page::Configurations, "Configuration").clicked() { self.technician.page = technician_view::Page::Configurations; self.library_open = false; }
                    if ui.selectable_label(self.technician.page == technician_view::Page::History, "History").clicked() { self.technician.page = technician_view::Page::History; self.library_open = false; }
                    if ui.selectable_label(reference_page, "References").clicked() { self.technician.page = technician_view::Page::References; self.technician.reference_context = true; self.library_open = false; }
                    if ui.selectable_label(matches!(self.technician.page, technician_view::Page::Troubleshooting(_)), "Troubleshooting").clicked() { self.technician.page = technician_view::Page::Troubleshooting(None); self.library_open = false; }
                    if ui.selectable_label(self.technician.page == technician_view::Page::Settings, "Settings").clicked() { self.technician.page = technician_view::Page::Settings; self.library_open = false; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.selectable_label(self.technician.page == technician_view::Page::Console, "Diagnostics").clicked() { self.technician.page = technician_view::Page::Console; self.library_open = false; }
                    });
                });
                ui.add_space(12.0); ui.separator();
                if self.offline { ui.weak("Offline mode · hardware access disabled"); }
                ui.add_space(20.0);
            });
            // Anchor the viewport to the window, and inset content independently of its scrollbar.
            egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt(self.technician.page.clone()).show(ui, |ui| {
                egui::Frame::default().inner_margin(egui::Margin { right: 18, ..Default::default() }).show(ui, |ui| {
                if self.technician.page == technician_view::Page::Settings { self.settings(ui); return; }
                if self.technician.page == technician_view::Page::History { self.history_view.show(ui, &self.history); }
                if self.technician.page == technician_view::Page::Configurations || self.library_open {
                    self.configuration_files(ui);
                }
                if self.technician.page != technician_view::Page::Console {
                    if self.technician.page != technician_view::Page::Configurations && !self.library_open {
                        let actions = self.technician.show(ui, &self.reference, &self.ports, &self.profiles, (self.result.as_ref(), self.bridge_source.as_ref().map_or("", |p| p.port.as_str()), self.adapter.as_ref()), self.foreground_busy());
                        for action in actions { self.handle_action(action); }
                    }
                    if !self.file_message.is_empty() { ui.horizontal_wrapped(|ui| { ui.label(&self.file_message); if ui.small_button("Dismiss message").clicked() { self.file_message.clear(); } }); }
                    if !self.backup_error.is_empty() { brand::attention(ui, "Backup needs attention", &self.backup_error); }
                    if matches!(self.technician.page, technician_view::Page::Overview | technician_view::Page::Configurations) && let Some(at) = &self.fetched_at { ui.weak(format!("Last received reading: {at}")); }
                    return;
                }
                ui.separator();
                ui.heading("Diagnostics");
                ui.strong("USB interfaces");
                ui.label("Choose the USB interface to use. E5 bridge console actions are available for a verified E5 bridge.");
                if self.ports.is_empty() { ui.label("No serial interfaces reported."); }
                for port in &self.ports {
                    ui.group(|ui| {
                        ui.strong(format!("{} — {}", port.port, port.description));
                        if let (Some(vid), Some(pid)) = (port.usb_vid, port.usb_pid) { ui.label(format!("USB {vid:04X}:{pid:04X}")); }
                        ui.label(format!("USB serial: {}", port.serial_number.as_deref().unwrap_or("unavailable")));
                        let candidate = modbus_configurator::adapter::is_bridge(port) || modbus_configurator::adapter::is_adapter(port);
                        ui.label(if port.busy { "Console operation in progress" } else { "Product identity checked when an action starts" });
                        if ui.add_enabled(candidate && self.active.is_none(), egui::Button::selectable(self.selected == port.port, "Use this interface")).clicked() {
                            self.selected = port.port.clone();
                            self.preferred_route = Some(port.clone());
                            self.preferences.automatic_polling = false;
                            self.status = "Automatic polling is off. Manual interface selected.".into();
                        }
                    });
                }
                ui.add_space(10.0);
                if !self.preferences.automatic_polling {
                    ui.weak("Automatic polling is off. Enable it in Settings to resume automatic readings.");
                }
                ui.heading(format!("E5 bridge console{}", if self.selected.is_empty() { String::new() } else { format!(" — {}", self.selected) }));
                ui.label("Read now asks the E5 bridge to poll its configured instruments. Backups are available in Configuration.");
                ui.horizontal(|ui| {
                    let ready = !self.selected.is_empty() && !self.foreground_busy()
                        && self.ports.iter().any(|p| p.port == self.selected && modbus_configurator::adapter::is_bridge(p));
                    if ui.add_enabled(ready, egui::Button::new("Read now")).clicked() { self.hardware(Operation::BridgeReadAll); }
                    if ui.add_enabled(ready, egui::Button::new("Release console")).clicked() { self.hardware(Operation::ClosePort); }
                });
                if let Some(result) = &self.result {
                    ui.label(format!("Verified console: {} firmware {}", result.identity.model, result.identity.firmware));
                    for exception in &result.exceptions { ui.label(format!("Point {}: {} (exception {})", exception.item, exception.message, exception.code)); }
                    if !result.readings.is_empty() {
                        let profile = self.profiles.iter().find(|profile| profile.contains_points(result));
                        ui.label("Last completed Read All:");
                        if let Some(profile) = profile { ui.label(format!("Names and units from the matching {} table; attached instrument identity is not established by this match.", profile.info.model)); }
                        else { ui.label("Register names are unavailable for this point table."); }
                        egui::Grid::new("readings").striped(true).show(ui, |ui| {
                            ui.strong("Point"); ui.strong("Register"); ui.strong("Value"); ui.strong("Units"); ui.end_row();
                            for reading in &result.readings {
                                let register = profile.and_then(|p| p.point_register(reading.item));
                                ui.label(reading.item.to_string()); ui.label(register.map_or("Unmapped", |r| r.name.as_str()));
                                ui.label(reading.value.to_string()); ui.label(register.map_or("—", |r| r.units.as_str())); ui.end_row();
                            }
                        });
                    }
                    crate::brand::collapsing(ui, "Native point table", |ui| { ui.monospace(&result.native_tsv); });

                }
                ui.separator();
                crate::brand::collapsing(ui, "Activity log", |ui| {
                    ui.weak("Timestamped actions, results, and connection changes. Automatic polling appears only when it fails.");
                    ui.horizontal(|ui| {
                        if ui.button("Export diagnostics…").clicked()
                            && let Some(path) = rfd::FileDialog::new().add_filter("Text log", &["txt"]).set_file_name("polygon-diagnostics.txt").save_file()
                        {
                            self.status = match crate::diagnostic_log::export(&path) {
                                Ok(()) => "Diagnostic logs exported.".into(),
                                Err(error) => format!("Could not export diagnostics: {error}"),
                            };
                        }
                        if ui.add_enabled(!self.replay_log.is_empty(), egui::Button::new("Copy log")).clicked() {
                            let text = self.replay_log.join("\r\n");
                            self.status = match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
                                Ok(()) => "Activity log copied to clipboard.".into(),
                                Err(error) => format!("Could not copy activity log: {error}. Use Save log instead."),
                            };
                        }
                        if ui.add_enabled(!self.replay_log.is_empty(), egui::Button::new("Save log…")).clicked()
                            && let Some(path) = rfd::FileDialog::new().add_filter("Text log", &["txt"]).set_file_name("activity-log.txt").save_file()
                        {
                            self.status = match std::fs::write(path, self.replay_log.join("\r\n")) {
                                Ok(()) => "Activity log saved.".into(),
                                Err(error) => format!("Could not save activity log: {error}"),
                            };
                        }
                        if ui.add_enabled(!self.replay_log.is_empty(), egui::Button::new("Clear log")).clicked() { self.replay_log.clear(); }
                    });
                    if self.replay_log.is_empty() { ui.label("No activity recorded yet."); }
                    else {
                        let mut rows: Vec<(&str, usize)> = Vec::new();
                        for message in &self.replay_log {
                            if let Some((last, count)) = rows.last_mut() && *last == message { *count += 1; }
                            else { rows.push((message.as_str(), 1)); }
                        }
                        ui.weak("Newest first · last 64 events");
                        egui::ScrollArea::vertical().id_salt("diagnostic-activity").max_height(280.0).auto_shrink([false, true]).show(ui, |ui| {
                            for (message, count) in rows.into_iter().rev() {
                                ui.horizontal_top(|ui| {
                                    ui.add_sized([45.0, 20.0], egui::Label::new(if count > 1 { format!("{count} times") } else { String::new() }));
                                    ui.add(egui::Label::new(message).wrap());
                                });
                                ui.separator();
                            }
                        });
                    }
                });
                crate::brand::collapsing(ui, "Offline diagnostics", |ui| {
                    ui.weak("Checks recorded console responses without communicating with connected equipment.");
                    if ui.add_enabled(self.active.is_none(), egui::Button::new("Run offline prompt replay")).clicked() {
                        self.replay_log.clear();
                        let fixture: Command = serde_json::from_str(include_str!("../tests/fixtures/navigation.json")).expect("embedded fixture");
                        self.request(fixture.operation, false);
                    }
                    ui.weak("Replay results appear in Activity log.");
                });
                ui.label("Point-table programming and backups are in Configuration. Firmware updates are not implemented.");
                });
            });
        });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
