use super::*;

#[test]
fn captured_dpt146_offset_reports_layout_small_floats_and_illegal_address() {
    let read: BridgeResult = serde_json::from_str(include_str!(
        "../../tests/fixtures/dpt146-offset-plus-one.json"
    ))
    .unwrap();
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    let findings = bridge(&read, &profiles, None);
    assert_eq!(read.readings.len(), 7);
    assert_eq!(
        findings
            .iter()
            .filter(|f| f.detail.contains("Possible zero/one"))
            .count(),
        1
    );
    assert_eq!(
        findings
            .iter()
            .filter(|f| f.detail.contains("subnormal"))
            .count(),
        4
    );
    assert!(
        findings
            .iter()
            .any(|f| f.subject == "Slave 1 · point 8" && f.detail.contains("Exception 2"))
    );
}
use modbus_configurator::{
    bridge::{PointException, Reading},
    contract::Identity,
    network::{self, Device},
};

fn result(table: String) -> BridgeResult {
    BridgeResult {
        dev_eui: None,
        identity: Identity {
            model: "E5".into(),
            firmware: "3.6".into(),
        },
        native_tsv: table,
        readings: vec![],
        successful_reads: None,
        exceptions: vec![],
    }
}

#[test]
fn indexing_checks_preserve_hmd65_convention_and_separate_partial_slaves() {
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    for id in ["hmd65", "hmd65-nonmetric", "dpt146"] {
        let index = profiles.iter().position(|p| p.info.id == id).unwrap();
        let mut device = Device::new(index, 7, &profiles);
        device.points = device.points.into_iter().take(3).collect();
        let mut other = device.clone();
        other.slave = 9;
        let table = network::compose(&[device, other], &profiles).unwrap();
        assert!(
            !bridge(&result(table.clone()), &profiles, None)
                .iter()
                .any(|f| f.warning)
        );
        for offset in [-1, 1] {
            let shifted = table
                .lines()
                .enumerate()
                .map(|(i, row)| {
                    if i == 0 {
                        return row.to_owned();
                    }
                    let mut fields: Vec<_> = row.split('\t').map(str::to_owned).collect();
                    if fields[1] == "9" {
                        fields[3] = (fields[3].parse::<i32>().unwrap() + offset).to_string();
                    }
                    fields.join("\t")
                })
                .collect::<Vec<_>>()
                .join("\n");
            let findings = bridge(&result(shifted), &profiles, None);
            assert!(findings.iter().any(
                |f| f.subject.starts_with("Slave 9") && f.detail.contains("Possible zero/one")
            ));
            assert!(
                !findings
                    .iter()
                    .any(|f| f.subject.starts_with("Slave 7") && f.warning)
            );
        }
    }
}

#[test]
fn physical_checks_do_not_reject_zero_or_assign_units_to_unknown_tables() {
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    let rh = profiles
        .iter()
        .find(|p| p.info.id == "hmd65")
        .unwrap()
        .point_register(1)
        .unwrap();
    assert!(value_check(0.0, Some(rh), true).is_none());
    assert!(value_check(100.0, Some(rh), true).is_none());
    assert!(
        value_check(101.0, Some(rh), true)
            .unwrap()
            .contains("physical bounds")
    );
    assert!(
        value_check(1e-44, Some(rh), true)
            .unwrap()
            .contains("subnormal")
    );
    assert!(value_check(101.0, None, true).is_none());
    assert!(
        value_check(f64::NAN, None, true)
            .unwrap()
            .contains("Non-finite")
    );
}

#[test]
fn missing_and_failed_values_are_separate_from_serial_evidence() {
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    let p = profiles.iter().find(|p| p.info.id == "dpt146").unwrap();
    let mut r = result(p.native_tsv.clone());
    r.successful_reads = Some(1);
    r.readings.push(Reading {
        item: 1,
        value: -300.0,
    });
    r.exceptions.push(PointException {
        item: 2,
        code: 2,
        message: "Illegal address".into(),
    });
    let settings = LineSettings {
        baud: 2400,
        data_bits: 7,
        parity: "Even".into(),
        stop_bits: "2".into(),
        retries: 1,
        timeout_ms: 10,
        delay_ms: 150,
    };
    let findings = bridge(&r, &profiles, Some(&settings));
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("physical bounds"))
    );
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("illegal address"))
    );
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("No value or exception"))
    );
    assert!(
        findings
            .iter()
            .any(|f| f.detail.contains("requires eight data bits"))
    );
    assert!(
        findings
            .iter()
            .any(|f| f.subject == "Response timeout" && f.warning)
    );
}
