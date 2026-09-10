//! Developer bench harness for the same service used by the GUI.
//! `cargo run --example bench_check -- inventory`
//! `cargo run --example bench_check -- read-all COM5`
use modbus_configurator::{catalog, contract::*, service::Service};
use std::{process::ExitCode, time::Duration};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (operation, port) = match args.as_slice() {
        [action] if action == "inventory" => (Operation::Inventory, None),
        [action, port] if action == "adapter-read" => (Operation::AdapterRead, Some(port.clone())),
        [action, port] if action == "read-all" => (Operation::BridgeReadAll, Some(port.clone())),
        _ => {
            return Err(
                "Usage: bench_check inventory | read-all COM-port | adapter-read COM-port".into(),
            );
        }
    };
    let expected_identity = matches!(operation, Operation::BridgeReadAll).then(|| Identity {
        model: "ENL-MOD-32".into(),
        firmware: "3.6".into(),
    });
    let service = Service::start().map_err(|e| e.to_string())?;
    service
        .send(Command {
            request_id: 1,
            port,
            expected_identity,
            operation,
        })
        .map_err(|e| e.to_string())?;
    loop {
        let event = service
            .events
            .recv_timeout(Duration::from_secs(55))
            .map_err(|e| e.to_string())?;
        match event.kind {
            EventKind::PortSnapshot { ports } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ports).map_err(|e| e.to_string())?
                );
                return Ok(());
            }
            EventKind::Progress { stage } => eprintln!("{stage}"),
            EventKind::AdapterResult { result } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
                );
                return Ok(());
            }
            EventKind::BridgeResult { result } => {
                let profiles = catalog::bundled()?;
                let dpt = profiles
                    .iter()
                    .find(|p| p.info.id == "dpt146")
                    .ok_or("DPT146 catalog entry is missing")?;
                let matches = dpt.matches(&result);
                let output = serde_json::json!({"expected_instrument": "Vaisala DPT146", "dpt146_table_matches": matches,
                    "instrument_identity_independently_verified": false, "bridge_result": result});
                println!(
                    "{}",
                    serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?
                );
                return if matches {
                    Ok(())
                } else {
                    Err("Exported table differs from the validated DPT146 configuration".into())
                };
            }
            EventKind::Error { code, message, .. } => return Err(format!("{code:?}: {message}")),
            _ => {}
        }
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
