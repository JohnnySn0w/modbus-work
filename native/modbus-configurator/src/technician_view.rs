//! Device navigation and shared readout presentation.
mod custom_profiles;
mod device_health;
mod device_pages;
mod freshness;
mod network_device;
mod network_readings;
mod overview;
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
    ClearErrors,
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
    pub bridge_busy: bool,
    pub bridge_polling: bool,
    pub bridge_fault: bool,
    pub errors_acknowledged: bool,
    pub bridge_received: std::collections::BTreeMap<u8, std::time::Instant>,
    pub adapter_received: std::collections::BTreeMap<u16, std::time::Instant>,
    pub slave_failures: std::collections::BTreeMap<u8, u32>,
    pub scan_seen: std::collections::BTreeSet<u8>,
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
    profile.rows.keys().find_map(|item| {
        let pdu = profile.point_register(*item)?.range().ok()?.0;
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
    let implausible = profile.map_or(0, |p| {
        result
            .readings
            .iter()
            .filter(|reading| {
                let Some(register) = p.point_register(reading.item) else {
                    return false;
                };
                match register.units.as_str() {
                    "°C" | "deg C" => !(-150.0..=200.0).contains(&reading.value),
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
            if ui.small_button(format!("Units: {}", options[selected].0)).on_hover_text("Cycle display units. Native readings, history and Modbus Bridge settings are unchanged.").clicked() {
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
    /// Route navigation to the relevant page without opening serial connections.
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
        let mut actions = vec![];
        match self.page.clone() {
            Page::Overview => {
                return self.overview(
                    ui,
                    DevicePageContext {
                        reference,
                        ports,
                        profiles,
                        result,
                        direct,
                        busy,
                    },
                    result_port,
                );
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
                            "dpt146" => "Live reads verified through the Modbus Bridge and the USB Modbus adapter.",
                            "hmd65" | "wattnode" | "ati-f12" => "Modbus Bridge preset and direct adapter profile implemented. Hardware validation pending.",
                            "iaq_plus" => "Reference only. Native IAQ identification, live reads and console backup are not implemented.",
                            _ => "USB discovery and direct DPT146 reads verified. Polling is blocked while a Modbus Bridge interface is present.",
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
    fn polling_is_quiet_but_partial_errors_and_stale_values_warn() {
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        let profile = profiles
            .iter()
            .find(|p| p.point_register(1).is_some_and(|r| r.units == "deg C"))
            .unwrap();
        let mut result = BridgeResult {
            dev_eui: None,
            identity: Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            native_tsv: profile.native_tsv.clone(),
            readings: vec![],
            successful_reads: None,
            exceptions: vec![],
        };
        assert!(configuration_warning(Some(profile), Some(&result), false).is_none());
        assert!(configuration_warning(Some(profile), Some(&result), true).is_some());
        result
            .exceptions
            .push(modbus_configurator::bridge::PointException {
                item: 1,
                code: 11,
                message: "No response".into(),
            });
        assert!(
            configuration_warning(Some(profile), Some(&result), false)
                .unwrap()
                .contains("1 point errors")
        );
        result.exceptions.clear();
        result.readings.push(Reading {
            item: 1,
            value: 900.0,
        });
        assert!(
            configuration_warning(Some(profile), Some(&result), false)
                .unwrap()
                .contains("1 implausible")
        );
    }

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
            dev_eui: None,
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
            dev_eui: None,
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
