use modbus_configurator::{
    adapter::{AdapterResult, Settings},
    contract::PortInfo,
    history::{History, LIMIT},
};
use std::collections::BTreeMap;
fn result() -> AdapterResult {
    AdapterResult {
        port: PortInfo {
            port: "COM41".into(),
            usb_vid: Some(1),
            usb_pid: Some(2),
            serial_number: Some("unit".into()),
            description: String::new(),
            identity: None,
            busy: false,
        },
        key: "dpt146".into(),
        settings: Settings {
            slave: 1,
            even: false,
            two_stops: true,
        },
        family_only: false,
        values: BTreeMap::from([(4, 0.0), (6, 21.5)]),
        errors: BTreeMap::new(),
    }
}
#[test]
fn adapter_errors_override_cached_values_and_nonfinite_values_are_gaps() {
    let mut r = result();
    r.errors.insert(6, "Timeout, retry later".into());
    r.values.insert(8, f64::NAN);
    r.values.insert(10, f64::INFINITY);
    r.errors.insert(12, "Missing".into());
    let mut h = History::default();
    h.adapter(&r, "time", 1);
    assert_eq!(h.samples.len(), 5);
    assert_eq!(h.samples[0].value, Some(0.0));
    assert!(
        h.samples
            .iter()
            .skip(1)
            .all(|s| s.value.is_none() && s.status != "Good")
    );
    assert_eq!(h.samples[1].status, "Timeout, retry later");
}
#[test]
fn history_limit_discards_oldest_and_gaps_do_not_cross_devices() {
    let mut r = result();
    r.values.remove(&6);
    let mut h = History::default();
    for n in 0..=LIMIT {
        h.adapter(&r, "time", n as u64);
    }
    assert_eq!(h.samples.len(), LIMIT);
    assert_eq!(h.discarded, 1);
    assert_eq!(h.samples.front().unwrap().unix_ms, 1);
    let mut other = r.port.clone();
    other.serial_number = Some("other".into());
    h.gap(&other, "Lost", "time", 60000);
    assert_eq!(h.discarded, 1);
    h.gap(&r.port, "Lost", "time", 60001);
    assert_eq!(h.discarded, 2);
    assert_eq!(h.samples.back().unwrap().value, None);
}
#[test]
fn changed_adapter_slave_and_unknown_usb_identity_are_not_merged() {
    let mut r = result();
    let mut h = History::default();
    h.adapter(&r, "time", 1);
    r.settings.slave = 2;
    h.adapter(&r, "time", 2);
    assert_ne!(h.samples[0].definition, h.samples[2].definition);
    r.port.serial_number = None;
    h.adapter(&r, "time", 3);
    assert!(h.samples[4].source.contains("Unverified"));
    assert_ne!(h.samples[0].source, h.samples[4].source);
}
fn folder(name: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("coverage-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn csv_round_trip_preserves_zero_missing_and_quoted_status_and_replaces_file() {
    let p = folder("export").join("readings.csv");
    std::fs::write(&p, "old export").unwrap();
    let mut r = result();
    r.errors.insert(6, "Timeout, \"retry\"\nnext cycle".into());
    let mut h = History::default();
    h.adapter(&r, "2026-09-10 14:00:00 (local)", 100);
    h.export(&p).unwrap();
    let mut reader = csv::Reader::from_path(&p).unwrap();
    let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(&rows[0][4], "0");
    assert_eq!(&rows[1][4], "");
    assert_eq!(&rows[1][5], "Timeout, \"retry\"\nnext cycle");
    assert_eq!(std::fs::read_dir(p.parent().unwrap()).unwrap().count(), 1);
}
#[test]
fn failed_export_preserves_destination_and_removes_owned_temporary_file() {
    let dir = folder("failed-export");
    let target = dir.join("existing-directory");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("keep"), "unchanged").unwrap();
    let mut h = History::default();
    h.adapter(&result(), "time", 1);
    assert!(h.export(&target).is_err());
    assert_eq!(
        std::fs::read_to_string(target.join("keep")).unwrap(),
        "unchanged"
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    let missing = dir.join("missing-parent").join("history.csv");
    assert!(h.export(&missing).is_err());
    assert!(!missing.exists());
}
