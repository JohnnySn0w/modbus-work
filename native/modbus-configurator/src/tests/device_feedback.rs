//! Receipt ages and acknowledgement preserve evidence and hardware safety.
use super::*;

#[test]
fn clear_errors_preserves_values_logs_and_programming_lock_and_new_fault_reappears() {
    let mut a = app();
    start(&mut a);
    let result = read(&a, 21.0);
    send(&mut a, EventKind::BridgeResult { result });
    a.programming_blocked = true;
    a.auto_paused = true;
    a.technician.bridge_stale = true;
    a.technician.slave_failures.insert(1, 3);
    let logs = a.replay_log.len();
    a.handle_action(technician_view::Action::ClearErrors);
    assert!(a.technician.errors_acknowledged);
    assert!(a.technician.slave_failures.is_empty());
    assert!(a.programming_blocked && a.auto_paused && a.technician.bridge_stale);
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 21.0);
    assert!(a.replay_log.len() >= logs);
    a.active = Some(42);
    send(
        &mut a,
        EventKind::Error {
            code: ErrorCode::Timeout,
            message: "New failure".into(),
            recoverable: true,
        },
    );
    assert!(!a.technician.errors_acknowledged);
}

#[test]
fn receipt_ages_distinguish_slaves_and_do_not_reset_on_repeated_snapshots_or_errors() {
    let mut a = app();
    start(&mut a);
    let devices = [
        modbus_configurator::network::Device::new(0, 1, &a.profiles),
        modbus_configurator::network::Device::new(0, 2, &a.profiles),
    ];
    let mut result = read(&a, 21.0);
    result.native_tsv = modbus_configurator::network::compose(&devices, &a.profiles).unwrap();
    result.successful_reads = None;
    send(
        &mut a,
        EventKind::BridgeSnapshot {
            result: result.clone(),
        },
    );
    let old = Instant::now() - Duration::from_secs(20);
    a.technician.bridge_received.insert(1, old);
    let second = a.profiles[0].rows.len() as u8 + 1;
    result.readings.push(Reading {
        item: second,
        value: 0.0,
    });
    send(
        &mut a,
        EventKind::BridgeSnapshot {
            result: result.clone(),
        },
    );
    assert_eq!(a.technician.bridge_received[&1], old);
    assert!(a.technician.bridge_received[&second].elapsed().as_secs() < 2);
    assert!(!a.technician.bridge_received.contains_key(&2));
    result.readings.clear();
    result.successful_reads = Some(0);
    result.exceptions = vec![PointException {
        item: 1,
        code: 11,
        message: "No response".into(),
    }];
    send(&mut a, EventKind::BridgeResult { result });
    assert_eq!(a.technician.bridge_received[&1], old);
    assert_eq!(
        a.result
            .as_ref()
            .unwrap()
            .readings
            .iter()
            .find(|r| r.item == second)
            .unwrap()
            .value,
        0.0
    );
}

#[test]
fn device_detail_adds_reading_registers_outside_the_summary_and_table_shows_age() {
    let mut a = app();
    start(&mut a);
    let mut result = read(&a, 21.0);
    result.readings.push(Reading {
        item: 5,
        value: 1.01,
    });
    send(&mut a, EventKind::BridgeResult { result });
    a.auto_paused = true;
    a.technician.page = technician_view::Page::Detail("dpt146".into());
    let ctx = egui::Context::default();
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Absolute pressure"), "{text}");
    a.technician
        .bridge_received
        .insert(1, Instant::now() - Duration::from_secs(12));
    a.technician.page = technician_view::Page::Registers("dpt146".into());
    let text = draw(&mut a, &ctx);
    assert!(
        text.contains("Since last read") && text.contains("12 s"),
        "{text}"
    );
}

#[test]
fn all_register_failures_show_connection_checks_without_claiming_proven_hardware_fault() {
    let mut a = app();
    start(&mut a);
    let mut result = read(&a, 21.0);
    result.readings.clear();
    result.successful_reads = Some(0);
    result.exceptions = a.profiles[0]
        .rows
        .keys()
        .map(|item| PointException {
            item: *item,
            code: 2,
            message: "Illegal address".into(),
        })
        .collect();
    send(&mut a, EventKind::BridgeResult { result });
    a.auto_paused = true;
    a.technician.page = technician_view::Page::Detail("dpt146".into());
    let text = draw(&mut a, &egui::Context::default());
    assert!(
        text.contains("Possible physical connection or serial line issue"),
        "{text}"
    );
    assert!(text.contains("Register configuration can also cause"));
}

#[test]
fn bridge_eui_is_copyable_hex_on_device_and_configuration_pages_and_not_reused_for_other_routes() {
    let mut a = app();
    start(&mut a);
    let mut result = read(&a, 21.0);
    result.dev_eui = Some("00-04-a3-0b-01-2d-df-57".into());
    send(&mut a, EventKind::BridgeResult { result });
    let ctx = egui::Context::default();
    for page in [
        technician_view::Page::Overview,
        technician_view::Page::Detail("bridge".into()),
        technician_view::Page::Configurations,
    ] {
        a.technician.page = page;
        let text = draw(&mut a, &ctx);
        assert!(text.contains("0004A30B012DDF57"), "{text}");
        let output = click(&mut a, &ctx, "Copy EUI");
        assert!(output.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(value) if value == "0004A30B012DDF57")));
    }
    a.ports[0].serial_number = Some("different-bridge".into());
    let text = draw(&mut a, &ctx);
    assert!(!text.contains("0004A30B012DDF57"));
    assert!(text.contains("Not available yet"));
}
