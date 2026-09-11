//! Map cards and details distinguish sensors by slave address, never model alone.
use super::*;
use modbus_configurator::{catalog::table_rows, network};
use std::collections::BTreeSet;

const CUSTOM: &str = "Custom register set, please select device";

/// A display interpretation chosen by the operator, never a hardware identity claim.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum TypeChoice {
    #[default]
    Automatic,
    Custom,
    Profile(usize),
}

/// Multiple slave addresses need separate entries, even for identical models.
pub(super) fn is_network(table: &str) -> bool {
    table
        .lines()
        .skip(1)
        .filter_map(|r| r.split('\t').nth(1))
        .collect::<BTreeSet<_>>()
        .len()
        > 1
}

/// Resolve a register using its configured encoding, excluding item and slave IDs.
fn register_for<'a>(
    profile: Option<&'a Profile>,
    row: &str,
) -> Option<&'a modbus_configurator::catalog::Register> {
    let profile = profile?;
    profile
        .rows
        .iter()
        .find(|(_, r)| r.split('\t').skip(2).eq(row.split('\t').skip(2)))
        .and_then(|(point, _)| profile.point_register(*point))
}

impl TechnicianView {
    /// Forget interpretations when a different physical E5 bridge supplies results.
    pub fn clear_network_session(&mut self) {
        self.network_types.clear();
        self.network_applied = None;
    }

