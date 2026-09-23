//! Modbus Bridge reading scenarios using the shared scripted transport.
use super::*;

#[test]
fn captured_truncated_detailed_output_is_not_accepted_as_a_complete_read() {
    let mut f = fixture();
    let (_, response) = f
        .exchanges
        .iter_mut()
        .find(|(command, _)| command == "A\r")
        .unwrap();
    *response = include_str!("../fixtures/bridge-detailed-exceptions.txt").into();
    let error = BridgeSession::new(Box::new(Script::new(f, 67)), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::InvalidResponse));
    assert!(error.message.contains("reading without a point header"));
}

#[test]
fn steady_polling_reuses_table_and_only_issues_read_commands() {
    let mut f = fixture();
    let read = f.exchanges[6..].to_vec();
    for _ in 0..20 {
        f.exchanges.extend(read.clone());
    }
    let script = Script::new(f, 67);
    let writes = script.writes.clone();
    let mut session = BridgeSession::new(Box::new(script), timing());
    let mut exports = 0;
    for _ in 0..21 {
        let r = session
            .poll_with_export(
                &identity(),
                true,
                &AtomicBool::new(false),
                |_| {},
                |_| exports += 1,
            )
            .unwrap();
        assert_eq!(r.successful_reads, Some(2));
        assert_eq!(r.dev_eui.as_deref(), Some("000000000000CAFE"));
    }
    assert_eq!(exports, 1);
    assert_eq!(writes.lock().unwrap().len(), 8 + 20 * 2);
}

#[test]
fn validated_export_is_available_for_backup_even_if_measurements_fail() {
    let mut f = fixture();
    let (_, response) = f
        .exchanges
        .iter_mut()
        .find(|(command, _)| command == "A\r")
        .unwrap();
    *response = include_str!("../fixtures/bridge-detailed-exceptions.txt").into();
    let mut backed_up = None;
    let result = BridgeSession::new(Box::new(Script::new(f, 67)), timing()).run_with_export(
        &identity(),
        true,
        &AtomicBool::new(false),
        |_| {},
        |table| backed_up = Some(table.to_owned()),
    );
    assert!(result.is_err());
    let table = backed_up.unwrap();
    assert_eq!(parse_export(&table, 2).unwrap().len(), 2);
}

