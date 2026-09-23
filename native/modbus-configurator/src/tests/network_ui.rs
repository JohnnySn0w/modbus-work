//! Mode changes preserve drafts and invalid networks cannot queue programming.
use super::*;

#[test]
fn adapter_failure_does_not_latch_bridge_recovery_or_pause_adapter_polling() {
    let mut a = app();
    start(&mut a);
    let adapter = adapter_result(1).port;
    a.selected = adapter.port.clone();
    a.ports = vec![adapter];
    send(
        &mut a,
        EventKind::Error {
            code: ErrorCode::Transport,
            message: "Adapter disconnected".into(),
            recoverable: true,
        },
    );
    assert!(!a.technician.bridge_fault);
    assert!(!a.auto_paused);
}

#[test]
fn failed_slave_stays_flagged_until_fresh_evidence_clears_it() {
    let mut a = app();
    start(&mut a);
    a.auto_request = true;
    let mut failed = read(&a, 20.0);
    failed.readings.clear();
    failed.successful_reads = Some(0);
    failed.exceptions = vec![PointException {
        item: 1,
        code: 11,
        message: "No response".into(),
    }];
    for _ in 0..2 {
        a.active = Some(42);
        send(
            &mut a,
            EventKind::BridgeResult {
                result: failed.clone(),
            },
        );
    }
    assert_eq!(a.technician.slave_failures.get(&1), Some(&2));
    a.last_scan = Instant::now();
    let ctx = egui::Context::default();
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Slave 1 · not responding"));
    assert!(text.contains("2 consecutive scans"));
    a.active = Some(42);
    let mut partial = failed.clone();
    partial.successful_reads = None;
    partial.exceptions.clear();
    send(
        &mut a,
        EventKind::BridgeSnapshot {
            result: partial.clone(),
        },
    );
    assert_eq!(a.result.as_ref().unwrap().exceptions.len(), 1);
    partial.readings.push(Reading {
        item: 1,
        value: 0.0,
    });
    send(
        &mut a,
        EventKind::BridgeSnapshot {
            result: partial.clone(),
        },
    );
    assert!(a.result.as_ref().unwrap().exceptions.is_empty());
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 0.0);
    assert!(a.technician.bridge_point_times.contains_key(&1));
    partial.successful_reads = Some(1);
    send(&mut a, EventKind::BridgeResult { result: partial });
    assert_eq!(a.technician.slave_failures.get(&1), Some(&0));
}

#[test]
fn snapshot_makes_configuration_available_before_a_failed_scan_finishes() {
    let mut a = app();
    start(&mut a);
    a.auto_request = true;
    let mut result = read(&a, 23.0);
    result.successful_reads = None;
    result.readings.clear();
    send(
        &mut a,
        EventKind::BridgeSnapshot {
            result: result.clone(),
        },
    );
    assert_eq!(a.active, Some(42));
    assert!(a.bridge_source.is_some());
    assert!(a.result.is_some());
    assert!(!a.foreground_busy());
    result
        .exceptions
        .push(modbus_configurator::bridge::PointException {
            item: 1,
            code: 11,
            message: "No response".into(),
        });
    send(&mut a, EventKind::BridgeSnapshot { result });
    assert!(a.log_text().contains("slave 1, point 1: exception 11"));
    send(
        &mut a,
        EventKind::Error {
            code: ErrorCode::Timeout,
            message: "Scan deadline expired".into(),
            recoverable: true,
        },
    );
    assert!(a.active.is_none());
    assert!(a.auto_paused);
    assert!(!a.programming_blocked);
    assert!(a.technician.bridge_fault);
    assert_eq!(a.result.as_ref().unwrap().exceptions.len(), 1);
    a.last_scan = Instant::now();
    a.loaded_config = Some(a.profiles[0].native_tsv.clone());
    a.technician.page = technician_view::Page::Configurations;
    let text = draw(&mut a, &egui::Context::default());
    assert!(text.contains("Check recovered console"));
    assert!(text.contains("Program Modbus Bridge"));
}

#[test]
fn preflight_failure_releases_programming_lock_but_uncertain_writes_do_not() {
    let mut a = app();
    for (code, blocked) in [
        (ErrorCode::Timeout, false),
        (ErrorCode::ProgrammingUncertain, true),
    ] {
        start(&mut a);
        a.programming = true;
        a.programming_blocked = true;
        send(
            &mut a,
            EventKind::Error {
                code,
                message: "Check console".into(),
                recoverable: false,
            },
        );
        assert_eq!(a.programming_blocked, blocked);
        assert!(!a.programming);
    }
}

