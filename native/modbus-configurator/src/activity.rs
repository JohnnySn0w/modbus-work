//! Timestamped operation summaries without raw console or configuration payloads.
use super::*;

impl Configurator {
    /// Include the complete latest assessment even if older activity rows were trimmed.
    pub(super) fn log_text(&self) -> String {
        let mut text = self.replay_log.join("\r\n");
        if !self.communication_log.is_empty() {
            text.push_str("\r\n\r\nCommunication timeline\r\n");
            text.push_str(&self.communication_log.join("\r\n"));
        }
        if let Some((port, at, findings)) = &self.diagnostic_report {
            text.push_str(&format!(
                "\r\n\r\nCommunication and data checks | {port} | {at}\r\n"
            ));
            for finding in findings {
                text.push_str(&format!(
                    "{} | {}: {}\r\n",
                    if finding.warning {
                        "Review"
                    } else {
                        "Information"
                    },
                    finding.subject,
                    finding.detail
                ));
            }
        }
        text
    }
    pub(super) fn operation_description(operation: &Operation) -> String {
        match operation {
            Operation::BridgeReadAll => "Read Modbus Bridge measurements".into(),
            Operation::BridgeExport => "Read Modbus Bridge configuration and save backup".into(),
            Operation::BridgeNamedBackup { name } => {
                format!("Save Modbus Bridge backup named {name}")
            }
            Operation::BridgeProgram { target, .. } => format!(
                "Program Modbus Bridge: {} point entries",
                target
                    .lines()
                    .skip(1)
                    .filter(|s| !s.trim().is_empty())
                    .count()
            ),
            Operation::BridgeLineSettings { target, .. } => format!(
                "Apply line settings: {}; retries {}; timeout {} milliseconds; delay {} milliseconds",
                target.summary(),
                target.retries,
                target.timeout_ms,
                target.delay_ms
            ),
            Operation::AdapterRead => "Read sensors through USB adapter".into(),
            Operation::ClosePort => "Release serial connection".into(),
            Operation::Cancel { request_id } => format!("Cancel request {request_id}"),
            Operation::Inventory => "Discover USB interfaces".into(),
            Operation::Replay { .. } => "Replay recorded console responses".into(),
        }
    }

    /// Record correlated results and failures; omit successful background polling.
    pub(super) fn record_service_activity(&mut self, event: &Event, active: bool, scan: bool) {
        if !active && !scan {
            return;
        }
        let detail = match &event.kind {
            EventKind::Error {
                code,
                message,
                recoverable,
            } => Some(format!(
                "Error {code:?}: {message}; retry permitted: {recoverable} (recovery not confirmed)"
            )),
            EventKind::PortSnapshot { ports } if scan => {
                let previous: Vec<_> = self
                    .ports
                    .iter()
                    .map(|p| (&p.port, &p.serial_number))
                    .collect();
                let current: Vec<_> = ports.iter().map(|p| (&p.port, &p.serial_number)).collect();
                (previous != current).then(|| {
                    format!(
                        "USB interfaces changed: {}",
                        if ports.is_empty() {
                            "none".into()
                        } else {
                            ports
                                .iter()
                                .map(|p| p.port.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    )
                })
            }
            EventKind::Progress { stage } => Some(format!("Progress: {stage}")),
            EventKind::PromptState { state } => Some(format!("Console state: {state:?}")),
            EventKind::Result { message } => Some(format!("Completed: {message}")),
            EventKind::Backup { path, error } => Some(match error {
                Some(error) => format!("Backup failed: {error}"),
                None => format!(
                    "Backup saved: {}",
                    path.as_deref().unwrap_or("path unavailable")
                ),
            }),
            EventKind::LineSettings { settings } => Some(format!(
                "Read line settings: {}; retries {}; timeout {} milliseconds; delay {} milliseconds",
                settings.summary(),
                settings.retries,
                settings.timeout_ms,
                settings.delay_ms
            )),
            EventKind::BridgeSnapshot { result } => Some(format!(
                "Scan progress: {} point readings; {} point errors; verified table available{}",
                result.readings.len(),
                result.exceptions.len(),
                result
                    .exceptions
                    .iter()
                    .map(|e| {
                        let slave = result
                            .native_tsv
                            .lines()
                            .find(|row| {
                                row.split('\t').next().and_then(|n| n.parse::<u8>().ok())
                                    == Some(e.item)
                            })
                            .and_then(|row| row.split('\t').nth(1))
                            .unwrap_or("unknown");
                        format!(
                            "; slave {slave}, point {}: exception {}: {}",
                            e.item, e.code, e.message
                        )
                    })
                    .collect::<String>()
            )),
            EventKind::BridgeResult { result } => Some(format!(
                "Modbus Bridge {} firmware {}: {} configured entries; {} returned values; {} exceptions{}",
                result.identity.model,
                result.identity.firmware,
                result
                    .native_tsv
                    .lines()
                    .skip(1)
                    .filter(|s| !s.trim().is_empty())
                    .count(),
                result.readings.len(),
                result.exceptions.len(),
                result
                    .exceptions
                    .iter()
                    .map(|e| format!("; point {}: exception {}: {}", e.item, e.code, e.message))
                    .collect::<String>()
            )),
            EventKind::AdapterResult { result } => Some(format!(
                "USB adapter: {}; {} values; {} errors{}",
                result.settings.label(),
                result.values.len(),
                result.errors.len(),
                result
                    .errors
                    .iter()
                    .map(|(address, error)| format!("; register {address}: {error}"))
                    .collect::<String>()
            )),
            _ => None,
        };
        if let Some(detail) = detail {
            let message = format!(
                "Request {} | {} | {detail}",
                event.request_id,
                if self.selected.is_empty() {
                    "USB discovery"
                } else {
                    &self.selected
                }
            );
            let failure = matches!(&event.kind, EventKind::Error { .. })
                || matches!(&event.kind, EventKind::BridgeSnapshot { result } if !result.exceptions.is_empty())
                || matches!(&event.kind, EventKind::BridgeResult { result } if !result.exceptions.is_empty())
                || matches!(&event.kind, EventKind::AdapterResult { result } if !result.errors.is_empty());
            if self.auto_request && !failure && !scan {
                crate::diagnostic_log::write(&message.replace(['\r', '\n'], " "));
            } else {
                self.record_activity(message);
            }
        }
    }
}
