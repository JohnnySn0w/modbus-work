use modbus_configurator::bridge::{parse_export, parse_point_report};

fn rows() -> std::collections::BTreeMap<u8, String> {
    parse_export(
        "1\t1\tHold\t4\tF32\tHL\t1\tInt\n9\t1\tHold\t1032\tF32\tHL\t1\tInt",
        2,
    )
    .unwrap()
}
const MIXED: &str = "--- [ 1]  ID:1  Reg:Hold  Addr:4  Data:F32 HL\n--- Reading: 22.75\n--- [ 9]  ID:1  Reg:Hold  Addr:1032  Data:F32 HL\n--- Exception: [2] 'Illegal Data Address'\n--- [ 9]  ID:1  Reg:Hold  Addr:1032  Data:F32 HL Retry:1\n--- Exception: [2] 'Illegal Data Address'\nModbus read completed";

#[test]
fn complete_hardware_capture_has_eight_values_and_four_explicit_exceptions() {
    let rows = parse_export(
        include_str!("../../../docs/evidence/bridge-live-12-point-backup-2026-09-09.tsv"),
        12,
    )
    .unwrap();
    let (readings, exceptions) =
        parse_point_report(include_str!("fixtures/bridge-detailed-complete.txt"), &rows).unwrap();
    assert_eq!(
        readings.iter().map(|r| r.item).collect::<Vec<_>>(),
        (1..=8).collect::<Vec<_>>()
    );
    assert_eq!(
        exceptions
            .iter()
            .map(|e| (e.item, e.code))
            .collect::<Vec<_>>(),
        vec![(9, 2), (10, 2), (11, 2), (12, 2)]
    );
    assert!(readings.iter().all(|r| r.value.is_finite()));
}

#[test]
fn mixed_detailed_results_keep_readings_and_final_device_exceptions() {
    let (readings, exceptions) = parse_point_report(MIXED, &rows()).unwrap();
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].item, 1);
    assert_eq!(readings[0].value, 22.75);
    assert_eq!(exceptions.len(), 1);
    assert_eq!(exceptions[0].item, 9);
    assert_eq!(exceptions[0].code, 2);
    assert_eq!(exceptions[0].message, "Illegal Data Address");
}

#[test]
fn successful_retry_replaces_exception_without_double_counting() {
    let text = MIXED.rsplit_once("--- Exception:").unwrap().0.to_string()
        + "--- Reading: 0.0\nModbus read completed";
    let (readings, exceptions) = parse_point_report(&text, &rows()).unwrap();
    assert_eq!(readings.len(), 2);
    assert!(exceptions.is_empty());
}

#[test]
fn incomplete_duplicate_or_mismatched_results_are_rejected() {
    for text in [
        MIXED.replace("Addr:4 ", "Addr:5 "),
        MIXED.replace("22.75", "NaN"),
        MIXED.replace("22.75", "invalid"),
        MIXED.replace("--- Exception: [2]", "--- Exception: bad"),
        MIXED.replace("--- Exception: [2]", "--- Exception: [999]"),
        MIXED.replace("--- Exception: [2]", "--- Exception: [0]"),
        "--- Exception: [2] lost header\n".to_string() + MIXED,
        MIXED.replace(
            "Data:F32 HL\n--- Reading",
            "Data:F32 HL Retry:1\n--- Reading",
        ),
        MIXED.replace(" Retry:1", ""),
        MIXED.replace("--- Reading: 22.75", ""),
        "--- Reading: 0\n".to_string() + MIXED,
        MIXED.to_string() + "\n--- Reading: 0",
    ] {
        assert!(parse_point_report(&text, &rows()).is_err(), "{text}");
    }
}

#[test]
fn compact_readings_reject_malformed_indices_values_and_exception_reports() {
    for text in [
        "1: 0\n9: bad",
        "999: 0\n9: 0",
        "1: 0 extra\n9: 0",
        "1: 0\n--- Exception: [2] bad",
    ] {
        assert!(parse_point_report(text, &rows()).is_err(), "{text}");
    }
    let (values, errors) = parse_point_report("Item 1: 0\nitem 9 = -2.5", &rows()).unwrap();
    assert_eq!(
        values.iter().map(|r| r.value).collect::<Vec<_>>(),
        vec![0.0, -2.5]
    );
    assert!(errors.is_empty());
}
