//! Device details and register tables; rendering never opens hardware directly.
use super::*;

impl TechnicianView {
    /// Render device readings and available actions.
    pub(super) fn detail_page(
        &mut self,
        ui: &mut egui::Ui,
        key: String,
        context: DevicePageContext<'_>,
    ) -> Vec<Action> {
        let DevicePageContext {
            reference,
            ports,
            profiles,
            result,
            direct,
            busy,
        } = context;
        let mut actions = vec![];
        if let Some(slave) = key
            .strip_prefix("slave:")
            .and_then(|id| id.parse::<u8>().ok())
        {
            return self.network_detail(ui, slave, result, profiles, reference, false);
        }
        let Some(device) = reference.devices.get(&key) else {
            return actions;
        };
        self.header(
            ui,
            &device.name,
            &device.subtitle,
            if self.reference_context {
                Page::References
            } else {
                Page::Overview
            },
        );
        if self.reference_context {
            ui.weak("Model reference");
            ui.add_space(8.0);
            ui.add(egui::Label::new(&device.description).wrap());
            ui.add_space(16.0);
            ui.horizontal_wrapped(|ui| {
                crate::device_art::photo(ui, &key, egui::vec2(240.0, 160.0));
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.strong("Documentation");
                    if reference.registers.contains_key(&key) && ui.button("Register map").clicked()
                    {
                        self.page = Page::Registers(key.clone());
                    }
                    for (title, path) in reference::manuals(&key) {
                        if ui.button(*title).clicked() {
                            actions.push(Action::OpenArtifact((*path).into()));
                        }
                    }
                    if reference::manuals(&key).is_empty() {
                        ui.add_enabled(false, egui::Button::new("Open manual (PDF)"));
                        ui.weak("No documentation bundled");
                    }
                    if key == "iaq_plus" && ui.button("Radio reference").clicked() {
                        self.page = Page::Radio;
                    }
                    if ui.button("Setup & troubleshooting").clicked() {
                        self.page = Page::Troubleshooting(Some(key.clone()));
                    }
                });
            });
            return actions;
        }
        if key == "bridge" {
            crate::brand::bridge_eui(
                ui,
                result
                    .filter(|_| self.bridge_connected)
                    .and_then(|r| r.dev_eui.as_deref()),
            );
        }
        if key == "adapter" && ports.iter().any(is_synetica) {
            crate::brand::attention(
                ui,
                "Adapter reads blocked",
                "Switch the Modbus Bridge off with its hardware switch before direct adapter reads. External power can remain connected.",
            );
        }
        let profile = profiles
            .iter()
            .filter(|p| reference::profile_matches(&key, &p.info.id))
            .find(|p| result.is_some_and(|r| p.contains_points(r)))
            .or_else(|| {
                profiles
                    .iter()
                    .find(|p| reference::profile_matches(&key, &p.info.id))
            });
        let direct = direct.filter(|d| key == "adapter" || d.key == key);
        let reading_key = direct.map_or(key.as_str(), |d| d.key.as_str());
        let reading_device = reference.devices.get(reading_key).unwrap_or(device);
        let connected = direct.is_some_and(|d| {
            ports
                .iter()
                .any(|p| modbus_configurator::adapter::same_route(p, &d.port))
        }) || match key.as_str() {
            "synetica_usb" => ports.iter().any(is_synetica),
            "bridge" => self.bridge_connected && result.is_some(),
            "dpt146" | "hmd65" | "wattnode" | "ati-f12" => {
                self.bridge_connected
                    && profile.is_some_and(|p| result.is_some_and(|r| p.contains_points(r)))
            }
            "adapter" => ports.iter().any(is_adapter),
            _ => false,
        };
        let has_readings = direct.is_some_and(|d| !d.values.is_empty())
            || profile.is_some_and(|p| {
                result.is_some_and(|r| p.contains_points(r) && !r.readings.is_empty())
            });
        let stale = if direct.is_some() {
            self.adapter_stale
        } else {
            self.bridge_stale
        };
        ui.weak(if !connected || stale {
            reading_state(connected, stale, has_readings)
        } else if direct.is_some() {
            "Connected through USB adapter"
        } else if matches!(key.as_str(), "dpt146" | "hmd65" | "wattnode" | "ati-f12") {
            "Configured through Modbus Bridge · sensor identity unverified"
        } else if key == "synetica_usb" {
            "Connected · not identified"
        } else {
            "Connected"
        });
        if direct.is_none()
            && matches!(
                key.as_str(),
                "bridge" | "dpt146" | "hmd65" | "wattnode" | "ati-f12"
            )
        {
            if profile.is_none_or(|p| result.is_some_and(|r| p.contains_points(r)))
                && let Some(notice) = &self.configuration_change
            {
                crate::brand::attention(ui, "Configuration changed", notice);
            }
            if let Some(warning) =
                configuration_warning(profile, result, stale).filter(|_| !self.errors_acknowledged)
            {
                crate::brand::attention(ui, "Check sensor configuration", &warning);
            }
            if matches!(key.as_str(), "dpt146" | "hmd65" | "wattnode" | "ati-f12")
                && profile.is_some_and(|p| result.is_some_and(|r| p.contains_points(r)))
            {
                ui.add(egui::Label::new("The point table selects register addresses; it does not identify the attached sensor. Confirm the physical sensor matches this configured model, even when reads succeed.").wrap());
            }
        }
        if let Some(direct) = direct {
            ui.weak(direct.settings.label());
            if direct.family_only {
                ui.label("WND meter module identified. Confirm the enclosure model on its label.");
            }
            if !self.errors_acknowledged && !direct.errors.is_empty() {
                crate::brand::collapsing(ui, "Register errors", |ui| {
                    for (address, error) in &direct.errors {
                        ui.label(format!("Register address {address}: {error}"));
                    }
                });
            }
        }
        if direct.is_none()
            && let Some(result) = result
            && profile.is_some_and(|p| p.contains_points(result))
            && let Some(slave) = result
                .native_tsv
                .lines()
                .nth(1)
                .and_then(|r| r.split('\t').nth(1))
                .and_then(|s| s.parse().ok())
        {
            self.slave_health(ui, result, slave);
        }
        if let Some(direct) = direct
            && !self.errors_acknowledged
            && !direct.errors.is_empty()
            && direct
                .values
                .keys()
                .all(|address| direct.errors.contains_key(address))
        {
            crate::brand::attention(
                ui,
                "No successful register reads",
                "Possible physical connection or serial line issue. Check device power, wiring, slave address, baud rate and parity.",
            );
        }
        ui.add_space(16.0);
        ui.columns(2, |columns| {
        let photo_width = columns[1].available_width().min(320.0);
        crate::device_art::photo(&mut columns[1], &key, egui::vec2(photo_width, 200.0));
        columns[1].add_space(12.0);
        columns[0].strong("Readings");
        if key == "adapter" {
            if direct.is_some() { columns[0].label(format!("Sensor: {}", reading_device.name)); }
            else { columns[0].label("No sensor read has completed through this adapter. USB detection alone does not confirm sensor communication."); }
        }
        columns[0].checkbox(&mut self.show_native, "Show native values alongside display units");
        let has_readings = direct.is_some_and(|d| !d.values.is_empty())
            || profile.is_some_and(|p| {
                result.is_some_and(|r| p.contains_points(r) && !r.readings.is_empty())
            });
        columns[0].weak(if has_readings {
            "Last completed read"
        } else {
            "No readings yet"
        });
        let mut readouts: Vec<_> = reading_device.readouts.iter().map(|r| (r.label.clone(), r.unit.clone(), None)).collect();
        if let Some(registers) = reference.registers.get(reading_key) {
            for register in registers {
                let live = direct.and_then(|d| d.values.get(&register.first_pdu()?).copied()).or_else(|| value(profile, result, register));
                if live.is_some() && !readouts.iter().any(|(label, _, _)| register.readout_label.as_ref() == Some(label)) {
                    readouts.push((register.name.clone(), register.unit.clone(), Some(register)));
                }
            }
        }
        for (label, unit, explicit_register) in readouts {
            let readout = reference::Readout { label, unit };
            let live_register = explicit_register.or_else(|| reference.registers.get(reading_key).and_then(|regs| {
                regs.iter()
                    .find(|r| r.readout_label.as_deref() == Some(&readout.label) && value(profile, result, r).is_some())
                    .or_else(|| regs.iter().find(|r| r.readout_label.as_deref() == Some(&readout.label)))
            }));
            let live = live_register.and_then(|r| {
                direct
                    .and_then(|d| d.values.get(&r.first_pdu()?).copied())
                    .or_else(|| value(profile, result, r))
            });
            columns[0].group(|ui| {
                let text = if let Some(register) = live_register && !crate::units::choices(&register.unit).is_empty() {
                    self.display_units(ui, reading_key, register.first_pdu(), live, &register.unit)
                } else if let Some(v) = live {
                    let unit = live_register.map_or(readout.unit.as_str(), |r| r.unit.as_str());
                    format!("{} {unit}", display_value(v, unit))
                } else if reading_key == "dpt146" && readout.label == "Device health" {
                    if !connected || stale { "Unavailable · readings stale".into() } else {
                        let health_values: Option<Vec<f64>> = ["Fault status", "Online status", "Error code"].iter().map(|label| {
                            let reg = reference.registers.get(reading_key)?.iter().find(|r| r.readout_label.as_deref() == Some(*label))?;
                            if let Some(d) = direct {
                                let address = reg.first_pdu()?;
                                if d.errors.contains_key(&address) { return None; }
                                d.values.get(&address).copied()
                            } else {
                                let p = profile?;
                                let r = result?;
                                let item = p.rows.iter().find(|(_, row)| row.split('\t').nth(3).and_then(|a| a.parse::<u16>().ok()) == reg.first_pdu())?.0;
                                if r.exceptions.iter().any(|e| e.item == *item) { return None; }
                                value(profile, result, reg)
                            }
                        }).collect();
                        match health_values {
                            Some(v) if v == [1.0, 1.0, 0.0] => "Online".into(),
                            Some(_) => "Attention required".into(),
                            None => "Unavailable · incomplete status read".into(),
                        }
                    }
                } else if key == "bridge"
                    && let Some(result) = result
                {
                    match readout.label.as_str() {
                        "Configured points" => result
                            .native_tsv
                            .lines()
                            .skip(1)
                            .filter(|l| !l.trim().is_empty())
                            .count()
                            .to_string(),
                        "Successful reads" => {
                            result.successful_reads.map_or("—".into(), |count| {
                                format!(
                                    "{count} successful · {} failed",
                                    result.exceptions.len()
                                )
                            })
                        }
                        _ => "—".into(),
                    }
                } else {
                    "—".into()
                };
                ui.horizontal(|ui| {
                    ui.label(&readout.label);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(RichText::new(text).font(egui::FontId::new(
                                20.0,
                                egui::FontFamily::Proportional,
                            )));
                        },
                    );
                });
                if live.is_some()
                    && let Some(address) = live_register.and_then(Register::first_pdu)
                {
                    let times = if direct.is_some() {
                        &self.adapter_times
                    } else {
                        &self.bridge_times
                    };
                    if let Some(at) = times.get(&address) {
                        ui.weak(format!("Last good: {at}"));
                    }
                    let failed = if let Some(d) = direct {
                        self.adapter_stale || d.errors.contains_key(&address)
                    } else {
                        self.bridge_stale
                            || result.is_some_and(|r| {
                                r.exceptions.iter().any(|e| {
                                    profile.is_some_and(|p| {
                                        p.rows
                                            .get(&e.item)
                                            .and_then(|row| row.split('\t').nth(3))
                                            .and_then(|v| v.parse::<u16>().ok())
                                            == Some(address)
                                    })
                                })
                            })
                    };
                    if !connected || failed {
                        ui.weak("Stale");
                    }
                }
            });
        }
        if key == "bridge" {
            columns[1].strong("Device");
        }

        if key == "bridge"
            && let Some(result) = result
        {
            columns[1].label(format!(
                "{} · firmware {}",
                result.identity.model, result.identity.firmware
            ));
        }
        columns[1].add_space(12.0);
        if reference.registers.contains_key(reading_key)
            && columns[1].button("Register table").clicked()
        {
            self.page = Page::Registers(reading_key.into());
        }
        if key == "bridge" {
            if let Some(result) = result
                && result.successful_reads.is_some()
            {
                crate::brand::collapsing(&mut columns[1], "Point results", |ui| {
                    egui::Grid::new("Modbus Bridge-point-results").striped(true).show(
                        ui,
                        |ui| {
                            ui.strong("Point");
                            ui.strong("Transmitted address");
                            ui.strong("Result");
                            ui.end_row();
                            for line in result
                                .native_tsv
                                .lines()
                                .skip(1)
                                .filter(|l| !l.trim().is_empty())
                            {
                                let fields: Vec<_> = line.split('\t').collect();
                                let Ok(item) = fields[0].parse::<u8>() else {
                                    continue;
                                };
                                ui.label(item.to_string());
                                ui.label(fields[3]);
                                if let Some(reading) =
                                    result.readings.iter().find(|r| r.item == item)
                                {
                                    ui.label(reading.value.to_string());
                                    if let Some(error) = result
                                        .exceptions
                                        .iter()
                                        .find(|e| e.item == item)
                                    {
                                        ui.weak(format!(
                                            "Last good value; {}",
                                            error.message
                                        ));
                                    }
                                } else if let Some(error) =
                                    result.exceptions.iter().find(|e| e.item == item)
                                {
                                    ui.colored_label(
                                        Color32::from_rgb(165, 65, 40),
                                        format!("{} ({})", error.message, error.code),
                                    );
                                } else {
                                    ui.label("Unavailable");
                                }
                                ui.end_row();
                            }
                        },
                    );
                });
            }
            if columns[1].button("Configure Modbus Bridge").clicked() {
                self.page = Page::Configurations;
            }
        }
        if ["dpt146", "hmd65", "wattnode", "ati-f12", "adapter"].contains(&key.as_str())
            && columns[1]
                .add_enabled(!busy && connected && !(key == "adapter" && ports.iter().any(is_synetica)), egui::Button::new("Read now"))
                .clicked()
        {
            actions.push(Action::ReadBridge);
        }
        if key == "iaq_plus" && columns[1].button("Radio reference").clicked() {
            self.page = Page::Radio;
        }
        columns[1].add_space(16.0);
        crate::brand::collapsing(&mut columns[1], "Reference information", |ui| {
            ui.label(&device.description);
        });
        columns[1].strong("Device manuals");
        let manuals = reference::manuals(&key);
        for (title, path) in manuals {
            if columns[1].button(*title).clicked() { actions.push(Action::OpenArtifact((*path).into())); }
        }
        if manuals.is_empty() {
            columns[1].add_enabled(false, egui::Button::new("Open manual (PDF)"));
            columns[1].weak("No documentation bundled");
        }
        if matches!(key.as_str(), "bridge" | "iaq_plus") {
            columns[1].weak("Full user guide not bundled; manufacturer download requires an account.");
        }

        if columns[1].button("Setup & troubleshooting").clicked() {
            self.page = Page::Troubleshooting(Some(key.clone()));
        }
    });
        actions
    }

    /// Render register values, units, and documentation with wrapped rows.
    pub(super) fn register_page(
        &mut self,
        ui: &mut egui::Ui,
        key: String,
        context: DevicePageContext<'_>,
    ) -> Vec<Action> {
        let DevicePageContext {
            reference,
            profiles,
            result,
            direct,
            ..
        } = context;
        let actions = vec![];
        if let Some(slave) = key
            .strip_prefix("slave:")
            .and_then(|id| id.parse::<u8>().ok())
        {
            return self.network_detail(ui, slave, result, profiles, reference, true);
        }

        let Some(device) = reference.devices.get(&key) else {
            return actions;
        };
        self.header(
            ui,
            &format!(
                "{} — {}",
                device.name,
                if self.reference_context {
                    "register map"
                } else {
                    "live registers"
                }
            ),
            "",
            Page::Detail(key.clone()),
        );
        ui.horizontal(|ui| {
            ui.label("Find");
            ui.text_edit_singleline(&mut self.search);
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Address notation");
            ui.selectable_value(&mut self.one_based_addresses, false, "Zero-based address");
            ui.selectable_value(&mut self.one_based_addresses, true, "1-based");
            ui.weak("Display only · manual addresses and programmed settings stay unchanged");
        });
        if self.reference_context {
            ui.weak("Manufacturer register definitions · native units");
            let query = self.search.to_lowercase();
            if let Some(registers) = reference.registers.get(&key) {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let widths = [180.0, 75.0, 75.0, 95.0, 60.0, 90.0, 320.0];
                        ui.horizontal_top(|ui| {
                            for (label, width) in [
                                "Register",
                                "Manual",
                                if self.one_based_addresses {
                                    "Address (one-based)"
                                } else {
                                    "Address (zero-based)"
                                },
                                "Data type / word order",
                                "Access",
                                "Units",
                                "Description",
                            ]
                            .into_iter()
                            .zip(widths)
                            {
                                register_cell(
                                    ui,
                                    width,
                                    egui::Label::new(RichText::new(label).strong()).wrap(),
                                );
                            }
                        });
                        for register in registers.iter().filter(|r| {
                            format!("{} {} {}", r.name, r.logical, r.pdu)
                                .to_lowercase()
                                .contains(&query)
                        }) {
                            ui.horizontal_top(|ui| {
                                for (text, width) in [
                                    &register.name,
                                    &register.logical,
                                    &display_address(&register.pdu, self.one_based_addresses),
                                    &register.data_type,
                                    &register.access,
                                    &register.unit,
                                    &register.description,
                                ]
                                .into_iter()
                                .zip(widths)
                                {
                                    register_cell(ui, width, egui::Label::new(text).wrap());
                                }
                            });
                            ui.add_space(8.0);
                        }
                    });
            }
            return actions;
        }
        ui.checkbox(
            &mut self.show_native,
            "Show native values alongside display units",
        );
        let query = self.search.to_lowercase();
        let profile = profiles
            .iter()
            .filter(|p| reference::profile_matches(&key, &p.info.id))
            .find(|p| result.is_some_and(|r| p.contains_points(r)))
            .or_else(|| {
                profiles
                    .iter()
                    .find(|p| reference::profile_matches(&key, &p.info.id))
            });
        if let Some(registers) = reference.registers.get(&key) {
            // Keep technical columns readable; give wider windows to descriptive text.
            let mut widths = [
                180.0, 65.0, 65.0, 95.0, 60.0, 80.0, 100.0, 110.0, 160.0, 240.0,
            ];
            let extra = (ui.available_width()
                - widths.iter().sum::<f32>()
                - ui.spacing().item_spacing.x * 9.0)
                .max(0.0);
            widths[0] += extra * 0.15;
            widths[8] += extra * 0.25;
            widths[9] += extra * 0.60;
            egui::ScrollArea::horizontal()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal_top(|ui| {
                            for label in [
                                "Register",
                                "Manual",
                                if self.one_based_addresses {
                                    "Address (one-based)"
                                } else {
                                    "Address (zero-based)"
                                },
                                "Data type / word order",
                                "Access",
                                "Readout",
                                "Since last read",
                                "Units",
                                "Decoded meaning",
                                "Description",
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                register_cell(
                                    ui,
                                    widths[label.0],
                                    egui::Label::new(egui::RichText::new(label.1).strong()).wrap(),
                                );
                            }
                        });
                        for register in registers {
                            if !format!("{} {} {}", register.name, register.logical, register.pdu)
                                .to_lowercase()
                                .contains(&query)
                            {
                                continue;
                            }
                            let live = direct
                                .filter(|d| d.key == key)
                                .and_then(|d| d.values.get(&register.first_pdu()?).copied())
                                .or_else(|| value(profile, result, register));
                            let received = live.and_then(|_| {
                                self.received_at(profile, direct.filter(|d| d.key == key), register)
                            });
                            let background = ui.painter().add(egui::Shape::Noop);
                            let row = ui.horizontal_top(|ui| {
                                for text in [
                                    &register.name,
                                    &register.logical,
                                    &display_address(&register.pdu, self.one_based_addresses),
                                    &register.data_type,
                                    &register.access,
                                ]
                                .into_iter()
                                .enumerate()
                                {
                                    register_cell(
                                        ui,
                                        widths[text.0],
                                        egui::Label::new(text.1).wrap(),
                                    );
                                }
                                // Native decoding stays independent of display-unit selection.
                                let options = crate::units::choices(&register.unit);
                                let index = self
                                    .unit_choices
                                    .get(&(key.clone(), register.first_pdu().unwrap_or(0)))
                                    .copied()
                                    .unwrap_or_else(|| self.unit_preset.index(&register.unit));
                                let (_, scale, offset) = options.get(index).copied().unwrap_or((
                                    &register.unit,
                                    1.0,
                                    0.0,
                                ));
                                register_cell(
                                    ui,
                                    widths[5],
                                    egui::Label::new(live.map_or("—".into(), |v| {
                                        let display = display_value(
                                            v * scale + offset,
                                            options.get(index).map_or(&register.unit, |o| o.0),
                                        );
                                        if self.show_native {
                                            format!(
                                                "{display}\nNative: {} {}",
                                                display_value(v, &register.unit),
                                                register.unit
                                            )
                                        } else {
                                            display
                                        }
                                    }))
                                    .wrap(),
                                );
                                register_cell(
                                    ui,
                                    widths[6],
                                    egui::Label::new(freshness::age_text(ui, received)),
                                );
                                ui.allocate_ui_with_layout(
                                    egui::vec2(widths[7], 0.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_width(widths[7]);
                                        self.display_units(
                                            ui,
                                            &key,
                                            register.first_pdu(),
                                            None,
                                            &register.unit,
                                        );
                                        if options.is_empty() {
                                            ui.label(&register.unit);
                                        }
                                    },
                                );
                                register_cell(
                                    ui,
                                    widths[8],
                                    egui::Label::new(register.decode(live.map(|v| v as i64)))
                                        .wrap(),
                                )
                                .on_hover_text(register.decode(None));
                                register_cell(
                                    ui,
                                    widths[9],
                                    egui::Label::new(&register.description).wrap(),
                                );
                            });
                            if self.reading_ok(
                                profile,
                                result,
                                direct.filter(|d| d.key == key),
                                register,
                            ) {
                                ui.painter().set(
                                    background,
                                    egui::Shape::rect_filled(
                                        row.response.rect,
                                        3.0,
                                        crate::brand::CYAN.gamma_multiply(0.12),
                                    ),
                                );
                            }
                            ui.add_space(6.0);
                        }
                    });
                });
        }
        actions
    }
}
