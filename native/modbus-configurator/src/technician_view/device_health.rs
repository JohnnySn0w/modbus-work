//! Attribute reported point failures to the configured slave, not to a guessed device identity.
use super::*;

impl TechnicianView {
    /// Report communication failures separately from register exceptions.
    pub(super) fn slave_health(&self, ui: &mut egui::Ui, result: &BridgeResult, slave: u8) {
        let Ok(rows) = modbus_configurator::catalog::table_rows(&result.native_tsv) else {
            return;
        };
        let errors: Vec<_> = result
            .exceptions
            .iter()
            .filter(|e| {
                rows.get(&e.item)
                    .and_then(|row| row.split('\t').nth(1))
                    .and_then(|id| id.parse::<u8>().ok())
                    == Some(slave)
            })
            .collect();
        if errors.is_empty() {
            return;
        }
        let no_response = errors.iter().any(|e| {
            let message = e.message.to_ascii_lowercase();
            e.code == 11
                || message.contains("timeout")
                || message.contains("timed out")
                || message.contains("no response")
        });
        crate::brand::badge(
            ui,
            &format!(
                "Slave {slave} · {}",
                if no_response {
                    "not responding"
                } else {
                    "register read errors"
                }
            ),
            true,
        );
        ui.label(if no_response {
            "Check this device’s power, wiring, slave address and serial settings."
        } else {
            "Review this device’s register configuration and reported exceptions."
        });
        let cycles = self.slave_failures.get(&slave).copied().unwrap_or(0);
        if cycles > 1 {
            ui.colored_label(
                crate::brand::ORANGE,
                format!("Read errors in {cycles} consecutive scans"),
            );
        }
        ui.weak(format!("{} failed register entries", errors.len()));
    }
}
