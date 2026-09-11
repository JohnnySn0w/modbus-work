//! Mode changes preserve drafts and invalid networks cannot queue programming.
use super::*;

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
        "11.25 deg C",
        "Custom profiles",
        "Save custom profile",
        "Load custom profile",
    ] {
        assert!(text.contains(expected), "Missing {expected}");
    }
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
    click(&mut a, &ctx, "Back up E5 bridge");
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
        "slave 1",
        "slave 2",
        "11.25",
        "28.75",
        "10:00:00",
        "10:00:10",
        "Configured on E5 bridge",
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
fn network_mode_preserves_single_draft_and_blocks_duplicate_slaves() {
    let mut a = app();
    a.storage_root = Ok(std::env::temp_dir().join(format!("network-ui-{}", std::process::id())));
    a.last_scan = Instant::now();
    a.loaded_config = Some(a.profiles[0].native_tsv.clone());
    let single = a.loaded_config.clone();
    a.catalog_view.selected = Some(0);
    a.technician.page = technician_view::Page::Configurations;
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    click(&mut a, &ctx, "Multi-device");
    assert!(a.multi_device);
    assert_eq!(a.network_devices.len(), 1);
    click(&mut a, &ctx, "Add device");
    assert_eq!(a.network_devices.len(), 2);
    assert_ne!(a.network_devices[0].slave, a.network_devices[1].slave);
    a.network_devices[1].slave = a.network_devices[0].slave;
    let mut valid = true;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| valid = a.network_configuration(ui));
    });
    assert!(!valid);
    assert!(a.queued.is_none());
    click(&mut a, &ctx, "Single device");
    assert!(!a.multi_device);
    assert_eq!(a.loaded_config, single);
    assert_eq!(a.catalog_view.selected, Some(0));
    std::fs::remove_dir_all(a.storage_root.unwrap()).unwrap();
}

#[test]
fn custom_network_requires_explicit_replacement_and_register_edits_remain_reviewable() {
    let mut a = app();
    let root = std::env::temp_dir().join(format!("network-edit-ui-{}", std::process::id()));
    a.storage_root = Ok(root.clone());
    a.last_scan = Instant::now();
    a.loaded_config = Some("unrecognized imported table".into());
    a.technician.page = technician_view::Page::Configurations;
    let ctx = egui::Context::default();
    ctx.style_mut(|s| s.animation_time = 0.0);
    click(&mut a, &ctx, "Multi-device");
    assert!(a.network_unrecognized);
    assert_eq!(
        a.loaded_config.as_deref(),
        Some("unrecognized imported table")
    );
    click(&mut a, &ctx, "Start new network");
    assert!(!a.network_unrecognized);
    click(&mut a, &ctx, "Add device");
    let count = a.network_devices[0].points.len();
    click(
        &mut a,
        &ctx,
        &format!("Register entries · {count} selected"),
    );
    click(&mut a, &ctx, "Clear selection");
    assert!(a.network_devices[0].points.is_empty());
    assert!(draw(&mut a, &ctx).contains("Network needs attention"));
    click(&mut a, &ctx, "Register entries · 0 selected");
    click(&mut a, &ctx, "Select all");
    assert_eq!(a.network_devices[0].points.len(), count);
    click(&mut a, &ctx, "Remove device");
    assert!(a.network_devices.is_empty());
    assert!(a.queued.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
