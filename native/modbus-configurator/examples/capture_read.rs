//! Private read-only trace capture for bench debugging. The trace can contain
//! console credentials; write only to an operator-approved private location.
use modbus_configurator::{
    bridge::{BridgeSession, Timing},
    contract::Identity,
    transport::{self, Transport},
};
use std::{
    fs::File,
    io::{self, Write},
    sync::atomic::AtomicBool,
};

struct Capture {
    inner: Box<dyn Transport>,
    file: File,
    parser: modbus_configurator::console::ConsoleParser,
    started: std::time::Instant,
}
impl Transport for Capture {
    fn continuous_receive(&self) -> bool {
        self.inner.continuous_receive()
    }
    fn requires_menu_selection_prompt(&self) -> bool {
        self.inner.requires_menu_selection_prompt()
    }
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(bytes)?;
        self.file.write_all(&bytes[..count])?;
        let state = self.parser.feed(&bytes[..count]);
        if std::env::var_os("MODBUS_TRACE_VERBOSE").is_some() {
            eprintln!(
                "Receive {} ms: {count} bytes, {state:?}",
                self.started.elapsed().as_millis()
            );
        }
        Ok(count)
    }
    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.parser.reset();
        self.inner.write(bytes)?;
        Ok(())
    }
    fn resume_receive(&mut self) -> io::Result<()> {
        eprintln!("Resuming CDC receive on existing handle");
        self.inner.resume_receive()
    }
    fn reconnect(&mut self) -> io::Result<()> {
        self.inner
            .reconnect()
            .inspect_err(|e| eprintln!("Host recovery failed: {e}"))
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err("Usage: capture_read COM-port private-new-trace-path [cycles]".into());
    }
    let port = &args[0];
    let path = &args[1];
    let cycles = args
        .get(2)
        .map_or(Ok(20), |s| s.parse::<usize>())
        .map_err(|e| e.to_string())?;
    if !(1..=1000).contains(&cycles) {
        return Err("Cycles must be 1..1000".into());
    }
    let candidate = serialport::available_ports().map_err(|e| e.to_string())?.iter().any(|p| {
        p.port_name.eq_ignore_ascii_case(port) && matches!(&p.port_type, serialport::SerialPortType::UsbPort(u) if u.vid == 0x0483 && u.pid == 0x5740)
    });
    if !candidate {
        return Err("Selected interface is not a Synetica USB console candidate".into());
    }
    let file = File::create_new(path).map_err(|e| e.to_string())?;
    let inner = transport::open(port).map_err(|e| e.to_string())?;
    let mut session = BridgeSession::new(
        Box::new(Capture {
            inner,
            file,
            parser: Default::default(),
            started: std::time::Instant::now(),
        }),
        Timing::default(),
    );
    let identity = Identity {
        model: "ENL-MOD-32".into(),
        firmware: "3.6".into(),
    };
    let cancel = AtomicBool::new(false);
    // Persist the actual native export before attempting any measurements, so a
    // later Read All failure cannot throw away a successful backup.
    let backup = session
        .poll_with_export(
            &identity,
            false,
            &cancel,
            |stage| eprintln!("{stage}"),
            |_| {},
        )
        .map_err(|e| format!("{:?}: {}", e.code, e.message))?;
    let backup_path = format!("{path}.backup.tsv");
    let mut saved = File::create_new(&backup_path).map_err(|e| e.to_string())?;
    saved
        .write_all(backup.native_tsv.as_bytes())
        .map_err(|e| e.to_string())?;
    saved.sync_all().map_err(|e| e.to_string())?;
    drop(saved);
    eprintln!("Verified native backup saved to {backup_path}");
    let mut summary = File::create_new(format!("{path}.summary.txt")).map_err(|e| e.to_string())?;
    for index in 1..=cycles {
        let started = std::time::Instant::now();
        let result = session
            .poll_with_export(
                &identity,
                true,
                &cancel,
                |stage| eprintln!("Read {index}: {stage}"),
                |_| eprintln!("Read {index}: table refreshed"),
            )
            .map_err(|e| format!("Read {index} failed: {:?}: {}", e.code, e.message))?;
        let line = format!(
            "Read {index}: {} values, {} exceptions, {} ms; temperature {:?}",
            result.readings.len(),
            result.exceptions.len(),
            started.elapsed().as_millis(),
            result
                .readings
                .iter()
                .find(|r| r.item == 1)
                .map(|r| r.value)
        );
        println!("{line}");
        writeln!(summary, "{line}").map_err(|e| e.to_string())?;
        summary.sync_all().map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
