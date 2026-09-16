//! Inspect Modbus Bridge prompts while preserving their current values. The explicit
//! --verify-roundtrip option changes delay by 1 ms and restores it. Private traces
//! and before-write snapshots remain under ignored tmp/serial-optimization.
use modbus_configurator::{
    bridge::{BridgeSession, Timing},
    contract::Identity,
    transport::{self, Transport},
};
use std::{
    io,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
struct Shared(Arc<Mutex<Box<dyn Transport>>>);
impl Transport for Shared {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.0.lock().unwrap().read(b)?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("tmp/serial-optimization/line-private.trace")?;
        file.write_all(&b[..n])?;
        Ok(n)
    }
    fn write(&mut self, b: &[u8]) -> io::Result<()> {
        self.0.lock().unwrap().write(b)
    }
    fn continuous_receive(&self) -> bool {
        true
    }
    fn requires_menu_selection_prompt(&self) -> bool {
        true
    }
    fn resume_receive(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().resume_receive()
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args().nth(1).ok_or("COM port required")?;
    let shared = Arc::new(Mutex::new(transport::open(&port)?));
    let mut session = BridgeSession::new(Box::new(Shared(shared.clone())), Timing::default());
    session
        .poll_with_export(
            &Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            },
            false,
            &AtomicBool::new(false),
            |_| {},
            |_| {},
        )
        .map_err(|e| e.message)?;
    if std::env::args().any(|a| a == "--verify-roundtrip") {
        let current = session.line_settings().ok_or("No line settings")?;
        let mut target = current.clone();
        println!("Initial delay: {} ms", current.delay_ms);
        target.delay_ms = if current.delay_ms < 10000 {
            current.delay_ms + 1
        } else {
            current.delay_ms - 1
        };
        let identity = Identity {
            model: "ENL-MOD-32".into(),
            firmware: "3.6".into(),
        };
        let cancel = AtomicBool::new(false);
        for (name, desired, reviewed) in [
            ("changed", &target, &current),
            ("restored", &current, &target),
        ] {
            session
                .configure_line(
                    &identity,
                    desired,
                    reviewed,
                    &cancel,
                    |s| println!("{s}"),
                    |settings| {
                        use std::io::Write;
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos();
                        let mut file = std::fs::File::create_new(format!(
                            "tmp/serial-optimization/line-{name}-{stamp}.json"
                        ))?;
                        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
                        file.sync_all()
                    },
                )
                .map_err(|e| e.message)?;
            assert_eq!(session.line_settings().as_ref(), Some(desired));
            println!("{name}: {} ms delay", desired.delay_ms);
        }
        let readings = session
            .poll_with_export(&identity, true, &cancel, |_| {}, |_| {})
            .map_err(|e| e.message)?;
        println!(
            "After restore: {} readings, {} exceptions",
            readings.readings.len(),
            readings.exceptions.len()
        );
        return Ok(());
    }
    let mut port = shared.lock().unwrap();
    for key in ["B", "D", "P", "S", "R", "T", "I"] {
        port.write(format!("{key}\r").as_bytes())?;
        let text = read_prompt(&mut **port)?;
        println!("{text}");
        let unchanged = text
            .lines()
            .find(|line| line.contains("<=="))
            .and_then(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .or_else(|| {
                text.lines()
                    .find(|line| line.contains("Current Setting:"))
                    .and_then(|line| line.split_once('='))
                    .and_then(|(_, value)| value.split_whitespace().next())
                    .map(str::to_owned)
            })
            .ok_or("Cannot identify unchanged setting")?;
        if !unchanged.chars().all(|c| c.is_ascii_digit()) {
            return Err("Unrecognized setting prompt".into());
        }
        port.write(format!("{unchanged}\r").as_bytes())?;
        let mut returned = read_prompt(&mut **port)?;
        if returned.contains("Press a key to continue") {
            port.write(b"\r")?;
            returned = read_prompt(&mut **port)?;
        }
        if !returned.contains("Modbus Configuration Menu:") {
            println!("{returned}");
            return Err("Menu did not return".into());
        }
    }
    Ok(())
}
fn read_prompt(port: &mut dyn Transport) -> io::Result<String> {
    let start = Instant::now();
    let mut recovery = Instant::now();
    let mut parser = modbus_configurator::console::ConsoleParser::default();
    while start.elapsed() < Duration::from_secs(3) {
        let mut bytes = [0; 1024];
        match port.read(&mut bytes) {
            Ok(n) => {
                parser.feed(&bytes[..n]);
            }
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
            Err(e) => return Err(e),
        }
        if recovery.elapsed() > Duration::from_millis(500) {
            port.resume_receive()?;
            recovery = Instant::now();
        }
    }
    Ok(parser.text().to_owned())
}
