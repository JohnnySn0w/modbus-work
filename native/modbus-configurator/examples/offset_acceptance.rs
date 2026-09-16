//! Explicit bench test: save original, program a +1 address shift, then restore.
//! Uses production diagnostics and serial workflows without recording console credentials.
#[path = "../src/diagnostic_checks.rs"]
#[allow(dead_code)]
mod diagnostic_checks;
use modbus_configurator::{
    bridge::{BridgeResult, BridgeSession, Timing},
    catalog, config_file,
    contract::Identity,
    transport,
};
use std::{path::Path, sync::atomic::AtomicBool};

fn evidence(root: &Path, name: &str, result: &BridgeResult) -> Result<(), String> {
    std::fs::write(
        root.join(format!("{name}.json")),
        serde_json::to_vec_pretty(result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let findings = diagnostic_checks::bridge(result, &catalog::bundled()?, None);
    let text = findings
        .iter()
        .map(|f| {
            format!(
                "{} | {}: {}",
                if f.warning { "Review" } else { "Information" },
                f.subject,
                f.detail
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(root.join(format!("{name}-diagnostics.txt")), &text)
        .map_err(|e| e.to_string())?;
    println!(
        "{name}: {} values, {} exceptions; {} indexing warnings",
        result.readings.len(),
        result.exceptions.len(),
        findings
            .iter()
            .filter(|f| f.detail.contains("Possible zero/one"))
            .count()
    );
    Ok(())
}

fn stage(message: &str) {
    eprintln!(
        "{} | {message}",
        modbus_configurator::last_good::timestamp()
    );
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: offset_acceptance USB-serial new-evidence-directory".into());
    }
    let root = Path::new(&args[1]);
    std::fs::create_dir(root).map_err(|e| e.to_string())?;
    let ports = serialport::available_ports().map_err(|e| e.to_string())?;
    let candidates: Vec<_> = ports.iter().filter(|p| matches!(&p.port_type, serialport::SerialPortType::UsbPort(u) if u.vid == 0x0483 && u.pid == 0x5740 && u.serial_number.as_ref() == Some(&args[0]))).collect();
    if candidates.len() != 1 {
        return Err("Expected exactly one E5 bridge with the selected USB serial".into());
    }
    let port = &candidates[0].port_name;
    stage(&format!("Opening {port}"));
    let mut session = BridgeSession::new(
        transport::open(port).map_err(|e| e.to_string())?,
        Timing::default(),
    );
    let identity = Identity {
        model: "ENL-MOD-32".into(),
        firmware: "3.6".into(),
    };
    let cancel = AtomicBool::new(false);
    let original = session
        .run(&identity, false, &cancel, stage)
        .map_err(|e| e.message)?;
    config_file::save(&root.join("original.tsv"), &original.native_tsv)
        .map_err(|e| e.to_string())?;
    if modbus_configurator::network::recognize(&original.native_tsv, &catalog::bundled()?).is_none()
    {
        return Err(
            "Original table is not an unambiguous reviewed layout; no programming performed".into(),
        );
    }
    let baseline = session
        .poll_with_export(&identity, true, &cancel, stage, |_| {})
        .map_err(|e| e.message)?;
    evidence(root, "baseline", &baseline)?;
    let mut shifted = String::from("Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\r\n");
    for row in original
        .native_tsv
        .lines()
        .skip(1)
        .filter(|s| !s.is_empty())
    {
        let mut fields: Vec<_> = row.split('\t').map(str::to_owned).collect();
        fields[3] = fields[3]
            .parse::<u16>()
            .map_err(|e| e.to_string())?
            .checked_add(1)
            .ok_or("Address overflow")?
            .to_string();
        shifted.push_str(&fields.join("\t"));
        shifted.push_str("\r\n");
    }
    config_file::save(&root.join("shifted.tsv"), &shifted).map_err(|e| e.to_string())?;
    // From the first mutation onward, all test failures flow through restoration.
    let test = (|| -> Result<(), String> {
        let programmed = session
            .program(
                &identity,
                &shifted,
                &original.native_tsv,
                &cancel,
                stage,
                |table| config_file::save(&root.join("pre-test-backup.tsv"), table),
            )
            .map_err(|e| e.message)?;
        evidence(root, "shifted-programmed", &programmed)?;
        let read = session
            .poll_with_export(&identity, true, &cancel, stage, |_| {})
            .map_err(|e| e.message)?;
        evidence(root, "shifted-read", &read)?;
        if !diagnostic_checks::bridge(&read, &catalog::bundled()?, None)
            .iter()
            .any(|f| f.detail.contains("Possible zero/one"))
        {
            return Err("Production diagnostics did not report the deliberate offset".into());
        }
        Ok(())
    })();
    stage("Restoring original table regardless of test outcome");
    let restoration = (|| -> Result<(), String> {
        let current = session
            .run(&identity, false, &cancel, stage)
            .map_err(|e| e.message)?;
        let restored = session
            .program(
                &identity,
                &original.native_tsv,
                &current.native_tsv,
                &cancel,
                stage,
                |table| config_file::save(&root.join("pre-restore-backup.tsv"), table),
            )
            .map_err(|e| e.message)?;
        if catalog::table_rows(&restored.native_tsv)? != catalog::table_rows(&original.native_tsv)?
        {
            return Err("Restored table differs from original".into());
        }
        evidence(root, "restored-table", &restored)?;
        let read = session
            .poll_with_export(&identity, true, &cancel, stage, |_| {})
            .map_err(|e| e.message)?;
        evidence(root, "restored-read", &read)?;
        std::fs::write(
            root.join("RESTORED.txt"),
            "Original table restored and read back successfully.\n",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = restoration {
        return Err(format!(
            "RESTORATION NOT VERIFIED: {error}. Original backup: {}. Test result: {test:?}",
            root.join("original.tsv").display()
        ));
    }
    test
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => {
            println!("Offset detected; original table restored and verified.");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
