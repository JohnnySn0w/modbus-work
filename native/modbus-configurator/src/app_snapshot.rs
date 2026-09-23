//! Apply in-flight network evidence without releasing the active console operation.
use super::*;

impl Configurator {
    /// Count failed scans per slave, never assigning a global timeout to an unobserved sensor.
    pub(super) fn track_slave_failures(
        &mut self,
        result: &modbus_configurator::bridge::BridgeResult,
    ) {
        if result.successful_reads.is_none() {
            return;
        }
        if self
            .result
            .as_ref()
            .is_none_or(|old| old.native_tsv != result.native_tsv)
        {
            self.technician.slave_failures.clear();
        }
        let Ok(rows) = modbus_configurator::catalog::table_rows(&result.native_tsv) else {
            return;
        };
        let mut failed = std::collections::BTreeMap::<u8, bool>::new();
        for (item, row) in rows {
            let Some(slave) = row.split('\t').nth(1).and_then(|s| s.parse().ok()) else {
                continue;
            };
            *failed.entry(slave).or_default() |= result.exceptions.iter().any(|e| e.item == item);
        }
        for (slave, error) in failed {
            let count = self.technician.slave_failures.entry(slave).or_default();
            *count = if error { count.saturating_add(1) } else { 0 };
        }
    }

    /// Publish the exported table before a slow scan, preserving last-good values on the same route.
    pub(super) fn apply_bridge_snapshot(
        &mut self,
        mut result: modbus_configurator::bridge::BridgeResult,
    ) {
        if !result.readings.is_empty() || !result.exceptions.is_empty() {
            self.technician.errors_acknowledged = false;
        }
        let source = self.ports.iter().find(|p| p.port == self.selected).cloned();
        let same = self
            .bridge_source
            .as_ref()
            .zip(source.as_ref())
            .is_some_and(|(a, b)| modbus_configurator::adapter::same_device(a, b));
        let same_table = same
            && self
                .result
                .as_ref()
                .is_some_and(|old| old.native_tsv == result.native_tsv);
        if !same {
            self.technician.clear_network_session();
        }
        if !same_table {
            self.technician.bridge_times.clear();
            self.technician.bridge_point_times.clear();
            self.technician.bridge_received.clear();
            self.technician.slave_failures.clear();
        }
        if same && let Some(old) = &self.result {
            let before = self.profiles.iter().find(|p| p.contains_points(old));
            let after = self.profiles.iter().find(|p| p.contains_points(&result));
            if let (Some(before), Some(after)) = (before, after)
                && before.info.id != after.info.id
            {
                self.technician.configuration_change = Some(format!(
                    "Sensor profile changed from {} to {}. Confirm the attached sensor and configuration.",
                    before.info.model, after.info.model
                ));
            }
        }
        let at = modbus_configurator::last_good::timestamp();
        if result.readings.is_empty() && result.exceptions.is_empty() {
            self.technician.scan_seen.clear();
        }
        for reading in &result.readings {
            // A partial report repeats earlier points; retain their original receipt time.
            if reading.value.is_finite()
                && !result.exceptions.iter().any(|e| e.item == reading.item)
                && self.technician.scan_seen.insert(reading.item)
            {
                self.technician
                    .bridge_received
                    .insert(reading.item, Instant::now());
                self.technician
                    .bridge_point_times
                    .insert(reading.item, at.clone());
                if let Some(address) = result
                    .native_tsv
                    .lines()
                    .find(|row| {
                        row.split('\t').next().and_then(|n| n.parse::<u8>().ok())
                            == Some(reading.item)
                    })
                    .and_then(|row| row.split('\t').nth(3)?.parse::<u16>().ok())
                {
                    self.technician.bridge_times.insert(address, at.clone());
                }
            }
        }
        // Keep the previous failure visible until this scan supplies a new outcome for that point.
        if same_table && let Some(old) = &self.result {
            for error in &old.exceptions {
                if !result.readings.iter().any(|r| r.item == error.item)
                    && !result.exceptions.iter().any(|e| e.item == error.item)
                {
                    result.exceptions.push(error.clone());
                }
            }
        }
        if same {
            modbus_configurator::last_good::bridge(self.result.as_ref(), &mut result);
        }
        self.bridge_source = source;
        self.result = Some(result);
    }
}
