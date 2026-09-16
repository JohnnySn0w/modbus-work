//! Host-side deadlines and metadata-only serial tracing for live diagnosis.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CommunicationSettings {
    pub automatic_scan_budget: bool,
    pub initial_seconds: u64,
    pub command_seconds: u64,
    pub read_all_seconds: u64,
    pub retry_seconds: u64,
}
impl Default for CommunicationSettings {
    fn default() -> Self {
        Self {
            automatic_scan_budget: true,
            initial_seconds: 20,
            command_seconds: 5,
            read_all_seconds: 180,
            retry_seconds: 30,
        }
    }
}
impl CommunicationSettings {
    /// Bound persisted or externally supplied values before creating deadlines.
    pub fn bounded(self) -> Self {
        Self {
            automatic_scan_budget: self.automatic_scan_budget,
            initial_seconds: self.initial_seconds.clamp(1, 120),
            command_seconds: self.command_seconds.clamp(1, 120),
            read_all_seconds: self.read_all_seconds.clamp(5, 900),
            retry_seconds: self.retry_seconds.clamp(5, 300),
        }
    }
}

impl BridgeSession {
    /// Called between operations, preserving the existing serial connection.
    pub fn configure_communication(
        &mut self,
        settings: CommunicationSettings,
        trace: impl Fn(&str) + Send + 'static,
    ) {
        let settings = settings.bounded();
        self.scale_scan_timeout = settings.automatic_scan_budget;
        self.initial_response = Duration::from_secs(settings.initial_seconds);
        self.read_all_response = Duration::from_secs(settings.read_all_seconds);
        self.timing.response = Duration::from_secs(settings.command_seconds);
        // Allow menu navigation and both Read All phases without a hidden 90-second cap.
        self.timing.operation = Duration::from_secs(
            settings.initial_seconds * 4
                + settings.command_seconds * 64
                + settings.read_all_seconds * 2,
        );
        self.trace = Some(Box::new(trace));
        self.trace_event(&format!("Host deadlines: initial {} seconds; command {} seconds; each Read All phase {} seconds; automatic retry {} seconds", settings.initial_seconds, settings.command_seconds, settings.read_all_seconds, settings.retry_seconds));
    }

    pub(super) fn trace_event(&self, message: &str) {
        if let Some(trace) = &self.trace {
            trace(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct PartialReply(Option<Vec<u8>>);
    impl Transport for PartialReply {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if let Some(data) = self.0.take() {
                bytes[..data.len()].copy_from_slice(&data);
                return Ok(data.len());
            }
            std::thread::sleep(Duration::from_millis(1));
            Err(io::ErrorKind::TimedOut.into())
        }
        fn write(&mut self, _: &[u8]) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn configured_deadlines_and_timeout_evidence_do_not_expose_payloads() {
        for payload in [None, Some(b"PRIVATE-RESPONSE-CONTENT".to_vec())] {
            let received = payload.is_some();
            let mut session =
                BridgeSession::new(Box::new(PartialReply(payload)), Timing::default());
            let messages = Arc::new(Mutex::new(Vec::new()));
            let sink = messages.clone();
            session.configure_communication(
                CommunicationSettings {
                    read_all_seconds: 600,
                    ..Default::default()
                },
                move |line| sink.lock().unwrap().push(line.to_owned()),
            );
            assert_eq!(session.initial_response, Duration::from_secs(20));
            assert_eq!(session.read_all_response, Duration::from_secs(600));
            assert!(session.timing.operation > session.read_all_response * 2);
            let error = session
                .receive(
                    &AtomicBool::new(false),
                    Instant::now() + Duration::from_millis(12),
                    false,
                )
                .unwrap_err();
            assert!(matches!(error.code, ErrorCode::Timeout));
            assert!(error.message.contains(if received {
                "A response arrived"
            } else {
                "No response bytes"
            }));
            let trace = messages.lock().unwrap().join("\n");
            assert!(trace.contains("Receive started"));
            assert!(trace.contains("Receive ended"));
            assert_eq!(trace.contains("First response bytes"), received);
            assert!(!trace.contains("PRIVATE-RESPONSE-CONTENT"));
        }
    }
    #[test]
    fn settings_migrate_and_bound_external_values() {
        assert_eq!(
            serde_json::from_str::<CommunicationSettings>("{}").unwrap(),
            CommunicationSettings::default()
        );
        let values = CommunicationSettings {
            automatic_scan_budget: true,
            initial_seconds: 0,
            command_seconds: u64::MAX,
            read_all_seconds: 1,
            retry_seconds: 900,
        }
        .bounded();
        assert_eq!(
            (
                values.initial_seconds,
                values.command_seconds,
                values.read_all_seconds,
                values.retry_seconds
            ),
            (1, 120, 5, 300)
        );
    }
}
