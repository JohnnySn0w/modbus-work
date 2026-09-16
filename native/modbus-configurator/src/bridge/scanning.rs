//! Bounded network scans and validated partial results.
use super::*;
impl BridgeSession {
    /// Deliver verified configuration and scan progress without ending the operation.
    pub fn observe_scan(&mut self, observer: impl Fn(BridgeResult) + Send + 'static) {
        self.scan_observer = Some(Box::new(observer));
    }

    /// Reserve time for every entry's retries. This is a conservative budget, not a duration prediction.
    fn scan_budget(&self, entries: usize) -> Duration {
        if !self.scale_scan_timeout {
            return self.read_all_response;
        }
        let per_entry = self.line_settings().map_or(15_000, |line| {
            (u64::from(line.retries) + 1)
                .saturating_mul(u64::from(line.timeout_ms) + u64::from(line.delay_ms) + 1_000)
                .max(15_000)
        });
        self.read_all_response.max(Duration::from_millis(
            per_entry
                .saturating_mul(entries as u64)
                .saturating_add(30_000)
                .min(3_600_000),
        ))
    }

    /// Ignore incomplete lines and invalid headers; never fabricate missing readings.
    pub(super) fn publish_scan_progress(&mut self) {
        let (Some(table), Some(observer)) = (&self.scan_table, &self.scan_observer) else {
            return;
        };
        let text = self.parser.text();
        let Some(end) = text.rfind('\n') else {
            return;
        };
        let count = table
            .native_tsv
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .count();
        let Ok(rows) = parse_export(&table.native_tsv, count) else {
            return;
        };
        let Ok((readings, exceptions)) = parsing::point_report(&text[..=end], &rows, true) else {
            return;
        };
        if readings.is_empty() && exceptions.is_empty() {
            return;
        }
        let signature = serde_json::to_string(&(&readings, &exceptions)).unwrap_or_default();
        if signature == self.scan_signature {
            return;
        }
        self.scan_signature = signature;
        let mut snapshot = table.clone();
        snapshot.readings = readings;
        snapshot.exceptions = exceptions;
        observer(snapshot);
    }

