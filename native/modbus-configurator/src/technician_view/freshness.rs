//! Monotonic receipt ages: only successful reads reset a register's timer.
use super::*;

impl TechnicianView {
    /// Highlight only successful, connected reads; retained failed values keep their age.
    pub(super) fn reading_ok(
        &self,
        profile: Option<&Profile>,
        result: Option<&BridgeResult>,
        direct: Option<&AdapterResult>,
        register: &Register,
    ) -> bool {
        let Some(address) = register.first_pdu() else {
            return false;
        };
        if let Some(direct) = direct {
            return !self.adapter_stale
                && direct.values.get(&address).is_some_and(|v| v.is_finite())
                && !direct.errors.contains_key(&address);
        }
        if self.bridge_stale || !self.bridge_connected {
            return false;
        }
        let Some(profile) = profile else {
            return false;
        };
        let Some(result) = result.filter(|r| profile.contains_points(r)) else {
            return false;
        };
        profile.rows.keys().any(|point| {
            profile
                .point_register(*point)
                .and_then(|r| r.range().ok())
                .is_some_and(|range| range.0 == address)
                && result
                    .readings
                    .iter()
                    .any(|r| r.item == *point && r.value.is_finite())
                && !result.exceptions.iter().any(|e| e.item == *point)
        })
    }

    /// Resolve the timer through the bridge point, keeping repeated slave registers separate.
    pub(super) fn received_at(
        &self,
        profile: Option<&Profile>,
        direct: Option<&AdapterResult>,
        register: &Register,
    ) -> Option<std::time::Instant> {
        let address = register.first_pdu()?;
        if let Some(direct) = direct {
            return direct
                .values
                .contains_key(&address)
                .then(|| self.adapter_received.get(&address).copied())
                .flatten();
        }
        let profile = profile?;
        let point = profile.rows.keys().find(|point| {
            profile
                .point_register(**point)
                .and_then(|r| r.range().ok())
                .is_some_and(|range| range.0 == address)
        })?;
        self.bridge_received.get(point).copied()
    }
}

/// Never manufacture an age for a register that has not supplied a successful value.
pub(super) fn age_text(ui: &egui::Ui, received: Option<std::time::Instant>) -> String {
    received.map_or_else(String::new, |at| {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
        format!("{} s", at.elapsed().as_secs())
    })
}
