//! Transaction boundaries for Modbus Bridge line settings; no hardware is opened.
use modbus_configurator::{
    bridge::{BridgeSession, LineSettings, Timing},
    contract::{ErrorCode, Identity},
    transport::Transport,
};
use std::{
    collections::VecDeque,
    io,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};
#[derive(serde::Deserialize)]
struct Fixture {
    initial: String,
    exchanges: Vec<(String, String)>,
}
struct Script {
    input: VecDeque<u8>,
    steps: VecDeque<(String, String)>,
    writes: Arc<Mutex<Vec<String>>>,
}
impl Transport for Script {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        if self.input.is_empty() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let n = b.len().min(self.input.len()).min(13);
        for slot in &mut b[..n] {
            *slot = self.input.pop_front().unwrap();
        }
        Ok(n)
    }
    fn write(&mut self, b: &[u8]) -> io::Result<()> {
        let command = String::from_utf8_lossy(b).into_owned();
        self.writes.lock().unwrap().push(command.clone());
        let (expected, response) = self.steps.pop_front().expect("Unexpected console write");
        assert_eq!(command, expected);
        self.input.extend(response.bytes());
        Ok(())
    }
}
fn menu(timeout: u32) -> String {
    format!(
        "Modbus Configuration Menu:\nB  - Baud Rate 19200\nD  - Data Bits 8\nP  - Parity None\nS  - Stop Bits 2\nR  - Retries 1\nT  - Timeout {timeout} ms\nI  - Inter Message Delay 150 ms\nEnter Selection:"
    )
}
#[test]
fn line_changes_require_fresh_review_and_backup_and_verify_readback() {
    for mode in ["success", "stale", "backup-fails", "bad-readback"] {
        let mut f: Fixture =
            serde_json::from_str(include_str!("fixtures/bridge-read-all.json")).unwrap();
        f.exchanges.truncate(6);
        f.exchanges[5].1 = menu(500);
        f.exchanges.extend([
            (
                "T\r".into(),
                "Current Setting: Timeout = 500 ms\nEnter a number between 10 and 20000 ms:".into(),
            ),
            (
                "600\r".into(),
                menu(if mode == "bad-readback" { 500 } else { 600 }),
            ),
        ]);
        let writes = Arc::new(Mutex::new(vec![]));
        let mut session = BridgeSession::new(
            Box::new(Script {
                input: f.initial.bytes().collect(),
                steps: f.exchanges.into(),
                writes: writes.clone(),
            }),
            Timing {
                response: Duration::from_millis(100),
                quiet: Duration::from_millis(1),
                operation: Duration::from_secs(2),
            },
        );
        let mut reviewed = LineSettings::parse(&menu(500)).unwrap();
        let mut target = reviewed.clone();
        target.timeout_ms = 600;
        if mode == "stale" {
            reviewed.delay_ms = 200;
        }
        let mut backed_up = false;
        let result = session.configure_line(
            &Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            &target,
            &reviewed,
            &AtomicBool::new(false),
            |_| {},
            |settings| {
                assert_eq!(settings.timeout_ms, 500);
                assert_eq!(writes.lock().unwrap().len(), 6);
                backed_up = true;
                if mode == "backup-fails" {
                    Err(io::Error::other("disk full"))
                } else {
                    Ok(())
                }
            },
        );
        match mode {
            "success" => {
                assert!(result.is_ok());
                assert_eq!(session.line_settings(), Some(target));
            }
            "bad-readback" => assert!(matches!(
                result.unwrap_err().code,
                ErrorCode::ProgrammingUncertain
            )),
            _ => {
                assert!(result.is_err());
                assert_eq!(writes.lock().unwrap().len(), 6);
            }
        }
        assert_eq!(backed_up, mode != "stale");
    }
}
