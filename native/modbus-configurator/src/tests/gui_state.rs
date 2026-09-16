//! GUI state regression tests using the shared offline fixtures.
use super::*;

#[test]
fn disconnected_programming_stays_blocked_after_late_error_and_replug() {
    for scan_error in [false, true] {
        let mut a = app();
        start(&mut a);
        a.programming = true;
        a.scan_pending = Some(43);
        let kind = if scan_error {
            EventKind::Error {
                code: ErrorCode::InventoryUnavailable,
                message: "USB inventory failed".into(),
                recoverable: true,
            }
        } else {
            EventKind::PortSnapshot { ports: vec![] }
        };
        a.handle_event(Event {
            request_id: 43,
            kind,
        });
        assert!(
            a.programming_blocked,
            "An interrupted program must stay blocked before its delayed completion arrives"
        );
        assert!(!a.programming);
        assert!(a.auto_paused);
        send(
            &mut a,
            EventKind::Error {
                code: ErrorCode::ProgrammingUncertain,
                message: "partial write".into(),
                recoverable: false,
            },
        );
        a.scan_pending = Some(44);
        a.handle_event(Event {
            request_id: 44,
            kind: EventKind::PortSnapshot {
                ports: vec![port()],
            },
        });
        assert!(a.programming_blocked);
        assert!(a.auto_paused);
    }
}

#[test]
fn stale_inventory_cannot_disconnect_current_device_or_clear_pending_scan() {
    let mut a = app();
    start(&mut a);
    a.scan_pending = Some(43);
    a.handle_event(Event {
        request_id: 40,
        kind: EventKind::PortSnapshot { ports: vec![] },
    });
    assert_eq!(a.active, Some(42));
    assert_eq!(a.scan_pending, Some(43));
    assert_eq!(a.ports.len(), 1);
}

#[test]
fn failed_point_retains_timestamp_and_value_but_history_has_gap() {
    let mut a = app();
    start(&mut a);
    let r = read(&a, 23.5);
    send(&mut a, EventKind::BridgeResult { result: r });
    a.technician
        .bridge_times
        .insert(4, "last successful time".into());
    start(&mut a);
    let mut r = read(&a, 0.0);
    r.readings.clear();
    r.successful_reads = Some(0);
    r.exceptions = vec![PointException {
        item: 1,
        code: 2,
        message: "Illegal address".into(),
    }];
    send(&mut a, EventKind::BridgeResult { result: r });
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 23.5);
    assert_eq!(a.technician.bridge_times[&4], "last successful time");
    assert!(
        a.history
            .samples
            .iter()
            .rev()
            .find(|s| s.definition.contains("Temperature"))
            .unwrap()
            .value
            .is_none()
    );
    start(&mut a);
    let r = read(&a, 0.0);
    send(&mut a, EventKind::BridgeResult { result: r });
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 0.0);
}

#[test]
fn delayed_read_after_disconnect_does_not_resurrect_device() {
    let mut a = app();
    start(&mut a);
    let r = read(&a, 23.5);
    send(&mut a, EventKind::BridgeResult { result: r });
    start(&mut a);
    a.scan_pending = Some(43);
    a.handle_event(Event {
        request_id: 43,
        kind: EventKind::PortSnapshot { ports: vec![] },
    });
    let count = a.history.samples.len();
    let r = read(&a, 99.0);
    send(&mut a, EventKind::BridgeResult { result: r });
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 23.5);
    assert_eq!(a.history.samples.len(), count);
    assert!(a.technician.bridge_stale);
}

#[test]
fn backup_feedback_does_not_erase_file_feedback_and_old_events_are_ignored() {
    let mut a = app();
    start(&mut a);
    a.file_message = "Configuration loaded".into();
    send(
        &mut a,
        EventKind::Backup {
            path: None,
            error: Some("disk full".into()),
        },
    );
    assert!(a.backup_error.contains("disk full"));
    assert_eq!(a.file_message, "Configuration loaded");
    send(
        &mut a,
        EventKind::Backup {
            path: Some("backup.tsv".into()),
            error: None,
        },
    );
    assert!(a.backup_error.is_empty());
    assert_eq!(a.backup_path.as_deref(), Some("backup.tsv"));
    a.handle_event(Event {
        request_id: 1,
        kind: EventKind::Error {
            code: ErrorCode::Transport,
            message: "late".into(),
            recoverable: true,
        },
    });
    assert_ne!(a.status, "late");
    assert_eq!(a.active, Some(42));
}

