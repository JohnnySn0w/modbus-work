//! GUI diagnostics regression tests using the shared offline fixtures.
use super::*;

#[test]
fn diagnostics_and_cancel_buttons_preserve_the_selected_usb_route() {
    use technician_view::Page;
    let mut a = app();
    let ctx = egui::Context::default();
    a.ports = vec![port()];
    a.technician.page = Page::Console;
    click(&mut a, &ctx, "Use this interface");
    assert_eq!(a.selected, port().port);
    assert!(!a.preferences.automatic_polling);
    // Polling must stay off even after a manual action clears the transient pause.
    a.auto_paused = false;
    a.last_fetch = Instant::now() - Duration::from_secs(30);
    a.last_scan = Instant::now();
    let _ = ctx.run(Default::default(), |ctx| a.frame(ctx));
    assert!(a.active.is_none());
    assert_eq!(
        a.preferred_route.as_ref().unwrap().serial_number,
        port().serial_number
    );
    click(&mut a, &ctx, "Read now");
    assert!(a.active.is_some());
    assert!(!a.preferences.automatic_polling);
    a.active = None;
    click(&mut a, &ctx, "Release console");
    assert!(a.auto_paused);
    a.active = Some(42);
    click(&mut a, &ctx, "Cancel");
    assert!(a.auto_paused);
    assert!(a.status.contains("Cancelling"));
    a.active = None;
    a.ports.clear();
    click(&mut a, &ctx, "Offline diagnostics");
    click(&mut a, &ctx, "Run offline prompt replay");
    assert!(a.next_id > 0);
    for status in ["", "Receiving"] {
        a.technician.page = Page::Overview;
        a.active = Some(42);
        a.status = status.into();
        let before = a.next_id;
        click(&mut a, &ctx, "Cancel");
        assert_eq!(a.next_id, before + 1);
        assert!(a.auto_paused);
    }
    a.active = None;
    a.programming_blocked = true;
    a.ports = vec![port()];
    a.technician.page = Page::Configurations;
    click(&mut a, &ctx, "Check recovered console");
    assert!(a.active.is_some());
    assert!(a.programming_blocked);
}

#[test]
fn activity_log_groups_repeats_and_keeps_recent_events() {
    let mut a = app();
    a.technician.page = technician_view::Page::Console;
    for n in 0..65 {
        a.record_activity(format!("Event {n}"));
    }
    assert_eq!(a.replay_log.len(), 64);
    assert!(a.replay_log[0].ends_with(" | Event 1"));
    a.replay_log = vec![
        "Modbus Bridge verified".into(),
        "Modbus Bridge verified".into(),
        "Backup saved".into(),
    ];
    let ctx = egui::Context::default();
    click(&mut a, &ctx, "Activity log");
    let text = draw(&mut a, &ctx);
    assert!(text.contains("2 times"));
    assert_eq!(text.matches("Modbus Bridge verified").count(), 1);
    click(&mut a, &ctx, "Clear log");
    assert!(a.replay_log.is_empty());
}

