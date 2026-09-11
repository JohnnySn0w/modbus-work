//! Edit downstream E5 bridge line settings without changing USB console framing.
use super::*;
impl Configurator {
    pub(super) fn line_configuration(&mut self, ui: &mut egui::Ui, source: Option<&PortInfo>) {
        crate::brand::collapsing(ui, "RS-485 line settings", |ui| {
            ui.label("Shared by every sensor on the E5 bridge's RS-485 bus. These settings are separate from the point-table file.");
            if ui
                .add_enabled(
                    self.ports
                        .iter()
                        .any(modbus_configurator::adapter::is_bridge)
                        && !self.foreground_busy(),
                    egui::Button::new("Read line settings"),
                )
                .clicked()
                && self.select_bridge_route()
            {
                self.hardware(Operation::BridgeExport);
            }
            let Some(current) = self.line_settings.clone().filter(|_| source.is_some()) else {
                ui.weak("Waiting for line settings from the E5 bridge.");
                return;
            };
            ui.weak(format!("On E5 bridge: {}", current.summary()));
            let busy = self.foreground_busy() || self.programming_blocked;
            let draft = self.line_draft.get_or_insert(current.clone());
            egui::Grid::new("bridge-line-settings")
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Baud rate");
                    egui::ComboBox::from_id_salt("bridge-baud")
                        .selected_text(draft.baud.to_string())
                        .show_ui(ui, |ui| {
                            for rate in [2400, 4800, 9600, 14400, 19200, 38400, 56000, 57600] {
                                ui.selectable_value(&mut draft.baud, rate, rate.to_string());
                            }
                        });
                    ui.end_row();
                    ui.label("Data bits");
                    ui.add(egui::DragValue::new(&mut draft.data_bits).range(7..=8));
                    ui.end_row();
                    ui.label("Parity");
                    egui::ComboBox::from_id_salt("bridge-parity")
                        .selected_text(&draft.parity)
                        .show_ui(ui, |ui| {
                            for value in ["None", "Odd", "Even"] {
                                ui.selectable_value(&mut draft.parity, value.into(), value);
                            }
                        });
                    ui.end_row();
                    ui.label("Stop bits");
                    egui::ComboBox::from_id_salt("bridge-stop-bits")
                        .selected_text(&draft.stop_bits)
                        .show_ui(ui, |ui| {
                            for value in ["1", "1.5", "2"] {
                                ui.selectable_value(&mut draft.stop_bits, value.into(), value);
                            }
                        });
                    ui.end_row();
                    for (label, value, range) in [
                        ("Retries", &mut draft.retries, 0..=10),
                        ("Response timeout (ms)", &mut draft.timeout_ms, 10..=20000),
                        ("Inter-message delay (ms)", &mut draft.delay_ms, 5..=10000),
                    ] {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value).range(range));
                        ui.end_row();
                    }
                });
            let changed = *draft != current;
            let target = draft.clone();
            if changed {
                ui.label("The connected sensors must use matching baud rate, data bits, parity and stop bits.");
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(changed && !busy, egui::Button::new("Apply line settings"))
                    .clicked()
                    && let Some(source) = source
                {
                    self.selected = source.port.clone();
                    self.hardware(Operation::BridgeLineSettings {
                        target,
                        reviewed: current.clone(),
                    });
                }
                if ui
                    .add_enabled(changed && !busy, egui::Button::new("Discard line changes"))
                    .clicked()
                {
                    self.line_draft = Some(current);
                }
            });
        });
    }
}
