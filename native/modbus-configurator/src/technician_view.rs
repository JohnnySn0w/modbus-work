//! Device navigation and shared readout presentation.
mod custom_profiles;
mod device_pages;
mod network_readings;
use eframe::egui::{self, Color32, RichText};
use modbus_configurator::{
    bridge::BridgeResult,
    catalog::Profile,
    contract::PortInfo,
    reference::{self, Device, Reference, Register},
};

#[derive(Default, Clone, PartialEq, Hash)]
pub enum Page {
    #[default]
    Overview,
    References,
    History,
    Detail(String),
    Registers(String),
    Radio,
    Console,
    Configurations,
    Settings,
    Troubleshooting(Option<String>),
}
#[derive(Clone)]
pub enum Action {
    Refresh,
    BackupBridge,
    ReadBridge,
    ChooseConfig(String),
    OpenArtifact(String),
}
#[derive(Default)]
pub struct TechnicianView {
    pub page: Page,
    pub reference_context: bool,
    pub bridge_stale: bool,
    pub bridge_connected: bool,
    pub configuration_change: Option<String>,
    pub adapter_stale: bool,
    pub bridge_times: std::collections::BTreeMap<u16, String>,
    pub bridge_point_times: std::collections::BTreeMap<u8, String>,
    pub network_draft: Option<(String, Vec<modbus_configurator::network::Device>)>,
    network_applied: Option<(String, Vec<modbus_configurator::network::Device>)>,
    network_types: std::collections::BTreeMap<(String, u8), network_readings::TypeChoice>,
    saved_profiles: Vec<modbus_configurator::custom_profile::SavedProfile>,
    custom_profile_root: Option<std::path::PathBuf>,
    custom_profile_name: String,
    custom_profile_message: String,
    pub adapter_times: std::collections::BTreeMap<u16, String>,
    show_native: bool,
    one_based_addresses: bool,
    pub unit_preset: crate::units::Preset,
    unit_choices: std::collections::BTreeMap<(String, u16), usize>,
    search: String,
}

fn display_value(value: f64, unit: &str) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    let decimals = match unit {
        "bara" | "bar" | "kV" | "kVac" | "kW" | "MPa" | "MPa(a)" => 3,
        "% vol" => 4,
        "ppmv" | "ppm" => 0,
        "" | "—" => return value.to_string(),
        _ => 2,
    };
    let rounded = if value.abs() < 0.5 * 10_f64.powi(-(decimals as i32)) {
        0.0
    } else {
        value
    };
    format!("{rounded:.decimals$}")
}

/// Change address notation only; register lookup and wire addresses stay zero-based.
fn display_address(pdu: &str, one_based: bool) -> String {
    if !one_based {
        return pdu.into();
    }
    pdu.split(['–', '-'])
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .ok()
                .and_then(|v| v.checked_add(1))
                .map(|v| v.to_string())
        })
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("–"))
        .unwrap_or_else(|| pdu.into())
}
fn reading_state(connected: bool, stale: bool, has_readings: bool) -> &'static str {
    match (connected, stale, has_readings) {
        (false, _, true) => "Disconnected · last good readings retained",
        (false, _, false) => "Disconnected · no readings yet",
        (true, true, true) => "Read failed · last good readings retained",
        (true, true, false) => "Read failed · no readings yet",
        _ => "Connected",
    }
}

use modbus_configurator::adapter::{AdapterResult, is_adapter, is_bridge as is_synetica};
fn value(
    profile: Option<&Profile>,
    result: Option<&BridgeResult>,
    register: &Register,
) -> Option<f64> {
    let profile = profile?;
    let result = result.filter(|r| profile.contains_points(r))?;
    let address = register.first_pdu()?;
    profile.rows.iter().find_map(|(item, row)| {
        let pdu = row.split('\t').nth(3)?.parse::<u16>().ok()?;
        (pdu == address)
            .then(|| {
                result
                    .readings
                    .iter()
                    .find(|r| r.item == *item)
                    .map(|r| r.value)
            })
            .flatten()
    })
}