#[test]
fn failed_adapter_read_does_not_advance_last_received_timestamp() {
    let mut a = app();
    let r = adapter_result(1);
    a.ports = vec![r.port.clone()];
    a.selected = r.port.port.clone();
    a.active = Some(42);
    send(&mut a, EventKind::AdapterResult { result: r });
    a.fetched_at = Some("last good time".into());
    a.technician
        .adapter_times
        .insert(4, "last good time".into());
    let mut r = adapter_result(1);
    r.values.clear();
    r.errors.insert(4, "Timeout".into());
    a.active = Some(42);
    send(&mut a, EventKind::AdapterResult { result: r });
    assert_eq!(a.fetched_at.as_deref(), Some("last good time"));
    assert_eq!(a.technician.adapter_times[&4], "last good time");
    assert_eq!(a.adapter.unwrap().values[&4], 21.5);
    assert_eq!(a.history.samples.back().unwrap().value, None);
}

#[test]
fn different_adapter_slave_does_not_inherit_previous_timestamps() {
    let mut a = app();
    let r = adapter_result(1);
    a.ports = vec![r.port.clone()];
    a.active = Some(42);
    send(&mut a, EventKind::AdapterResult { result: r });
    let mut r = adapter_result(2);
    r.values.clear();
    r.values.insert(6, 4.0);
    a.active = Some(42);
    send(&mut a, EventKind::AdapterResult { result: r });
    assert!(!a.technician.adapter_times.contains_key(&4));
    assert!(!a.adapter.unwrap().values.contains_key(&4));
}

#[test]
fn explicit_verified_console_check_clears_programming_block() {
    let mut a = app();
    start(&mut a);
    a.programming_blocked = true;
    a.auto_paused = true;
    let mut r = read(&a, 0.0);
    r.readings.clear();
    r.successful_reads = None;
    send(&mut a, EventKind::BridgeResult { result: r });
    assert!(!a.programming_blocked);
    assert!(a.auto_paused);
    assert!(a.history.samples.is_empty());
}

#[test]
fn polling_respects_pause_block_and_ambiguous_routes() {
    for state in ["paused", "blocked", "ambiguous", "bridge", "adapter"] {
        let mut a = app();
        a.ports = vec![port()];
        a.selected.clear();
        a.auto_paused = state == "paused";
        a.programming_blocked = state == "blocked";
        if state == "ambiguous" {
            let mut other = port();
            other.port = "COM99".into();
            other.serial_number = Some("other".into());
            a.ports.push(other);
        }
        if state == "adapter" {
            a.ports = vec![adapter_result(1).port];
        }
        a.last_scan = Instant::now();
        a.last_fetch = Instant::now() - Duration::from_secs(6);
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| a.frame(ctx));
        if ["bridge", "adapter"].contains(&state) {
            assert!(a.active.is_some());
            assert!(a.auto_request);
        } else {
            assert!(a.active.is_none());
        }
        if state == "ambiguous" {
            assert!(a.status.contains("Multiple USB"));
        }
    }
    let mut a = app();
    a.handle_action(technician_view::Action::ReadBridge);
    assert!(!a.auto_paused);
    a.handle_action(technician_view::Action::BackupBridge);
    assert!(a.status.contains("Select one"));
    a.ports = vec![port()];
    a.handle_action(technician_view::Action::BackupBridge);
    assert!(a.active.is_some());
    a.handle_action(technician_view::Action::Refresh);
    assert!(a.scan_pending.is_some());
}