#[test]
fn settings_persist_and_polling_off_does_not_cancel_or_restart_work() {
    let mut a = app();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!(
            "gui-settings-{}",
            modbus_configurator::history::now_ms()
        ));
    a.storage_root = Ok(root.clone());
    a.load_preferences();
    assert!(a.preferences.automatic_polling);
    let ctx = egui::Context::default();
    a.technician.page = technician_view::Page::Settings;
    start(&mut a);
    a.auto_request = true;
    click(&mut a, &ctx, "Automatic polling");
    assert!(!a.preferences.automatic_polling);
    assert_eq!(a.active, Some(42));
    click(&mut a, &ctx, "Dark");
    assert!(ctx.style().visuals.dark_mode);
    assert_eq!(ctx.style().visuals.panel_fill, brand::BLACK);
    let mut b = app();
    b.storage_root = Ok(root.clone());
    b.load_preferences();
    assert!(b.preferences.dark_mode == Some(true) && !b.preferences.automatic_polling);
    let result = read(&a, 24.0);
    send(&mut a, EventKind::BridgeResult { result });
    a.auto_paused = false;
    a.last_fetch = Instant::now() - Duration::from_secs(60);
    a.last_scan = Instant::now();
    let _ = ctx.run(Default::default(), |ctx| a.frame(ctx));
    assert!(a.active.is_none());
    a.handle_action(technician_view::Action::ReadBridge);
    assert!(a.active.is_some() && !a.auto_request);
    assert!(!a.preferences.automatic_polling);
    b.storage_root = Err("Unavailable settings directory".into());
    b.load_preferences();
    assert!(!b.preferences.automatic_polling && b.settings_message.contains("Could not load"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn system_theme_tracks_changes_and_keeps_explicit_preferences() {
    let mut a = app();
    a.auto_paused = true;
    let ctx = egui::Context::default();
    assert_eq!(a.preferences.dark_mode, None);
    for (choice, system, expected) in [
        (None, egui::Theme::Dark, true),
        (None, egui::Theme::Light, false),
        (Some(true), egui::Theme::Light, true),
        (Some(false), egui::Theme::Dark, false),
    ] {
        a.preferences.dark_mode = choice;
        a.last_scan = Instant::now();
        let _ = ctx.run(
            egui::RawInput {
                system_theme: Some(system),
                ..Default::default()
            },
            |ctx| a.frame(ctx),
        );
        assert_eq!(ctx.style().visuals.dark_mode, expected);
    }
    let old: Preferences =
        serde_json::from_str(r#"{"dark_mode":true,"automatic_polling":false}"#).unwrap();
    assert_eq!(old.dark_mode, Some(true));
    assert!(!old.automatic_polling);
}

#[test]
fn activity_records_timestamped_results_and_background_failures_without_poll_noise() {
    let mut a = app();
    start(&mut a);
    a.auto_request = true;
    send(
        &mut a,
        EventKind::Progress {
            stage: "routine polling".into(),
        },
    );
    assert!(a.replay_log.is_empty());
    send(
        &mut a,
        EventKind::Error {
            code: ErrorCode::Timeout,
            message: "Sensor did not respond".into(),
            recoverable: true,
        },
    );
    let error = a.replay_log.last().unwrap();
    assert!(error.contains("Request 42 | COM41 | Error Timeout: Sensor did not respond"));
    assert!(error.starts_with(&modbus_configurator::last_good::timestamp()[..10]));
    assert!(error.contains(" (local) | "));
    start(&mut a);
    a.auto_request = false;
    send(
        &mut a,
        EventKind::Backup {
            path: Some("Backups/test.tsv".into()),
            error: None,
        },
    );
    let mut result = read(&a, 22.5);
    result.exceptions.push(PointException {
        item: 2,
        code: 2,
        message: "Illegal address".into(),
    });
    send(&mut a, EventKind::BridgeResult { result });
    start(&mut a);
    send(
        &mut a,
        EventKind::Result {
            message: "Read complete".into(),
        },
    );
    let text = a.replay_log.join("\n");
    assert!(text.contains("Backup saved: Backups/test.tsv"));
    assert!(
        text.contains("1 returned values; 1 exceptions; point 2: exception 2: Illegal address")
    );
    assert!(text.contains("Completed: Read complete"));
    assert!(!text.contains("Item\tID"));
}

#[test]
fn diagnostic_snapshot_uses_current_response_before_retained_values() {
    let mut a = app();
    start(&mut a);
    let previous = read(&a, 22.0);
    send(&mut a, EventKind::BridgeResult { result: previous });
    start(&mut a);
    let mut current = read(&a, 0.0);
    current.readings.clear();
    current.successful_reads = Some(0);
    send(&mut a, EventKind::BridgeResult { result: current });
    let (port, at, findings) = a.diagnostic_report.as_ref().unwrap();
    assert_eq!(port, "COM41");
    assert!(at.contains("(local)"));
    assert!(
        findings
            .iter()
            .any(|f| f.subject == "Slave 1 · point 1" && f.detail.contains("No value"))
    );
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 22.0);
    assert!(
        a.replay_log
            .iter()
            .any(|entry| entry.contains("Diagnostic review | Slave 1 · point 1"))
    );
    a.replay_log.clear();
    let text = a.log_text();
    assert!(text.contains("Communication and data checks | COM41"));
    assert!(text.contains("No value or exception returned"));
    start(&mut a);
    a.auto_request = true;
    let mut repeated = read(&a, 0.0);
    repeated.readings.clear();
    repeated.successful_reads = Some(0);
    send(&mut a, EventKind::BridgeResult { result: repeated });
    assert!(
        a.replay_log.is_empty(),
        "Unchanged automatic findings should not flood activity"
    );
}

#[test]
fn communication_timeline_is_bounded_transferable_and_does_not_replace_status() {
    let mut a = app();
    start(&mut a);
    a.auto_request = true;
    a.status = "Ready".into();
    for i in 0..140 {
        send(
            &mut a,
            EventKind::Progress {
                stage: format!("Communication | Waiting sample {i}"),
            },
        );
    }
    assert_eq!(a.communication_log.len(), 128);
    assert!(a.communication_log[0].contains("Waiting sample 12"));
    assert_eq!(a.status, "Ready");
    assert!(a.replay_log.is_empty());
    assert!(
        a.log_text()
            .contains("Request 42 | COM41 | Communication | Waiting sample 139")
    );
    a.active = None;
    send(
        &mut a,
        EventKind::Progress {
            stage: "Communication | stale request".into(),
        },
    );
    assert!(!a.log_text().contains("stale request"));
    let settings: Preferences = serde_json::from_str(r#"{"automatic_polling": false}"#).unwrap();
    assert_eq!(settings.communication.read_all_seconds, 180);
}
