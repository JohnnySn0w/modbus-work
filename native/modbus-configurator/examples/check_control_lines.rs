//! Read-only recovery experiment. One handle, no configuration/menu commands.
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = std::env::args().nth(1).ok_or("COM port required")?;
    let ports = serialport::available_ports()?;
    if !ports.iter().any(|p|p.port_name==name && matches!(&p.port_type,serialport::SerialPortType::UsbPort(u) if u.vid==0x0483 && u.pid==0x5740)) {return Err("Not a Synetica console".into());}
    let mut port = serialport::new(name, 115200)
        .timeout(Duration::from_millis(100))
        .open()?;
    port.write_request_to_send(false)?;
    let mut state = modbus_configurator::console::PromptState::Unknown;
    for stage in 0..3 {
        if stage == 0 {
            port.write_data_terminal_ready(true)?;
        }
        if stage == 1 {
            if !matches!(
                state,
                modbus_configurator::console::PromptState::MainMenu
                    | modbus_configurator::console::PromptState::ModbusMenu
                    | modbus_configurator::console::PromptState::ImportExport
                    | modbus_configurator::console::PromptState::Password
            ) {
                return Err("No known menu to exit".into());
            }
            port.write_all(
                if state == modbus_configurator::console::PromptState::Password {
                    b"\r"
                } else {
                    b"X\r"
                },
            )?;
            port.flush()?;
        }
        if stage == 2 {
            port.set_baud_rate(115200)?;
        }
        let mut parser = modbus_configurator::console::ConsoleParser::default();
        let mut count = 0;
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            let _ = port.bytes_to_read()?;
            let mut bytes = [0; 64];
            match port.read(&mut bytes) {
                Ok(n) => {
                    count += n;
                    parser.feed(&bytes[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e.into()),
            }
        }
        state = parser.state();
        println!("Stage {stage}: {count} bytes; prompt {state:?}");
    }
    port.write_data_terminal_ready(false)?;
    Ok(())
}
