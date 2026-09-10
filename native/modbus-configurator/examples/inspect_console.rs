use modbus_configurator::{console::ConsoleParser, transport};
use std::{
    io,
    time::{Duration, Instant},
};
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let serial = args.first().filter(|s| !s.is_empty()).ok_or_else(|| {
        io::Error::other("Usage: inspect_console USB-serial [finish-owned-import]")
    })?;
    let ports = serialport::available_ports()?;
    let candidates: Vec<_> = ports.iter().filter(|p| matches!(&p.port_type, serialport::SerialPortType::UsbPort(u) if u.vid==0x0483 && u.pid==0x5740 && u.serial_number.as_ref()==Some(serial))).collect();
    if candidates.len() != 1 {
        return Err(io::Error::other(
            "Expected exactly one bridge with that USB serial",
        ));
    }
    let mut port = transport::open(&candidates[0].port_name)?;
    if args.get(1).is_some_and(|s| s == "finish-owned-import") {
        port.write(b"\r")?;
    }
    let mut parser = ConsoleParser::default();
    let started = Instant::now();
    let mut recovery = Instant::now();
    let mut buf = [0; 1024];
    while started.elapsed() < Duration::from_secs(8) {
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                parser.feed(&buf[..n]);
            }
            Ok(_) => break,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(e) => return Err(e),
        }
        if recovery.elapsed() > Duration::from_millis(500) {
            port.resume_receive()?;
            recovery = Instant::now();
        }
    }
    println!("State: {:?}", parser.state());
    for line in parser.text().lines() {
        if !line.to_ascii_lowercase().contains("deveui") {
            println!("{line}");
        }
    }
    Ok(())
}
