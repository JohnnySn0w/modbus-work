//! USB discovery and service results; stale replies must not replace current state.
use super::*;

impl Configurator {
    /// Cancel work on a changed USB route; require review after interrupted programming.
    pub(super) fn interrupt_active(&mut self) {
        if self.queued.take().is_some() {
            self.status = "Queued action cancelled: USB interface changed.".into();
        }
        if self.programming {
            self.programming = false;
            self.programming_blocked = true;
            self.auto_paused = true;
            self.status = "Programming interrupted by USB discovery. Check the recovered console and point table before continuing.".into();
        }
        if let Some(request_id) = self.active.take() {
            self.request(Operation::Cancel { request_id }, false);
        }
    }
    /// Apply correlated service events, ignoring stale replies and quieting automatic polls.
    pub(super) fn handle_event(&mut self, event: Event) {
        let is_scan = self.scan_pending == Some(event.request_id);
        let is_active = self.active == Some(event.request_id);
        if is_active
            && let EventKind::Progress { stage } = &event.kind
            && stage.starts_with("Communication | ")
        {
            let message = format!("Request {} | {} | {stage}", event.request_id, self.selected);
            diagnostic_log::write(&message);
            if self.communication_log.len() >= 128 {
                self.communication_log.remove(0);
            }
            self.communication_log.push(format!(
                "{} | {message}",
                modbus_configurator::last_good::timestamp()
            ));
            return;
        }
        if is_active {
            let findings = match &event.kind {
                EventKind::BridgeResult { result } => Some(diagnostic_checks::bridge(
                    result,
                    &self.profiles,
                    self.line_settings.as_ref(),
                )),
                EventKind::AdapterResult { result } => {
                    Some(diagnostic_checks::adapter(result, &self.profiles))
                }
                _ => None,
            };
            if let Some(findings) = findings {
                let changed = self
                    .diagnostic_report
                    .as_ref()
                    .is_none_or(|(port, _, previous)| {
                        port != &self.selected || previous != &findings
                    });
                for finding in &findings {
                    let message = format!(
                        "Request {} | {} | Diagnostic {} | {}: {}",
                        event.request_id,
                        self.selected,
                        if finding.warning {
                            "review"
                        } else {
                            "information"
                        },
                        finding.subject,
                        finding.detail
                    );
                    if !self.auto_request || changed {
                        self.record_activity(message);
                    } else {
                        diagnostic_log::write(&message);
                    }
                }
                self.diagnostic_report = Some((
                    self.selected.clone(),
                    modbus_configurator::last_good::timestamp(),
                    findings,
                ));
            }
        }
        self.record_service_activity(&event, is_active, is_scan);
        let quiet = is_active
            && self.auto_request
            && matches!(
                &event.kind,
                EventKind::Progress { .. }
                    | EventKind::BridgeResult { .. }
                    | EventKind::AdapterResult { .. }
                    | EventKind::Result { .. }
            );
        let previous_status = self.status.clone();
        match event.kind {
            EventKind::LineSettings { settings } => {
                if is_active {
                    if self.line_draft.is_none()
                        || self.line_draft == self.line_settings
                        || self.programming
                    {
                        self.line_draft = Some(settings.clone());
                    }
                    self.line_settings = Some(settings);
                }
            }
            EventKind::PortSnapshot { ports } => {
                if !is_scan {
                    return;
                }
                let changed = self.ports.len() != ports.len()
                    || self.ports.iter().any(|old| {
                        !ports
                            .iter()
                            .any(|p| modbus_configurator::adapter::same_route(old, p))
                    });
                let selected_changed = self
                    .ports
                    .iter()
                    .find(|p| p.port == self.selected)
                    .is_some_and(|old| {
                        !ports
                            .iter()
                            .any(|p| modbus_configurator::adapter::same_route(old, p))
                    });
                let adapter_blocked =
                    self.ports.iter().any(|p| {
                        p.port == self.selected && modbus_configurator::adapter::is_adapter(p)
                    }) && ports.iter().any(modbus_configurator::adapter::is_bridge);
                if selected_changed || adapter_blocked {
                    self.line_settings = None;
                    self.line_draft = None;
                    if let Some(port) = self.ports.iter().find(|p| p.port == self.selected) {
                        self.history.gap(
                            port,
                            if selected_changed {
                                "USB disconnected or serial port changed"
                            } else {
                                "Adapter paused: E5 bridge connected"
                            },
                            &modbus_configurator::last_good::timestamp(),
                            modbus_configurator::history::now_ms(),
                        );
                    }
                    self.interrupt_active();
                    self.technician.bridge_stale = true;
                    self.technician.adapter_stale = true;
                    self.selected.clear();
                }
                if changed {
                    if !self.programming_blocked {
                        self.auto_paused = false;
                    }
                    self.last_fetch = Instant::now() - Duration::from_secs(5);
                }
                self.ports = ports;
                self.scan_pending = None;
            }
            EventKind::Error { message, code, .. } => {
                if is_active || is_scan {
                    self.queued = None;
                }
                if is_active && self.programming {
                    self.programming = false;
                    self.auto_paused = true;
                }
                if is_active && matches!(code, ErrorCode::ProgrammingUncertain) {
                    self.programming_blocked = true;
                }
                if !is_active && !is_scan {
                    return;
                }
                if is_scan {
                    self.interrupt_active();
                    self.scan_pending = None;
                    self.ports.clear();
                }
                if is_active {
                    self.active = None;
                    self.last_fetch = Instant::now()
                        + Duration::from_secs(
                            self.preferences.communication.bounded().retry_seconds - 5,
                        );
                }
                if let Some(port) = self
                    .ports
                    .iter()
                    .find(|p| p.port == self.selected)
                    .or(self.bridge_source.as_ref())
                {
                    self.history.gap(
                        port,
                        &message,
                        &modbus_configurator::last_good::timestamp(),
                        modbus_configurator::history::now_ms(),
                    );
                }
                self.technician.bridge_stale = true;
                self.technician.adapter_stale = true;
                self.status = message;
            }
            EventKind::Backup { path, error } => {
                if is_active {
                    if let Some(path) = path {
                        self.backup_path = Some(path);
                    }
                    self.backup_error = error
                            .map_or(String::new(), |e| format!("Backup could not be saved. Live readings continue; programming requires a successful backup. {e}"));
                }
            }
            EventKind::AdapterResult { mut result } => {
                if is_active {
                    self.active = None;
                    if modbus_configurator::adapter::guard(&self.ports, &result.port).is_ok() {
                        self.status = if result.errors.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "{} readings · {} register errors",
                                result.values.len(),
                                result.errors.len()
                            )
                        };
                        let at = modbus_configurator::last_good::timestamp();
                        self.history
                            .adapter(&result, &at, modbus_configurator::history::now_ms());
                        if !self.adapter.as_ref().is_some_and(|old| {
                            modbus_configurator::adapter::same_device(&old.port, &result.port)
                                && old.key == result.key
                                && old.settings.slave == result.settings.slave
                        }) {
                            self.technician.adapter_times.clear();
                        }
                        for address in result.values.keys() {
                            self.technician.adapter_times.insert(*address, at.clone());
                        }
                        let received_fresh = !result.values.is_empty();
                        modbus_configurator::last_good::adapter(self.adapter.as_ref(), &mut result);
                        self.technician.adapter_stale = false;
                        if received_fresh {
                            self.fetched_at = Some(at);
                        }
                        self.adapter = Some(result);
                        self.last_fetch = Instant::now();
                    }
                }
            }
            EventKind::BridgeResult { mut result } => {
                if is_active {
                    self.programming_blocked = false;
                    self.active = None;
                    self.status = match result.successful_reads {
                            Some(count) => format!("{count} successful reads · {} point errors", result.exceptions.len()),
                            None => "Native point table exported and checked against the E5 bridge point count.".into(),
                        };
                    if self.programming {
                        self.programming = false;
                        self.auto_paused = false;
                        self.file_message = if self.line_applying {
                            self.technician.bridge_stale = true;
                            self.line_applying = false;
                            "E5 bridge line settings applied and verified."
                        } else {
                            "E5 bridge point table programmed and verified."
                        }
                        .into();
                    }
                    if self.auto_request && result.exceptions.is_empty() {
                        self.status.clear();
                    }
                    let source = self.ports.iter().find(|p| p.port == self.selected).cloned();
                    let same = self
                        .bridge_source
                        .as_ref()
                        .zip(source.as_ref())
                        .is_some_and(|(a, b)| modbus_configurator::adapter::same_device(a, b));
                    if !same {
                        self.technician.configuration_change = None;
                        self.technician.clear_network_session();
                    }
                    if same && let Some(old) = &self.result {
                        let before = self.profiles.iter().find(|p| p.contains_points(old));
                        let after = self.profiles.iter().find(|p| p.contains_points(&result));
                        if let (Some(before), Some(after)) = (before, after)
                            && before.info.id != after.info.id
                        {
                            self.technician.configuration_change = Some(format!(
                                "Sensor profile changed from {} to {}. Possible configuration mismatch: confirm the attached sensor is {}.",
                                before.info.model, after.info.model, after.info.model
                            ));
                        }
                    }
                    if !same
                        || self
                            .result
                            .as_ref()
                            .is_some_and(|old| old.native_tsv != result.native_tsv)
                    {
                        self.technician.bridge_times.clear();
                        self.technician.bridge_point_times.clear();
                    }
                    let at = modbus_configurator::last_good::timestamp();
                    if let Some(port) = &source {
                        self.history.bridge(
                            port,
                            &result,
                            &at,
                            modbus_configurator::history::now_ms(),
                        );
                    }
                    if !result.readings.is_empty() {
                        self.fetched_at = Some(at.clone());
                    }
                    for reading in &result.readings {
                        self.technician
                            .bridge_point_times
                            .insert(reading.item, at.clone());
                        if let Some(address) = result
                            .native_tsv
                            .lines()
                            .find(|line| {
                                line.split('\t').next().and_then(|v| v.parse::<u8>().ok())
                                    == Some(reading.item)
                            })
                            .and_then(|line| line.split('\t').nth(3))
                            .and_then(|v| v.parse::<u16>().ok())
                        {
                            self.technician.bridge_times.insert(address, at.clone());
                        }
                    }
                    if same {
                        modbus_configurator::last_good::bridge(self.result.as_ref(), &mut result);
                    }
                    if result.successful_reads.is_some() {
                        self.technician.bridge_stale = false;
                    }
                    self.bridge_source = source;
                    self.result = Some(result);
                    self.last_fetch = Instant::now();
                }
            }
            EventKind::PromptState { state } => {
                let _ = state;
            }
            EventKind::Progress { stage } => {
                if is_active {
                    self.status = stage.clone();
                }
            }
            EventKind::Result { message } => {
                if is_active {
                    self.active = None;
                }
                if is_active {
                    self.status = message;
                }
            }
        }
        if quiet {
            self.status = previous_status;
        }
    }
}