fn configuration_warning(
    profile: Option<&Profile>,
    result: Option<&BridgeResult>,
    stale: bool,
) -> Option<String> {
    let result = result?;
    if profile.is_some_and(|p| !p.contains_points(result)) {
        return None;
    }
    if stale {
        return Some("Read failed. Check the sensor type, wiring and serial settings; retained readings are stale.".into());
    }
    if result.successful_reads.is_none() {
        return Some("Awaiting live validation of the configured point table.".into());
    }
    let implausible = profile.map_or(0, |p| {
        result
            .readings
            .iter()
            .filter(|reading| {
                let Some(register) = p.point_register(reading.item) else {
                    return false;
                };
                match register.units.as_str() {
                    "°C" => !(-150.0..=200.0).contains(&reading.value),
                    "%RH" => !(0.0..=100.0).contains(&reading.value),
                    "bara" => !(0.0..=12.0).contains(&reading.value),
                    "ppmv" => !(0.0..=1_000_000.0).contains(&reading.value),
                    _ => !reading.value.is_finite(),
                }
            })
            .count()
    });
    if !result.exceptions.is_empty() || implausible > 0 {
        Some(format!(
            "Possible configuration mismatch · {} point errors · {implausible} implausible readings. Check the connected sensor model, address and serial settings.",
            result.exceptions.len()
        ))
    } else {
        None
    }
}

/// Read-only inputs shared by device and register pages.
struct DevicePageContext<'a> {
    reference: &'a Reference,
    ports: &'a [PortInfo],
    profiles: &'a [Profile],
    result: Option<&'a BridgeResult>,
    direct: Option<&'a AdapterResult>,
    busy: bool,
}

