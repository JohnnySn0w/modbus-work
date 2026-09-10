//! Explicitly authorized bench programming through the production session.
use modbus_configurator::{
    bridge::{BridgeSession, Timing},
    catalog, config_file,
    contract::{Identity, PortInfo},
    transport::{self, Transport},
};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
    sync::atomic::AtomicBool,
};
struct Trace {
    inner: Box<dyn Transport>,
    file: File,
}
impl Transport for Trace {
    fn continuous_receive(&self) -> bool {
        true
    }
    fn requires_menu_selection_prompt(&self) -> bool {
        true
    }
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(b)?;
        self.file.write_all(&b[..n])?;
        Ok(n)
    }
    fn write(&mut self, b: &[u8]) -> io::Result<()> {
        self.inner.write(b)
    }
    fn resume_receive(&mut self) -> io::Result<()> {
        self.inner.resume_receive()
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "Usage: program_acceptance USB-serial dpt146|target.tsv new-private-trace-path".into(),
        );
    }
    let target = match args[1].as_str() {
        "dpt146" => {
            catalog::bundled()?
                .into_iter()
                .find(|p| p.info.id == "dpt146")
                .unwrap()
                .native_tsv
        }
        path => config_file::load(Path::new(path)).map_err(|e| e.to_string())?,
    };
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    let candidates: Vec<_> = ports.iter().filter(|p| matches!(&p.port_type,serialport::SerialPortType::UsbPort(u) if u.vid==0x0483 && u.pid==0x5740 && u.serial_number.as_ref()==Some(&args[0]))).collect();
    if candidates.len() != 1 {
        return Err("Expected exactly one bridge with that USB serial".into());
    }
    let candidate = candidates[0];
    let port = PortInfo {
        port: candidate.port_name.clone(),
        usb_vid: Some(0x0483),
        usb_pid: Some(0x5740),
        serial_number: Some(args[0].clone()),
        description: String::new(),
        identity: None,
        busy: false,
    };
    let file = File::create_new(&args[2]).map_err(|e| e.to_string())?;
    let mut session = BridgeSession::new(
        Box::new(Trace {
            inner: transport::open(&port.port).map_err(|e| e.to_string())?,
            file,
        }),
        Timing::default(),
    );
    let identity = Identity {
        model: "ENL-MOD-32".into(),
        firmware: "3.6".into(),
    };
    let cancel = AtomicBool::new(false);
    let current = session
        .run(&identity, false, &cancel, |s| eprintln!("{s}"))
        .map_err(|e| e.message)?;
    eprintln!(
        "Current table: {} points",
        current.native_tsv.lines().skip(1).count()
    );
    let result = session
        .program(
            &identity,
            &target,
            &current.native_tsv,
            &cancel,
            |s| eprintln!("{s}"),
            |table| {
                let path = config_file::backup(
                    Path::new("native/modbus-configurator/target/acceptance-backups"),
                    &port,
                    table,
                )?;
                eprintln!("Pre-write backup: {}", path.display());
                Ok(())
            },
        )
        .map_err(|e| format!("{:?}: {}", e.code, e.message))?;
    config_file::save(
        Path::new(&format!("{}.verified.tsv", args[2])),
        &result.native_tsv,
    )
    .map_err(|e| e.to_string())?;
    for n in 1..=3 {
        let read = session
            .poll_with_export(&identity, true, &cancel, |s| eprintln!("{s}"), |_| {})
            .map_err(|e| e.message)?;
        println!(
            "Read {n}: {} values, {} exceptions",
            read.readings.len(),
            read.exceptions.len()
        );
        std::fs::write(
            format!("{}.read{n}.json", args[1]),
            serde_json::to_string_pretty(&read).unwrap(),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