#[test]
fn active_operation_feedback_is_correlated_and_program_completion_resumes_polling() {
    let mut a = app();
    start(&mut a);
    for _ in 0..80 {
        send(
            &mut a,
            EventKind::Progress {
                stage: "Receiving".into(),
            },
        );
    }
    assert_eq!(a.replay_log.len(), 64);
    assert_eq!(a.status, "Receiving");
    send(
        &mut a,
        EventKind::PromptState {
            state: modbus_configurator::console::PromptState::MainMenu,
        },
    );
    assert_eq!(a.replay_log.len(), 64);
    a.replay_log.clear();
    send(
        &mut a,
        EventKind::PromptState {
            state: modbus_configurator::console::PromptState::MainMenu,
        },
    );
    assert_eq!(a.replay_log.len(), 1);
    send(
        &mut a,
        EventKind::Result {
            message: "Released".into(),
        },
    );
    assert!(a.active.is_none());
    assert_eq!(a.status, "Released");
    start(&mut a);
    a.programming = true;
    a.programming_blocked = true;
    a.auto_paused = true;
    let result = read(&a, 21.5);
    send(&mut a, EventKind::BridgeResult { result });
    assert!(!a.programming && !a.programming_blocked && !a.auto_paused);
    assert_eq!(
        a.file_message,
        "Modbus Bridge point table programmed and verified."
    );
    start(&mut a);
    a.auto_request = true;
    let previous_status = a.status.clone();
    let result = read(&a, 22.5);
    send(&mut a, EventKind::BridgeResult { result });
    assert_eq!(a.status, previous_status);
}

#[test]
fn polling_is_quiet_and_programming_queues_without_interrupting_the_read() {
    let mut a = app();
    start(&mut a);
    a.auto_request = true;
    a.status = "Backup saved".into();
    send(
        &mut a,
        EventKind::Progress {
            stage: "Reading points".into(),
        },
    );
    assert_eq!(a.status, "Backup saved");
    assert!(!a.foreground_busy());
    let target = a.profiles[1].native_tsv.clone();
    a.hardware(Operation::BridgeProgram {
        target: target.clone(),
        reviewed: a.profiles[0].native_tsv.clone(),
    });
    assert_eq!(a.active, Some(42));
    assert!(!a.programming && !a.programming_blocked);
    assert!(
        matches!(&a.queued, Some((Operation::BridgeProgram { target: t, .. }, route)) if t == &target && route.port == port().port)
    );
    let ctx = egui::Context::default();
    assert!(draw(&mut a, &ctx).contains("Program Modbus Bridge queued"));
    let result = read(&a, 23.0);
    send(&mut a, EventKind::BridgeResult { result });
    a.start_queued();
    assert!(a.queued.is_none() && a.active.is_some());
    assert!(a.programming && a.programming_blocked && !a.auto_request);
}

#[test]
fn queued_actions_can_be_cancelled_and_never_follow_a_changed_usb_route() {
    for cancel in [true, false] {
        let mut a = app();
        start(&mut a);
        a.auto_request = true;
        a.hardware(Operation::BridgeExport);
        if cancel {
            click(&mut a, &egui::Context::default(), "Cancel queued");
            assert_eq!(a.active, Some(42));
        } else {
            a.active = None;
            a.ports[0].serial_number = Some("different-unit".into());
            a.start_queued();
            assert!(a.status.contains("cancelled"));
            assert!(a.active.is_none());
        }
        assert!(a.queued.is_none());
    }
}

#[test]
fn automatic_polling_hands_off_between_bridge_and_adapter_without_com_assumptions() {
    let mut adapter = port();
    adapter.port = "COM73".into();
    adapter.usb_vid = Some(0x0403);
    adapter.usb_pid = Some(0x6001);
    adapter.serial_number = Some("adapter-unique".into());
    for (previous, current) in [(port(), adapter.clone()), (adapter, port())] {
        let mut a = app();
        a.preferred_route = Some(previous);
        a.ports = vec![current.clone()];
        a.last_scan = Instant::now();
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| a.frame(ctx));
        assert_eq!(a.selected, current.port);
        assert!(a.active.is_some() && a.auto_request);
    }
}