impl TechnicianView {
    pub fn reset_unit_overrides(&mut self) {
        self.unit_choices.clear();
    }
    fn display_units(
        &mut self,
        ui: &mut egui::Ui,
        device: &str,
        address: Option<u16>,
        value: Option<f64>,
        native: &str,
    ) -> String {
        let options = crate::units::choices(native);
        let mut selected = self.unit_preset.index(native);
        if let Some(address) = address
            && !options.is_empty()
        {
            selected = self
                .unit_choices
                .get(&(device.into(), address))
                .copied()
                .unwrap_or(selected)
                % options.len();
            if ui.small_button(format!("Units: {}", options[selected].0)).on_hover_text("Cycle display units. Native readings, history and E5 bridge settings are unchanged.").clicked() {
                selected = (selected + 1) % options.len();
                self.unit_choices.insert((device.into(), address), selected);
            }
        }
        let (unit, scale, offset) = options.get(selected).copied().unwrap_or((native, 1.0, 0.0));
        let display = value.map_or(format!("— {unit}"), |v| {
            format!("{} {unit}", display_value(v * scale + offset, unit))
        });
        if self.show_native {
            format!(
                "{display}\nNative: {} {native}",
                value.map_or("—".into(), |v| display_value(v, native))
            )
        } else {
            display
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        reference: &Reference,
        ports: &[PortInfo],
        profiles: &[Profile],
        snapshot: (Option<&BridgeResult>, &str, Option<&AdapterResult>),
        busy: bool,
    ) -> Vec<Action> {
        let (result, result_port, direct) = snapshot;
        let network = result.is_some_and(|r| {
            network_readings::is_network(&r.native_tsv)
                || !profiles.iter().any(|p| p.contains_points(r))
        });
        let mut actions = vec![];
        match self.page.clone() {
            Page::Overview => {
                ui.horizontal(|ui| {
                    ui.heading("Devices");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Refresh").clicked() {
                            actions.push(Action::Refresh);
                        }
                    });
                });
                ui.add_space(25.0);
                let mut nodes: Vec<(&str, String)> = vec![];
                let bridge = ports
                    .iter()
                    .find(|p| is_synetica(p) && p.port == result_port)
                    .or_else(|| ports.iter().find(|p| is_synetica(p)));
                if let Some(port) = bridge {
                    let known = self.bridge_connected
                        && port.port == result_port
                        && result.is_some_and(|r| {
                            r.identity.model == "ENL-MOD-32" && r.identity.firmware == "3.6"
                        });
                    nodes.push((
                        "bridge",
                        if known {
                            port.port.clone()
                        } else {
                            format!("{} · awaiting identification", port.port)
                        },
                    ));
                    if known && !network {
                        for key in ["dpt146", "hmd65", "wattnode", "ati-f12"] {
                            if profiles.iter().any(|p| {
                                reference::profile_matches(key, &p.info.id)
                                    && result.is_some_and(|r| p.contains_points(r))
                            }) {
                                nodes.push((key, "Configured on E5 bridge".into()));
                            }
                        }
                    }
                }
                if !self.bridge_connected && result.is_some() {
                    nodes.push(("bridge", format!("{result_port} - disconnected")));
                    for key in ["dpt146", "hmd65", "wattnode", "ati-f12"]
                        .into_iter()
                        .filter(|_| !network)
                    {
                        if profiles.iter().any(|p| {
                            reference::profile_matches(key, &p.info.id)
                                && result.is_some_and(|r| p.contains_points(r))
                        }) {
                            nodes.push((key, "Disconnected - last good readings".into()));
                        }
                    }
                }
                for port in ports.iter().filter(|p| is_adapter(p)) {
                    nodes.push((
                        "adapter",
                        if bridge.is_some() {
                            format!("{} · paused while E5 bridge is connected", port.port)
                        } else {
                            format!("{} · USB adapter available", port.port)
                        },
                    ));
                }
                if let Some(direct) = direct {
                    nodes.push((
                        &direct.key,
                        format!(
                            "{} - {}",
                            direct.port.port,
                            if ports
                                .iter()
                                .any(|p| modbus_configurator::adapter::same_route(p, &direct.port))
                            {
                                direct.settings.label()
                            } else {
                                "Disconnected - last good readings".into()
                            }
                        ),
                    ));
                }
                if nodes.is_empty() {
                    ui.group(|ui| {
                        ui.heading("No devices connected");
                        ui.label("Connect an E5 bridge or USB-COMi-TB to get started.");
                    });
                }
                for (interfaces, heading) in [(true, "Connections"), (false, "Sensors")] {
                    let in_group = |key: &str| {
                        matches!(key, "bridge" | "adapter" | "synetica_usb") == interfaces
                    };
                    if !nodes.iter().any(|(key, _)| in_group(key)) {
                        continue;
                    }
                    ui.strong(heading);
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        for (key, connection) in nodes.iter().filter(|(key, _)| in_group(key)) {
                            if let Some(device) = reference.devices.get(*key) {
                                let response = ui.group(|ui| {
                                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                        ui.set_width(285.0);
                                        ui.set_min_height(210.0);
                                        Self::icon(ui, device);
                                        ui.label(RichText::new(&device.name).strong().size(17.0));
                                        if matches!(
                                            *key,
                                            "dpt146" | "hmd65" | "wattnode" | "ati-f12"
                                        ) && direct.is_none_or(|d| d.key != *key)
                                        {
                                            ui.add(
                                                egui::Label::new(
                                                    "Configured model · sensor identity unverified",
                                                )
                                                .wrap(),
                                            );
                                        }
                                        if matches!(
                                            *key,
                                            "bridge" | "dpt146" | "hmd65" | "wattnode" | "ati-f12"
                                        ) && direct.is_none_or(|d| d.key != *key)
                                        {
                                            let profile = profiles.iter().find(|p| {
                                                reference::profile_matches(key, &p.info.id)
                                            });
                                            if profile.is_none_or(|p| {
                                                result.is_some_and(|r| p.contains_points(r))
                                            }) && let Some(notice) = &self.configuration_change
                                            {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(notice)
                                                            .color(crate::brand::ORANGE),
                                                    )
                                                    .wrap(),
                                                );
                                            }
                                            if let Some(warning) = configuration_warning(
                                                profile,
                                                result,
                                                self.bridge_stale,
                                            ) {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(warning)
                                                            .color(crate::brand::ORANGE),
                                                    )
                                                    .wrap(),
                                                );
                                            }
                                        }
                                        if *key == "synetica_usb" {
                                            ui.weak("Not identified");
                                        }
                                        ui.add_space(12.0);
                                        if *key == "adapter" && ports.iter().any(is_synetica) {
                                            ui.colored_label(
                                                crate::brand::ORANGE,
                                                "Blocked · another Modbus master may be active",
                                            );
                                            ui.weak(connection);
                                        } else {
                                            ui.label(connection);
                                        }
                                        if matches!(
                                            *key,
                                            "dpt146" | "hmd65" | "wattnode" | "ati-f12" | "bridge"
                                        ) && self.bridge_stale
                                        {
                                            ui.weak("Last good readings - stale");
                                        }
                                        let profile = profiles
                                            .iter()
                                            .find(|p| reference::profile_matches(key, &p.info.id));
                                        if let Some(registers) = reference.registers.get(*key) {
                                            for register in registers
                                                .iter()
                                                .filter(|r| r.readout_label.is_some())
                                                .take(3)
                                            {
                                                let live = direct
                                                    .filter(|d| d.key == *key)
                                                    .and_then(|d| {
                                                        d.values
                                                            .get(&register.first_pdu()?)
                                                            .copied()
                                                    })
                                                    .or_else(|| value(profile, result, register));
                                                if let Some(v) = live {
                                                    ui.label(format!(
                                                        "{}: {v:.2} {}",
                                                        register
                                                            .readout_label
                                                            .as_deref()
                                                            .unwrap_or(&register.name),
                                                        register.unit
                                                    ));
                                                }
                                            }
                                        }
                                        ui.button("View device")
                                    })
                                    .inner
                                });
                                if response.inner.clicked() {
                                    self.reference_context = false;
                                    self.page = Page::Detail((*key).into());
                                }
                            }
                        }
                    });
                    ui.add_space(20.0);
                }
                if network && let Some(result) = result {
                    self.network_readings(ui, result, profiles);
                }
            }
            Page::References => {
                ui.heading("Device references");
                ui.label("Browse supported models, register maps and setup guidance.");
                ui.weak("A reference entry does not indicate a connected device.");
                ui.add_space(20.0);
                let reference_width = (ui.available_width() - 12.0).max(0.0);
                for key in [
                    "bridge", "dpt146", "hmd65", "wattnode", "ati-f12", "iaq_plus", "adapter",
                ] {
                    let device = &reference.devices[key];
                    ui.group(|ui| {
                        ui.set_width((reference_width - 14.0).max(0.0));
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(&device.name).clicked() {
                                self.reference_context = true;
                                self.page = Page::Detail(key.into());
                            }
                            ui.label(&device.subtitle);
                        });
                        ui.weak(match key {
                            "bridge" => "Live reads, point-table program and restore tested on firmware 3.6. Full power-cycle acceptance pending.",
                            "dpt146" => "Live reads verified through the E5 bridge and the USB Modbus adapter.",
                            "hmd65" | "wattnode" | "ati-f12" => "E5 bridge preset and direct adapter profile implemented. Hardware validation pending.",
                            "iaq_plus" => "Reference only. Native IAQ identification, live reads and console backup are not implemented.",
                            _ => "USB discovery and direct DPT146 reads verified. Polling is blocked while an E5 bridge interface is present.",
                        });
                    });
                }
            }
            Page::Detail(key) => {
                return self.detail_page(
                    ui,
                    key,
                    DevicePageContext {
                        reference,
                        ports,
                        profiles,
                        result,
                        direct,
                        busy,
                    },
                );
            }
            Page::Registers(key) => {
                return self.register_page(
                    ui,
                    key,
                    DevicePageContext {
                        reference,
                        ports,
                        profiles,
                        result,
                        direct,
                        busy,
                    },
                );
            }
            Page::Radio => {
                self.header(
                    ui,
                    "Radio reference",
                    "Reference information only. Radio settings cannot be changed here.",
                    Page::Detail("iaq_plus".into()),
                );
                for region in reference.regions.values() {
                    ui.group(|ui| {
                        ui.strong(&region.display_name);
                        ui.label(format!(
                            "{} MHz · {}",
                            region.frequency_mhz,
                            if region.enabled {
                                "Reference default"
                            } else {
                                "Not supported"
                            }
                        ));
                        ui.label("Details").on_hover_text(&region.notes);
                    });
                }
            }
            Page::Configurations => {
                self.header(
                    ui,
                    "Configurations",
                    "Choose a configuration to preview and export.",
                    Page::Overview,
                );
                for key in ["dpt146", "hmd65", "wattnode", "ati-f12"] {
                    let device = &reference.devices[key];
                    let width = ui.available_width().min(560.0);
                    ui.group(|ui| {
                        ui.set_width(width - 16.0);
                        ui.heading(&device.name);
                        ui.label(&device.status);
                        if ui.button("Preview configuration").clicked() {
                            actions.push(Action::ChooseConfig(key.into()));
                        }
                    });
                }
            }
            Page::Troubleshooting(mut selected) => {
                if let Some(key) = &selected
                    && ui.button("‹ Device details").clicked()
                {
                    self.page = Page::Detail(key.clone());
                    return actions;
                }
                crate::troubleshooting::show(ui, reference, &mut selected);
                self.page = Page::Troubleshooting(selected);
            }
            Page::Console | Page::History | Page::Settings => {}
        }
        actions
    }
    fn header(&mut self, ui: &mut egui::Ui, title: &str, subtitle: &str, back: Page) {
        ui.horizontal_wrapped(|ui| {
            let back_label = match &back {
                Page::Overview => "‹ Devices",
                Page::References => "‹ References",
                Page::Detail(_) => "‹ Device details",
                _ => "‹ Back",
            };
            if ui.button(back_label).clicked() {
                self.page = back;
                self.search.clear();
            }
            ui.heading(title);
        });
        if !subtitle.is_empty() {
            ui.weak(subtitle);
        }
        ui.add_space(16.0);
    }
    fn icon(ui: &mut egui::Ui, device: &Device) {
        crate::device_art::device(ui, &device.key);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn address_notation_preserves_ranges_without_changing_wire_data() {
        assert_eq!(super::display_address("4–5", true), "5–6");
        assert_eq!(super::display_address("0", true), "1");
        assert_eq!(super::display_address("65535", true), "65536");
        assert_eq!(super::display_address("4–5", false), "4–5");
        assert_eq!(super::display_address("—", true), "—");
    }
    use super::*;
    use modbus_configurator::{bridge::Reading, contract::Identity};

    #[test]
    fn readout_mapping_uses_python_semantics_only_for_an_exact_table() {
        let reference = Reference::bundled().unwrap();
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        let dpt = &profiles[0];
        let register = reference.registers["dpt146"]
            .iter()
            .find(|r| r.readout_label.as_deref() == Some("Temperature"))
            .unwrap();
        let mut result = BridgeResult {
            identity: Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            native_tsv: dpt.native_tsv.clone(),
            readings: vec![Reading {
                item: 1,
                value: 22.5,
            }],
            successful_reads: Some(8),
            exceptions: vec![],
        };
        assert_eq!(value(Some(dpt), Some(&result), register), Some(22.5));
        assert_eq!(register.unit, "°C");
        result.native_tsv = result.native_tsv.replace("\t4\t", "\t5\t");
        assert_eq!(value(Some(dpt), Some(&result), register), None);
    }

    #[test]
    fn disconnected_device_page_keeps_value_timestamp_and_stale_label() {
        let reference = Reference::bundled().unwrap();
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        let result = BridgeResult {
            identity: Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            native_tsv: profiles[0].native_tsv.clone(),
            readings: vec![Reading {
                item: 1,
                value: 23.5,
            }],
            successful_reads: Some(1),
            exceptions: vec![],
        };
        let stamp = "2026-09-10 14:32:08 (local)";
        for page in [Page::Detail("dpt146".into()), Page::Overview] {
            let mut view = TechnicianView {
                page: page.clone(),
                bridge_stale: true,
                ..Default::default()
            };
            view.bridge_times.insert(4, stamp.into());
            let ctx = egui::Context::default();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1180.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        view.show(
                            ui,
                            &reference,
                            &[],
                            &profiles,
                            (Some(&result), "COM5", None),
                            false,
                        );
                    });
                },
            );
            fn texts(shape: &egui::epaint::Shape, text: &mut String) {
                match shape {
                    egui::epaint::Shape::Text(t) => {
                        text.push_str(t.galley.text());
                        text.push(' ');
                    }
                    egui::epaint::Shape::Vec(v) => {
                        for s in v {
                            texts(s, text);
                        }
                    }
                    _ => {}
                }
            }
            let mut text = String::new();
            for shape in &output.shapes {
                texts(&shape.shape, &mut text);
            }
            assert!(text.contains("23.5"), "{text}");
            assert!(text.to_lowercase().contains("disconnected"), "{text}");
            if matches!(page, Page::Detail(_)) {
                assert!(text.contains(stamp), "{text}");
                assert!(text.contains("Stale"), "{text}");
            }
        }
    }

    #[test]
    fn all_python_navigation_destinations_render_without_hardware_actions() {
        let reference = Reference::bundled().unwrap();
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        let mut pages = vec![
            Page::Overview,
            Page::References,
            Page::Configurations,
            Page::Radio,
        ];
        pages.extend(reference.devices.keys().map(|k| Page::Detail(k.clone())));
        pages.extend(
            reference
                .registers
                .keys()
                .map(|k| Page::Registers(k.clone())),
        );
        for size in [egui::vec2(1180.0, 760.0), egui::vec2(960.0, 640.0)] {
            for page in &pages {
                let context = egui::Context::default();
                let mut view = TechnicianView {
                    page: page.clone(),
                    ..Default::default()
                };
                let output = context.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                assert!(
                                    view.show(
                                        ui,
                                        &reference,
                                        &[],
                                        &profiles,
                                        (None, "", None),
                                        false
                                    )
                                    .is_empty()
                                );
                            });
                        });
                    },
                );
                assert!(!output.shapes.is_empty());
            }
        }
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;
    #[test]
    fn precision_preserves_real_zero_and_useful_pressure_digits() {
        assert_eq!(display_value(25.321365, "°C"), "25.32");
        assert_eq!(display_value(1.014391, "bara"), "1.014");
        assert_eq!(display_value(13069.823242, "ppmv"), "13070");
        assert_eq!(display_value(-0.0001, "°C"), "0.00");
        assert_eq!(display_value(0.0, "°C"), "0.00");
        assert_eq!(display_value(f64::NAN, "°C"), "—");
    }
    #[test]
    fn disconnected_without_history_never_claims_retained_values() {
        assert_eq!(
            reading_state(false, false, false),
            "Disconnected · no readings yet"
        );
        assert!(reading_state(false, false, true).contains("retained"));
        assert_eq!(
            reading_state(true, true, false),
            "Read failed · no readings yet"
        );
    }
}

fn register_cell(ui: &mut egui::Ui, width: f32, label: egui::Label) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            ui.add(label)
        },
    )
    .inner
}

#[cfg(test)]
#[path = "tests/custom_profile_ui.rs"]
mod custom_profile_tests;
