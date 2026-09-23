//! Map cards and details distinguish sensors by slave address, never model alone.
use super::*;
use modbus_configurator::{catalog::table_rows, network};

const CUSTOM: &str = "Custom register set, please select device";

/// A display interpretation chosen by the operator, never a hardware identity claim.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum TypeChoice {
    #[default]
    Automatic,
    Custom,
    Profile(usize),
}

/// Resolve a register using its configured encoding, excluding item and slave IDs.
pub(super) fn register_for<'a>(
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
    /// Forget interpretations when a different physical Modbus Bridge supplies results.
    pub fn clear_network_session(&mut self) {
        self.network_types.clear();
        self.network_applied = None;
        self.slave_failures.clear();
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

    /// Use operator labels only for the exact table reported by the Modbus Bridge.
    fn network_selection(&mut self, table: &str, profiles: &[Profile]) -> Vec<network::Device> {
        if self
            .network_draft
            .as_ref()
            .is_some_and(|(draft, _)| table_rows(draft).ok() == table_rows(table).ok())
        {
            self.network_applied = self.network_draft.clone();
        }
        self.network_applied
            .as_ref()
            .filter(|(applied, _)| table_rows(applied).ok() == table_rows(table).ok())
            .map(|(_, devices)| devices.clone())
            .or_else(|| network::recognize(table, profiles))
            .unwrap_or_default()
    }

    /// Present one card per configured slave below its Modbus Bridge connection.
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
        let slaves: Vec<_> = rows
            .values()
            .filter_map(|r| r.split('\t').nth(1)?.parse::<u8>().ok())
            .fold(Vec::new(), |mut ids, id| {
                if !ids.contains(&id) {
                    ids.push(id);
                }
                ids
            });
        let card_width = (ui.available_width() - 14.0).max(650.0);
        ui.vertical(|ui| {
            for slave in slaves {
                let profile = self.selected_profile(&result.native_tsv, slave, &devices, profiles);
                let model = profile.map_or(CUSTOM, |p| p.info.model.as_str());
                ui.push_id(("network", slave), |ui| {
                    ui.group(|ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(card_width);
                            let key = ["dpt146", "hmd65", "wattnode", "ati-f12"]
                                .into_iter()
                                .find(|key| {
                                    profile.is_some_and(|p| {
                                        reference::profile_matches(key, &p.info.id)
                                    })
                                })
                                .unwrap_or("sensor");
                            ui.horizontal(|ui| {
                                self.slave_card_badge(ui, Some(result), slave);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("View device").clicked() {
                                            self.reference_context = false;
                                            self.page = Page::Detail(format!("slave:{slave}"));
                                        }
                                    },
                                );
                            });
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(120.0);
                                    crate::device_art::photo(ui, key, egui::vec2(120.0, 100.0));
                                    ui.weak("Configured on Modbus Bridge");
                                });
                                ui.vertical(|ui| {
                                    ui.set_width(280.0);
                                    ui.strong(model);
                                    let points: Vec<_> = rows
                                        .iter()
                                        .filter(|(_, row)| {
                                            row.split('\t')
                                                .nth(1)
                                                .and_then(|s| s.parse::<u8>().ok())
                                                == Some(slave)
                                        })
                                        .collect();
                                    if !self.bridge_connected || self.bridge_stale {
                                        ui.weak("Last good readings · stale");
                                    }
                                    ui.add_space(8.0);
                                    for (item, row) in points.iter().take(3) {
                                        let register = register_for(profile, row);
                                        let name = register.map_or_else(
                                            || format!("Point {item}"),
                                            |r| r.name.clone(),
                                        );
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
                                });
                                ui.vertical(|ui| {
                                    ui.set_width((card_width - 430.0).max(200.0));
                                    self.slave_card_advice(ui, result, slave);
                                    if profile.is_some_and(|p| p.contains_points(result)) {
                                        if let Some(notice) = &self.configuration_change {
                                            ui.colored_label(crate::brand::ORANGE, notice);
                                        }
                                        if let Some(warning) = configuration_warning(
                                            profile,
                                            Some(result),
                                            self.bridge_stale,
                                        )
                                        .filter(|_| !self.errors_acknowledged)
                                        {
                                            ui.colored_label(crate::brand::ORANGE, warning);
                                        }
                                    }
                                });
                            });
                        });
                    });
                });
            }
        });
    }

    /// Show a device summary or the separate register table for one slave.
    pub(super) fn network_detail(
        &mut self,
        ui: &mut egui::Ui,
        slave: u8,
        result: Option<&BridgeResult>,
        profiles: &[Profile],
        reference: &Reference,
        registers_only: bool,
    ) -> Vec<Action> {
        let actions = vec![];
        let devices = result
            .map(|r| self.network_selection(&r.native_tsv, profiles))
            .unwrap_or_default();
        let profile =
            result.and_then(|r| self.selected_profile(&r.native_tsv, slave, &devices, profiles));
        let model = profile.map_or(CUSTOM, |p| p.info.model.as_str());
        self.header(
            ui,
            &format!("{model} · slave {slave}"),
            "Configured on Modbus Bridge",
            if registers_only {
                Page::Detail(format!("slave:{slave}"))
            } else {
                Page::Overview
            },
        );
        let Some(result) = result else {
            ui.label("No readings yet");
            return actions;
        };
        self.slave_health(ui, result, slave);
        if !registers_only {
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
                        ui.selectable_value(
                            &mut choice,
                            TypeChoice::Automatic,
                            "Use configured type",
                        );
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
        }
        let profile = self.selected_profile(&result.native_tsv, slave, &devices, profiles);
        ui.weak("Remembered for this session and point table. Units apply only where the register definition matches. Read success does not verify device type.");
        let Ok(rows) = table_rows(&result.native_tsv) else {
            return actions;
        };
        let points: Vec<_> = rows
            .iter()
            .filter(|(_, row)| {
                row.split('\t').nth(1).and_then(|s| s.parse::<u8>().ok()) == Some(slave)
            })
            .collect();
        if points.is_empty() {
            ui.label("This slave is no longer in the Modbus Bridge point table.");
            return actions;
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
        if !registers_only && let Some(profile) = profile {
            return self.network_device_cards(ui, slave, result, profile, reference);
        }
        let highlights: Vec<_> = points
            .iter()
            .map(|(item, _)| {
                result
                    .readings
                    .iter()
                    .any(|r| r.item == **item && r.value.is_finite())
                    && !result.exceptions.iter().any(|e| e.item == **item)
                    && self.bridge_connected
                    && !self.bridge_stale
            })
            .collect();
        egui::Grid::new(("slave-readings", slave))
            .with_row_color(move |row, _| {
                row.checked_sub(1)
                    .and_then(|index| highlights.get(index))
                    .copied()
                    .filter(|live| *live)
                    .map(|_| crate::brand::CYAN.gamma_multiply(0.12))
            })
            .show(ui, |ui| {
                for heading in [
                    "Register",
                    "Value",
                    "Since last read",
                    "Last good reading",
                    "Status",
                ] {
                    ui.strong(heading);
                }
                ui.end_row();
                for (item, row) in points {
                    let register = register_for(profile, row);
                    ui.label(register.map_or_else(|| format!("Point {item}"), |r| r.name.clone()));
                    ui.label(Self::network_value(result, *item, register));
                    ui.label(freshness::age_text(
                        ui,
                        self.bridge_received.get(item).copied(),
                    ));
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
        actions
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