#[test]
fn out_of_order_slaves_keep_their_model_value_and_configured_order() {
    let mut a = app();
    a.last_scan = Instant::now();
    let devices: Vec<_> = [("wnd-m1-mb", 1), ("hmd65-nonmetric", 3), ("ati-f12", 2)]
        .into_iter()
        .map(|(key, slave)| {
            let profile = a
                .profiles
                .iter()
                .position(|p| p.info.id.contains(key))
                .unwrap();
            let mut device = modbus_configurator::network::Device::new(profile, slave, &a.profiles);
            device.points = device.points.iter().take(2).copied().collect();
            device
        })
        .collect();
    let mut result = read(&a, 11.0);
    result.native_tsv = modbus_configurator::network::compose(&devices, &a.profiles).unwrap();
    result.readings = vec![
        Reading {
            item: 1,
            value: 11.0,
        },
        Reading {
            item: 3,
            value: 33.0,
        },
        Reading {
            item: 5,
            value: 22.0,
        },
    ];
    a.technician.network_draft = Some((result.native_tsv.clone(), devices.clone()));
    a.result = Some(result);
    let ctx = egui::Context::default();
    let text = draw(&mut a, &ctx);
    assert!(text.find("Slave 1").unwrap() < text.find("Slave 3").unwrap());
    assert!(text.find("Slave 3").unwrap() < text.find("Slave 2").unwrap());
    for device in devices {
        a.technician.page = technician_view::Page::Detail(format!("slave:{}", device.slave));
        let text = draw(&mut a, &ctx);
        assert!(text.contains(&a.profiles[device.profile].info.model));
        assert!(text.contains(&format!("{}", device.slave * 11)));
    }
}