    /// Read the configured table, publishing partial evidence before validating the final summary.
    pub(super) fn read_points(
        &mut self,
        result: &mut BridgeResult,
        cancel: &AtomicBool,
        deadline: Instant,
        progress: &mut impl FnMut(&str),
    ) -> Result<()> {
        let count = result
            .native_tsv
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count();
        let rows = parse_export(&result.native_tsv, count)?;
        if count == 0 {
            return Err(BridgeError::new(
                ErrorCode::InvalidResponse,
                "The Modbus Bridge has no configured points to read.",
            ));
        }
        progress("Reading all configured Modbus points through the Modbus Bridge");
        let slaves: std::collections::BTreeSet<_> = rows
            .values()
            .filter_map(|row| row.split('\t').nth(1))
            .collect();
        self.trace_event(&format!("Read All: {count} configured entries; slave addresses {}; host response limit {} seconds", slaves.into_iter().collect::<Vec<_>>().join(", "), self.read_all_response.as_secs()));
        if let Some(settings) = self.line_settings() {
            self.trace_event(&format!("Modbus Bridge downstream settings: {}; retries {}; sensor timeout {} ms; inter-message delay {} ms", settings.summary(), settings.retries, settings.timeout_ms, settings.delay_ms));
        } else {
            self.trace_event(
                "Modbus Bridge downstream settings unavailable in the current menu; not inferred",
            );
        }
        let timeout = self.scan_budget(count);
        let deadline = if self.scale_scan_timeout {
            deadline.max(Instant::now() + timeout * 2 + self.timing.response * 4)
        } else {
            deadline
        };
        self.trace_event(&format!(
            "Effective scan phase budget: {} seconds; automatic scaling {}",
            timeout.as_secs(),
            self.scale_scan_timeout
        ));
        self.scan_table = Some(result.clone());
        self.scan_signature.clear();
        if let Some(observer) = &self.scan_observer {
            observer(result.clone());
        }
        let scan: Result<()> = (|| {
            self.send("A", cancel, deadline, timeout)?;
            if self.parser.state() == PromptState::ReadOptions {
                self.send("D", cancel, deadline, timeout)?;
            }
            Ok(())
        })();
        self.scan_table = None;
        scan?;
        if !self
            .parser
            .text()
            .to_ascii_lowercase()
            .contains("modbus read completed")
        {
            return Err(BridgeError::new(
                ErrorCode::InvalidResponse,
                "Modbus Bridge Read All did not complete.",
            ));
        }
        if !matches!(
            self.parser.state(),
            PromptState::ReadComplete | PromptState::Continue
        ) {
            return Err(BridgeError::new(
                ErrorCode::UnsafeState,
                "Read All ended at an unexpected prompt.",
            ));
        }
        let (readings, exceptions) = parse_point_report(self.parser.text(), &rows)?;
        result.readings = readings;
        result.exceptions = exceptions;
        self.send("", cancel, deadline, self.timing.response)?;
        self.require(PromptState::ModbusMenu)?;
        verify_summary(
            self.parser.text(),
            result.readings.len(),
            result.exceptions.len(),
        )?;
        result.successful_reads = Some(result.readings.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Silent;
    impl Transport for Silent {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::TimedOut.into())
        }
        fn write(&mut self, _: &[u8]) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn automatic_budget_covers_large_failed_network_and_respects_manual_override() {
        let mut session = BridgeSession::new(Box::new(Silent), Timing::default());
        session.configure_communication(CommunicationSettings::default(), |_| {});
        assert_eq!(session.scan_budget(32), Duration::from_secs(510));
        session.parser.feed(b"Modbus Configuration Menu:\nB  - Baud Rate 19200\nD  - Data Bits 8\nP  - Parity None\nS  - Stop Bits 2\nR  - Retries 4\nT  - Timeout 5000 ms\nI  - Inter Message Delay 1000 ms\n");
        assert_eq!(session.scan_budget(32), Duration::from_secs(1150));
        session.configure_communication(
            CommunicationSettings {
                automatic_scan_budget: false,
                read_all_seconds: 23,
                ..Default::default()
            },
            |_| {},
        );
        assert_eq!(session.scan_budget(32), Duration::from_secs(23));
    }

    #[test]
    fn partial_scan_reports_only_complete_validated_points_and_deduplicates_chunks() {
        let mut session = BridgeSession::new(Box::new(Silent), Timing::default());
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        session.observe_scan(move |result| sink.lock().unwrap().push(result));
        session.scan_table = Some(BridgeResult {
            identity: Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            native_tsv: format!(
                "{HEADER}\r\n1\t3\tHold\t4\tF32\tHL\t1\tInt\r\n2\t1\tHold\t4\tF32\tHL\t1\tInt\r\n"
            ),
            readings: vec![],
            exceptions: vec![],
            successful_reads: None,
        });
        session.parser.feed(b"--- [1] ID:3 Reg:Hold Addr:4 Data:F32 HL\n--- Exception: [11] 'No response'\n--- [2] ID:1 Reg:Hold Addr:4 Data:F32 HL\n--- Reading: 2");
        session.publish_scan_progress();
        session.publish_scan_progress();
        assert_eq!(events.lock().unwrap().len(), 1);
        assert_eq!(events.lock().unwrap()[0].exceptions[0].item, 1);
        assert!(events.lock().unwrap()[0].readings.is_empty());
        session.parser.feed(b"5.5\n");
        session.publish_scan_progress();
        let snapshot = events.lock().unwrap().last().unwrap().clone();
        assert_eq!(snapshot.readings[0].value, 25.5);
        assert_eq!(snapshot.readings[0].item, 2);
        assert!(snapshot.successful_reads.is_none());
        session.parser.reset();
        session
            .parser
            .feed(b"--- [1] ID:2 Reg:Hold Addr:4 Data:F32 HL\n--- Reading: 99\n");
        session.publish_scan_progress();
        assert_eq!(
            events.lock().unwrap().len(),
            2,
            "wrong slave must not produce a measurement"
        );
    }
}