#[test]
fn password_only_console_requires_redrawn_identity_before_derived_login() {
    let mut f = fixture();
    f.exchanges.insert(0, ("\r".into(), f.initial.clone()));
    f.initial = "Password:".into();
    BridgeSession::new(Box::new(Script::new(f, 32)), timing())
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn complete_mixed_read_is_returned_only_with_matching_bridge_summary() {
    let mut f = fixture();
    let index = f.exchanges.iter().position(|(c, _)| c == "A\r").unwrap();
    f.exchanges[index].1 = "--- [1] ID:1 Reg:Hold Addr:4 Data:F32 HL\n--- Reading: 23.5\n--- [3] ID:1 Reg:Hold Addr:512 Data:U16 HH\n--- Exception: [2] 'Illegal Data Address'\nModbus read completed\nPress a key to continue".into();
    f.exchanges[index + 1].1 = "Modbus Configuration Menu:\n1/1 (OK/Exceptions)".into();
    let result = BridgeSession::new(Box::new(Script::new(f.clone(), 64)), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(result.successful_reads, Some(1));
    assert_eq!(result.exceptions[0].item, 3);
    f.exchanges[index + 1].1 = "Modbus Configuration Menu:\n2/0 (OK/Exceptions)".into();
    assert!(
        BridgeSession::new(Box::new(Script::new(f, 64)), timing())
            .run(&identity(), true, &AtomicBool::new(false), |_| {})
            .is_err()
    );
}

#[test]
fn banner_without_password_does_not_send_speculative_input() {
    let mut f = fixture();
    f.initial = format!(
        "Synetica - enLink :: Wireless Sensor Networks\n{}",
        f.initial.replace("Password:", "")
    );
    f.exchanges.clear();
    let script = Script::new(f, 32);
    let writes = script.writes.clone();
    assert!(
        BridgeSession::new(Box::new(script), timing())
            .run(&identity(), false, &AtomicBool::new(false), |_| {})
            .is_err()
    );
    assert!(writes.lock().unwrap().is_empty());
}

#[test]
fn fresh_main_menu_logs_off_before_identity_and_configuration_access() {
    let mut f = fixture();
    f.exchanges.insert(0, ("X\r".into(), f.initial.clone()));
    f.initial = "enLink Main Menu:\r\nX - Exit and log off\r\nEnter Selection:".into();
    let script = Script::new(f, 16);
    let mut session = BridgeSession::new(Box::new(script), timing());
    session
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn read_all_selects_detailed_mode_when_the_device_requests_options() {
    let mut f = fixture();
    let index = f.exchanges.iter().position(|(c, _)| c == "A\r").unwrap();
    let result = f.exchanges[index].1.clone();
    f.exchanges[index].1 =
        "Read All Data Points Options:\r\nN - Normal\r\nD - Detailed\r\nEnter Selection [Normal]:"
            .into();
    f.exchanges.insert(index + 1, ("D\r".into(), result));
    BridgeSession::new(Box::new(Script::new(f, 17)), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn full_read_all_handles_arbitrary_chunks_and_sparse_point_indices() {
    for chunk in [1, 2, 17, 1024] {
        let script = Script::new(fixture(), chunk);
        let writes = script.writes.clone();
        let mut session = BridgeSession::new(Box::new(script), timing());
        let result = session
            .run(&identity(), true, &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(result.successful_reads, Some(2));
        assert_eq!(
            result
                .readings
                .iter()
                .map(|r| (r.item, r.value))
                .collect::<Vec<_>>(),
            vec![(1, 22.5), (3, 1.0)]
        );
        assert!(
            result
                .native_tsv
                .starts_with("Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\r\n")
        );
        assert!(!result.native_tsv.contains("cafe"));
        assert_eq!(writes.lock().unwrap().len(), 8);
    }
}

#[test]
fn missing_new_response_cannot_reuse_an_old_menu() {
    let mut f = fixture();
    f.exchanges[1].1.clear();
    let script = Script::new(f, 11);
    let writes = script.writes.clone();
    let error = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::Timeout));
    assert_eq!(*writes.lock().unwrap(), vec!["cafe\r", "C\r"]);
}

#[test]
fn invalid_readings_summary_and_export_are_rejected() {
    for (index, old, new) in [
        (3, "1\t1\tHold", "1\t0\tHold"),
        (3, "3\t1\tHold", "1\t1\tHold"),
        (6, "22.5", "NaN"),
        (6, "3\t1", "2\t1"),
        (7, "2/0", "1/1"),
    ] {
        let mut f = fixture();
        f.exchanges[index].1 = f.exchanges[index].1.replace(old, new);
        let error = BridgeSession::new(Box::new(Script::new(f, 5)), timing())
            .run(&identity(), true, &AtomicBool::new(false), |_| {})
            .unwrap_err();
        assert!(
            matches!(error.code, ErrorCode::InvalidResponse),
            "{}",
            error.message
        );
    }
    assert!(parse_export("1\t1\tHold\t4\tF32\tHL\t1\tInt", 2).is_err());
    assert!(
        parse_export("Press a key to continue", 0)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn delayed_output_is_awaited_without_retrying_commands() {
    let mut script = Script::new(fixture(), 1);
    script.idle_reads = 4;
    script.response_delay = 4;
    let writes = script.writes.clone();
    let result = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(result.successful_reads, Some(2));
    assert_eq!(writes.lock().unwrap().len(), 8);
}

#[test]
fn banner_eui_accepts_separators_but_rejects_incomplete_or_non_hex_identifiers() {
    use modbus_configurator::bridge::normalize_dev_eui;
    for value in [
        "00-00-00-00-00-00-ca-fe",
        "00:00:00:00:00:00:ca:fe",
        "000000000000cafe",
        "00 00 00 00 00 00 ca fe",
    ] {
        assert_eq!(
            normalize_dev_eui(value).as_deref(),
            Some("000000000000CAFE")
        );
        let mut f = fixture();
        f.initial = f.initial.replace("00-00-00-00-00-00-ca-fe", value);
        let r = BridgeSession::new(Box::new(Script::new(f, 7)), timing())
            .run(&identity(), false, &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(r.dev_eui.as_deref(), Some("000000000000CAFE"));
    }
    for invalid in [
        "",
        "cafe",
        "000000000000cafg",
        "000000000000cafe00",
        "serial=000000000000cafe",
    ] {
        assert!(normalize_dev_eui(invalid).is_none());
    }
}