#[test]
fn saved_custom_type_is_reused_after_restart_on_a_partial_register_set() {
    let mut a = app();
    a.last_scan = Instant::now();
    let mut result = read(&a, 11.25);
    result.native_tsv = format!(
        "{}\r\n{}\r\n",
        result.native_tsv.lines().next().unwrap(),
        result.native_tsv.lines().nth(1).unwrap()
    );
    let saved = modbus_configurator::custom_profile::SavedProfile::new(
        "Short DPT",
        &a.profiles[0].info.id,
        &result.native_tsv,
        1,
    )
    .unwrap();
    let root = std::env::temp_dir().join(format!("saved-profile-ui-{}", std::process::id()));
    modbus_configurator::custom_profile::save(&root, &[saved], &a.profiles).unwrap();
    a.technician.load_custom_profiles(&root, &a.profiles);
    a.result = Some(result);
    a.technician.page = technician_view::Page::Detail("slave:1".into());
    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    let text = draw(&mut a, &ctx);
    for expected in [
        "DPT146 · slave 1",
        "11.25 °C",
        "Readings",
        "Register table",
        "Device manuals",
        "Custom profiles",
        "Save custom profile",
        "Load custom profile",
    ] {
        assert!(text.contains(expected), "Missing {expected}");
    }
    ctx.memory_mut(|m| m.set_everything_is_visible(false));
    draw(&mut a, &ctx);
    click(&mut a, &ctx, "Register table");
    assert!(a.technician.page == technician_view::Page::Registers("slave:1".into()));
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Since last read"));
    assert!(text.contains("11.25 deg C"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ambiguous_single_slave_has_a_session_type_selector_and_keeps_numeric_reads() {
    let mut a = app();
    a.last_scan = Instant::now();
    let mut result = read(&a, 11.25);
    let row = result
        .native_tsv
        .lines()
        .nth(1)
        .unwrap()
        .replace("\tF32\t", "\tS32\t");
    result.native_tsv = format!("Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\r\n{row}\r\n");
    a.result = Some(result);
    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Custom register set, please select device"));
    assert!(text.contains("11.25 · units unknown"));
    a.technician.page = technician_view::Page::Detail("slave:1".into());
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Device type for this session"));
    assert!(text.contains("11.25 · units unknown"));
}

#[test]
fn named_backup_keeps_its_name_while_queued_behind_polling() {
    let mut a = app();
    start(&mut a);
    a.last_scan = Instant::now();
    a.bridge_source = Some(port());
    a.result = Some(read(&a, 21.5));
    a.auto_request = true;
    a.backup_name = "Before sensor change".into();
    a.technician.page = technician_view::Page::Configurations;
    let ctx = egui::Context::default();
    click(&mut a, &ctx, "Back up Modbus Bridge");
    a.backup_name = "Another name".into();
    assert!(
        matches!(&a.queued, Some((Operation::BridgeNamedBackup { name }, _)) if name == "Before sensor change")
    );
}

#[test]
fn repeated_models_have_separate_readings_and_timestamps() {
    let mut a = app();
    a.last_scan = Instant::now();
    let devices: Vec<_> = (1..=2)
        .map(|slave| modbus_configurator::network::Device::new(0, slave, &a.profiles))
        .collect();
    let mut result = read(&a, 11.25);
    result.native_tsv = modbus_configurator::network::compose(&devices, &a.profiles).unwrap();
    result.readings.push(Reading {
        item: 9,
        value: 28.75,
    });
    a.result = Some(result);
    a.technician
        .bridge_point_times
        .insert(1, "2026-09-11 10:00:00 (local)".into());
    a.technician
        .bridge_point_times
        .insert(9, "2026-09-11 10:00:10 (local)".into());
    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    let text = draw(&mut a, &ctx);
    for expected in [
        "Slave 1",
        "Slave 2",
        "11.25",
        "28.75",
        "10:00:00",
        "10:00:10",
        "Configured on Modbus Bridge",
    ] {
        assert!(text.contains(expected), "Missing {expected}");
    }
    for (slave, own, other) in [(1, "11.25", "28.75"), (2, "28.75", "11.25")] {
        a.technician.page = technician_view::Page::Detail(format!("slave:{slave}"));
        let text = draw(&mut a, &ctx);
        assert!(text.contains(own));
        assert!(!text.contains(other));
        assert!(text.contains("Stale"));
    }
    a.technician.page = technician_view::Page::Detail("slave:3".into());
    assert!(draw(&mut a, &ctx).contains("no longer"));
}

#[test]
fn unified_editor_starts_with_one_device_and_blocks_duplicate_slaves() {
    let mut a = app();
    let root = std::env::temp_dir().join(format!("network-ui-{}", std::process::id()));
    a.storage_root = Ok(root.clone());
    a.technician.page = technician_view::Page::Configurations;
    let ctx = egui::Context::default();
    let text = draw(&mut a, &ctx);
    assert!(!text.contains("Single device") && !text.contains("Multi-device"));
    assert_eq!(a.network_devices.len(), 1);
    click(&mut a, &ctx, "Add device");
    assert_eq!(a.network_devices.len(), 2);
    assert_ne!(a.network_devices[0].slave, a.network_devices[1].slave);
    let before = a.loaded_config.clone();
    a.network_devices[1].slave = a.network_devices[0].slave;
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Each device needs a different slave address"));
    assert_eq!(a.loaded_config, before);
    assert!(a.queued.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_table_populates_one_device_without_replacement_and_register_edits_stay_open() {
    let mut a = app();
    let root = std::env::temp_dir().join(format!("network-edit-ui-{}", std::process::id()));
    a.storage_root = Ok(root.clone());
    let original = a.profiles[0].native_tsv.replace("\t4\tF32", "\t1234\tF32");
    a.loaded_config = Some(original.clone());
    a.technician.page = technician_view::Page::Configurations;
    let ctx = egui::Context::default();
    ctx.style_mut(|s| s.animation_time = 0.0);
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Custom register set"));
    assert!(!text.contains("mapped unambiguously"));
    assert_eq!(a.loaded_config.as_ref(), Some(&original));
    assert_eq!(a.network_devices.len(), 1);
    let count = a.network_devices[0].points.len();
    click(
        &mut a,
        &ctx,
        &format!("Register entries · {count} selected"),
    );
    click(&mut a, &ctx, "Clear selection");
    assert!(a.network_devices[0].points.is_empty());
    assert!(draw(&mut a, &ctx).contains("Network needs attention"));
    assert!(draw(&mut a, &ctx).contains("Select all"));
    click(&mut a, &ctx, "Select all");
    assert_eq!(a.network_devices[0].points.len(), count);
    click(&mut a, &ctx, "Remove device");
    assert_eq!(a.network_devices.len(), 1, "Keep the first entry available");
    click(&mut a, &ctx, "Add device");
    assert_eq!(a.network_devices.len(), 2);
    assert!(a.loaded_config.as_ref().unwrap().contains("\t1234\tF32"));
    assert!(a.queued.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
