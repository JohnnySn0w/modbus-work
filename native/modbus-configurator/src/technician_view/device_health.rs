//! Attribute reported point failures to the configured slave, not to a guessed device identity.
use super::*;

impl TechnicianView {
    /// Summarize only failures attributed to this slave in the current table.
    fn slave_status(&self, result: &BridgeResult, slave: u8) -> Option<(bool, bool, usize)> {
        if self.errors_acknowledged {
            return None;
        }
        let Ok(rows) = modbus_configurator::catalog::table_rows(&result.native_tsv) else {
            return None;
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
        let points: Vec<_> = rows
            .iter()
            .filter(|(_, row)| {
                row.split('\t').nth(1).and_then(|id| id.parse::<u8>().ok()) == Some(slave)
            })
            .map(|(item, _)| *item)
            .collect();
        let all_failed = !points.is_empty()
            && points.iter().all(|item| {
                errors.iter().any(|e| e.item == *item)
                    || (result.successful_reads.is_some()
                        && !result
                            .readings
                            .iter()
                            .any(|r| r.item == *item && r.value.is_finite()))
            });
        if errors.is_empty() && !all_failed {
            return None;
        }
        let no_response = all_failed
            || errors.iter().any(|e| {
                let message = e.message.to_ascii_lowercase();
                e.code == 11
                    || message.contains("timeout")
                    || message.contains("timed out")
                    || message.contains("no response")
            });
        Some((all_failed, no_response, errors.len()))
    }

    /// Use one badge for both the slave address and its current error state.
    pub(super) fn slave_card_badge(
        &self,
        ui: &mut egui::Ui,
        result: Option<&BridgeResult>,
        slave: u8,
    ) {
        let status = result.and_then(|r| self.slave_status(r, slave));
        let label = match status {
            Some((_, true, _)) => format!("Slave {slave} · not responding"),
            Some(_) => format!("Slave {slave} · register read errors"),
            None => format!("Slave {slave}"),
        };
        crate::brand::badge(ui, &label, status.is_some());
    }

    /// Keep overview advice compact; detailed checks remain on the device page.
    pub(super) fn slave_card_advice(&self, ui: &mut egui::Ui, result: &BridgeResult, slave: u8) {
        let Some((_, no_response, failed)) = self.slave_status(result, slave) else {
            return;
        };
        ui.colored_label(
            crate::brand::ORANGE,
            if no_response {
                "Check device power, wiring, slave address and serial settings."
            } else {
                "Review register configuration and reported exceptions."
            },
        );
        ui.weak(format!("{failed} failed register entries"));
        let cycles = self.slave_failures.get(&slave).copied().unwrap_or(0);
        if cycles > 1 {
            ui.weak(format!("Read errors in {cycles} consecutive scans"));
        }
    }

    /// Report communication failures separately from register exceptions on the detail page.
    pub(super) fn slave_health(&self, ui: &mut egui::Ui, result: &BridgeResult, slave: u8) {
        let Some((all_failed, no_response, failed)) = self.slave_status(result, slave) else {
            return;
        };
        if all_failed {
            crate::brand::attention(
                ui,
                &format!("Slave {slave} · no successful register reads"),
                "Possible physical connection or serial line issue. Check device power, wiring, slave address, baud rate and parity. Register configuration can also cause read failures.",
            );
        }
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
        ui.weak(format!("{failed} failed register entries"));
    }
}
