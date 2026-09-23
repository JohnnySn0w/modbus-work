//! Model-style summaries for configured and custom slave entries.
use super::network_readings::register_for;
use super::*;

impl TechnicianView {
    /// Display readings by bridge point; selecting a model never changes wire addresses or values.
    pub(super) fn network_device_cards(
        &mut self,
        ui: &mut egui::Ui,
        slave: u8,
        result: &BridgeResult,
        profile: &Profile,
        reference: &Reference,
    ) -> Vec<Action> {
        let mut actions = vec![];
        let key = ["dpt146", "hmd65", "wattnode", "ati-f12"]
            .into_iter()
            .find(|key| reference::profile_matches(key, &profile.info.id))
            .unwrap_or("sensor");
        let registers = reference.registers.get(key);
        ui.add_space(16.0);
        ui.columns(2, |columns| {
            columns[0].strong("Readings");
            columns[0].checkbox(
                &mut self.show_native,
                "Show native values alongside display units",
            );
            if let Ok(rows) = modbus_configurator::catalog::table_rows(&result.native_tsv) {
                for (item, row) in rows.iter().filter(|(_, row)| {
                    row.split('\t').nth(1).and_then(|id| id.parse::<u8>().ok()) == Some(slave)
                }) {
                    let definition = register_for(Some(profile), row);
                    let register = definition.and_then(|definition| {
                        let address = definition.range().ok()?.0;
                        registers?.iter().find(|r| r.first_pdu() == Some(address))
                    });
                    let name = definition.map_or_else(
                        || format!("Register {}", row.split('\t').nth(3).unwrap_or("?")),
                        |r| r.name.clone(),
                    );
                    let live = result
                        .readings
                        .iter()
                        .find(|r| r.item == *item && r.value.is_finite())
                        .map(|r| r.value);
                    let error = result.exceptions.iter().find(|e| e.item == *item);
                    columns[0].push_id(item, |ui| {
                        ui.group(|ui| {
                            ui.strong(name);
                            let text = if let Some(register) = register {
                                self.display_units(
                                    ui,
                                    &format!("slave:{slave}"),
                                    register.first_pdu(),
                                    live,
                                    &register.unit,
                                )
                            } else {
                                live.map_or("—".into(), |v| format!("{v} · units unknown"))
                            };
                            ui.label(RichText::new(text).size(20.0));
                            if let Some(at) = self.bridge_point_times.get(item) {
                                ui.weak(format!("Last good: {at}"));
                            }
                            if live.is_some()
                                && (self.bridge_stale || !self.bridge_connected || error.is_some())
                            {
                                ui.weak("Stale");
                            }
                            if let Some(error) = error
                                && !self.errors_acknowledged
                            {
                                ui.colored_label(crate::brand::ORANGE, &error.message);
                            }
                        });
                    });
                }
            }
            let width = columns[1].available_width().min(320.0);
            crate::device_art::photo(&mut columns[1], key, egui::vec2(width, 200.0));
            columns[1].add_space(12.0);
            if columns[1].button("Register table").clicked() {
                self.page = Page::Registers(format!("slave:{slave}"));
            }
            if columns[1]
                .add_enabled(
                    self.bridge_connected && !self.bridge_busy,
                    egui::Button::new("Read now"),
                )
                .clicked()
            {
                actions.push(Action::ReadBridge);
            }
            columns[1].strong("Device manuals");
            for (title, path) in reference::manuals(key) {
                if columns[1].button(*title).clicked() {
                    actions.push(Action::OpenArtifact((*path).into()));
                }
            }
            if columns[1].button("Setup & troubleshooting").clicked() {
                self.page = Page::Troubleshooting(Some(key.into()));
            }
        });
        actions
    }
}