    /// Resolve a session override independently for each slave and exact point table.
    fn selected_profile<'a>(
        &self,
        table: &str,
        slave: u8,
        devices: &[network::Device],
        profiles: &'a [Profile],
    ) -> Option<&'a Profile> {
        match self
            .network_types
            .get(&(table.to_owned(), slave))
            .copied()
            .unwrap_or_default()
        {
            TypeChoice::Automatic
                if self.saved_profiles.iter().any(|p| p.matches(table, slave)) =>
            {
                modbus_configurator::custom_profile::matching_model(
                    &self.saved_profiles,
                    table,
                    slave,
                    profiles,
                )
                .and_then(|index| profiles.get(index))
            }
            TypeChoice::Automatic => devices
                .iter()
                .find(|d| d.slave == slave)
                .and_then(|d| profiles.get(d.profile)),
            TypeChoice::Custom => None,
            TypeChoice::Profile(index) => profiles.get(index),
        }
    }

    /// Use operator labels only for the exact table reported by the E5 bridge.
    fn network_selection(&mut self, table: &str, profiles: &[Profile]) -> Vec<network::Device> {
        if self
            .network_draft
            .as_ref()
            .is_some_and(|(draft, _)| draft == table)
        {
            self.network_applied = self.network_draft.clone();
        }
        self.network_applied
            .as_ref()
            .filter(|(applied, _)| applied == table)
            .map(|(_, devices)| devices.clone())
            .or_else(|| network::recognize(table, profiles))
            .unwrap_or_default()
    }

    /// Present one card per configured slave below its E5 bridge connection.
    pub(super) fn network_readings(
        &mut self,
        ui: &mut egui::Ui,
        result: &BridgeResult,
        profiles: &[Profile],
    ) {
        let Ok(rows) = table_rows(&result.native_tsv) else {
            return;
        };
        let devices = self.network_selection(&result.native_tsv, profiles);
        ui.strong("E5 bridge · RS-485 network");
        ui.add_space(8.0);
        let slaves: BTreeSet<_> = rows
            .values()
            .filter_map(|r| r.split('\t').nth(1)?.parse::<u8>().ok())
            .collect();
        ui.horizontal_wrapped(|ui| {
            for slave in slaves {
                let profile = self.selected_profile(&result.native_tsv, slave, &devices, profiles);
                let model = profile.map_or(CUSTOM, |p| p.info.model.as_str());
                ui.push_id(("network", slave), |ui| {
                    ui.group(|ui| {
                        ui.set_width(285.0);
                        ui.set_min_height(260.0);
                        let key = ["dpt146", "hmd65", "wattnode", "ati-f12"]
                            .into_iter()
                            .find(|key| {
                                profile.is_some_and(|p| reference::profile_matches(key, &p.info.id))
                            })
                            .unwrap_or("sensor");
                        crate::device_art::device(ui, key);
                        ui.strong(format!("{model} · slave {slave}"));
                        ui.weak("Configured on E5 bridge");
                        let points: Vec<_> = rows
                            .iter()
                            .filter(|(_, row)| {
                                row.split('\t').nth(1).and_then(|s| s.parse::<u8>().ok())
                                    == Some(slave)
                            })
                            .collect();
                        let errors = result
                            .exceptions
                            .iter()
                            .filter(|e| points.iter().any(|(item, _)| **item == e.item))
                            .count();
                        if !self.bridge_connected || self.bridge_stale {
                            ui.weak("Last good readings · stale");
                        } else if errors > 0 {
                            ui.colored_label(crate::brand::ORANGE, format!("{errors} read errors"));
                        }
                        ui.add_space(8.0);
                        for (item, row) in points.iter().take(3) {
                            let register = register_for(profile, row);
                            let name = register
                                .map_or_else(|| format!("Point {item}"), |r| r.name.clone());
                            ui.label(format!(
                                "{name}: {}",
                                Self::network_value(result, **item, register)
                            ));
                        }
                        let latest = points
                            .iter()
                            .filter_map(|(item, _)| self.bridge_point_times.get(item))
                            .max();
                        ui.weak(format!(
                            "Last reading: {}",
                            latest.map_or("—", String::as_str)
                        ));
                        if ui.button("View device").clicked() {
                            self.reference_context = false;
                            self.page = Page::Detail(format!("slave:{slave}"));
                        }
                    });
                });
            }
        });
    }

    /// Show every configured point and its last-good timestamp for one slave.
    pub(super) fn network_detail(
        &mut self,
        ui: &mut egui::Ui,
        slave: u8,
        result: Option<&BridgeResult>,
        profiles: &[Profile],
    ) {
        let devices = result
            .map(|r| self.network_selection(&r.native_tsv, profiles))
            .unwrap_or_default();
        let profile =
            result.and_then(|r| self.selected_profile(&r.native_tsv, slave, &devices, profiles));
        let model = profile.map_or(CUSTOM, |p| p.info.model.as_str());
        self.header(
            ui,
            &format!("{model} · slave {slave}"),
            "Configured on E5 bridge",
            Page::Overview,
        );
        let Some(result) = result else {
            ui.label("No readings yet");
            return;
        };
        let selection_key = (result.native_tsv.clone(), slave);
        let mut choice = self
            .network_types
            .get(&selection_key)
            .copied()
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.label("Device type for this session");
            let selected = match choice {
                TypeChoice::Automatic => "Use configured type",
                TypeChoice::Custom => CUSTOM,
                TypeChoice::Profile(index) => profiles
                    .get(index)
                    .map_or(CUSTOM, |p| p.info.model.as_str()),
            };
            egui::ComboBox::from_id_salt(("session-type", slave))
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut choice, TypeChoice::Automatic, "Use configured type");
                    ui.selectable_value(&mut choice, TypeChoice::Custom, CUSTOM);
                    for (index, profile) in profiles.iter().enumerate() {
                        ui.selectable_value(
                            &mut choice,
                            TypeChoice::Profile(index),
                            &profile.info.model,
                        );
                    }
                });
        });
        self.network_types.insert(selection_key, choice);
        let selected = self
            .selected_profile(&result.native_tsv, slave, &devices, profiles)
            .and_then(|selected| profiles.iter().position(|p| p.info.id == selected.info.id));
        self.custom_profile_controls(ui, &result.native_tsv, slave, profiles, selected);
        let profile = self.selected_profile(&result.native_tsv, slave, &devices, profiles);
        ui.weak("Remembered for this session and point table. Units apply only where the register definition matches. Read success does not verify device type.");
        let Ok(rows) = table_rows(&result.native_tsv) else {
            return;
        };
        let points: Vec<_> = rows
            .iter()
            .filter(|(_, row)| {
                row.split('\t').nth(1).and_then(|s| s.parse::<u8>().ok()) == Some(slave)
            })
            .collect();
        if points.is_empty() {
            ui.label("This slave is no longer in the E5 bridge point table.");
            return;
        }
        let matched = points
            .iter()
            .filter(|(_, row)| register_for(profile, row).is_some())
            .count();
        if profile.is_some() && matched != points.len() {
            crate::brand::attention(
                ui,
                "Register definition mismatch",
                &format!(
                    "{} of {} entries match this device type. Other values remain numeric without assigned units.",
                    matched,
                    points.len()
                ),
            );
        }
        egui::Grid::new(("slave-readings", slave))
            .striped(true)
            .show(ui, |ui| {
                for heading in ["Register", "Value", "Last good reading", "Status"] {
                    ui.strong(heading);
                }
                ui.end_row();
                for (item, row) in points {
                    let register = register_for(profile, row);
                    ui.label(register.map_or_else(|| format!("Point {item}"), |r| r.name.clone()));
                    ui.label(Self::network_value(result, *item, register));
                    ui.label(
                        self.bridge_point_times
                            .get(item)
                            .map_or("—", String::as_str),
                    );
                    let error = result.exceptions.iter().find(|e| e.item == *item);
                    let status = error.map_or_else(
                        || {
                            if !result
                                .readings
                                .iter()
                                .any(|r| r.item == *item && r.value.is_finite())
                            {
                                "No reading".into()
                            } else if self.bridge_stale || !self.bridge_connected {
                                "Stale".into()
                            } else {
                                "Read OK".into()
                            }
                        },
                        |e| format!("Exception {} · {}", e.code, e.message),
                    );
                    ui.label(status);
                    ui.end_row();
                }
            });
    }

    /// Format native values without substituting zero for missing data.
    fn network_value(
        result: &BridgeResult,
        item: u8,
        register: Option<&modbus_configurator::catalog::Register>,
    ) -> String {
        result
            .readings
            .iter()
            .find(|r| r.item == item && r.value.is_finite())
            .map_or_else(
                || "—".into(),
                |v| {
                    let unit = register.map_or("", |r| r.units.as_str());
                    if register.is_none() {
                        format!("{} · units unknown", display_value(v.value, unit))
                    } else {
                        format!("{} {unit}", display_value(v.value, unit))
                    }
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_types_are_scoped_and_units_require_complete_encoding_match() {
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        let mut device = network::Device::new(0, 1, &profiles);
        device.points = device.points.into_iter().take(1).collect();
        let table = network::compose(&[device], &profiles).unwrap();
        let mut view = TechnicianView::default();
        view.network_types
            .insert((table.clone(), 1), TypeChoice::Profile(0));
        let profile = view.selected_profile(&table, 1, &[], &profiles);
        let row = table.lines().nth(1).unwrap();
        assert!(register_for(profile, row).is_some());
        assert!(register_for(profile, &row.replace("\tF32\t", "\tS32\t")).is_none());
        assert!(view.selected_profile(&table, 2, &[], &profiles).is_none());
        assert!(
            view.selected_profile(&(table.clone() + "\n"), 1, &[], &profiles)
                .is_none()
        );
        view.clear_network_session();
        assert!(view.selected_profile(&table, 1, &[], &profiles).is_none());
    }

    #[test]
    fn draft_edits_do_not_relabel_the_applied_network() {
        let profiles = modbus_configurator::catalog::bundled().unwrap();
        let applied: Vec<_> = (1..=2)
            .map(|slave| network::Device::new(0, slave, &profiles))
            .collect();
        let table = network::compose(&applied, &profiles).unwrap();
        let mut view = TechnicianView {
            network_draft: Some((table.clone(), applied.clone())),
            ..Default::default()
        };
        assert_eq!(view.network_selection(&table, &profiles), applied);
        let draft = vec![network::Device::new(1, 1, &profiles)];
        let draft_table = network::compose(&draft, &profiles).unwrap();
        view.network_draft = Some((draft_table.clone(), draft.clone()));
        assert_eq!(view.network_selection(&table, &profiles), applied);
        assert_eq!(view.network_selection(&draft_table, &profiles), draft);
    }
}
