use super::*;
#[path = "line_ui.rs"]
mod line_ui;
#[path = "network_ui.rs"]
mod network_ui;
use modbus_configurator::{
    bridge::{PointException, Reading},
    service::Backend,
    transport::Transport,
};
struct Offline;
impl Backend for Offline {
    fn inventory(&self) -> Result<Vec<PortInfo>, String> {
        Ok(vec![])
    }
    fn open(&self, _: &str) -> std::io::Result<Box<dyn Transport>> {
        panic!("GUI event tests must not open hardware")
    }
}
pub(crate) fn app() -> Configurator {
    Configurator::new(
        Service::with_backend(std::sync::Arc::new(Offline), Default::default()).unwrap(),
        modbus_configurator::reference::Reference::bundled().unwrap(),
        modbus_configurator::catalog::bundled().unwrap(),
    )
}
fn port() -> PortInfo {
    PortInfo {
        port: "COM41".into(),
        usb_vid: Some(0x0483),
        usb_pid: Some(0x5740),
        serial_number: Some("test-unit".into()),
        description: "USB console".into(),
        identity: None,
        busy: false,
    }
}
pub(crate) fn read(app: &Configurator, value: f64) -> BridgeResult {
    BridgeResult {
        identity: Identity {
            model: "ENL-MOD-32".into(),
            firmware: "3.6".into(),
        },
        native_tsv: app.profiles[0].native_tsv.clone(),
        readings: vec![Reading { item: 1, value }],
        successful_reads: Some(1),
        exceptions: vec![],
    }
}
fn start(a: &mut Configurator) {
    a.active = Some(42);
    a.selected = port().port;
    a.ports = vec![port()];
}
fn send(a: &mut Configurator, kind: EventKind) {
    a.handle_event(Event {
        request_id: 42,
        kind,
    });
}
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

fn adapter_result(slave: u8) -> modbus_configurator::adapter::AdapterResult {
    let mut p = port();
    p.usb_vid = Some(0x0403);
    p.usb_pid = Some(0x6001);
    modbus_configurator::adapter::AdapterResult {
        port: p,
        key: "dpt146".into(),
        settings: modbus_configurator::adapter::Settings {
            slave,
            even: false,
            two_stops: true,
        },
        family_only: false,
        values: std::collections::BTreeMap::from([(4, 21.5)]),
        errors: Default::default(),
    }
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

fn painted_text(shape: &egui::epaint::Shape, text: &mut String) {
    match shape {
        egui::epaint::Shape::Text(t) => {
            text.push_str(&t.galley.job.text);
            text.push('\n');
        }
        egui::epaint::Shape::Vec(v) => {
            for s in v {
                painted_text(s, text)
            }
        }
        _ => {}
    }
}
pub(crate) fn draw(a: &mut Configurator, ctx: &egui::Context) -> String {
    a.auto_paused = true;
    a.last_scan = Instant::now();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 2400.0),
            )),
            ..Default::default()
        },
        |ctx| a.frame(ctx),
    );
    let mut text = String::new();
    for shape in output.shapes {
        painted_text(&shape.shape, &mut text);
    }
    text
}
#[test]
fn all_view_states_render_correct_provenance_and_configuration_review() {
    use technician_view::Page;
    let mut a = app();
    let ctx = egui::Context::default();
    brand::apply(&ctx);
    brand::typography(&ctx);
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    for connected in [false, true] {
        a.ports = if connected { vec![port()] } else { vec![] };
        a.bridge_source = Some(port());
        let mut r = read(&a, 21.5);
        r.readings.extend([
            Reading {
                item: 6,
                value: 1.0,
            },
            Reading {
                item: 7,
                value: 1.0,
            },
            Reading {
                item: 8,
                value: 0.0,
            },
        ]);
        a.result = if connected { Some(r) } else { None };
        a.fetched_at = Some("2026-09-10 12:00:00 (local)".into());
        let mut pages = vec![
            Page::Overview,
            Page::References,
            Page::Console,
            Page::Radio,
            Page::History,
        ];
        pages.extend(
            a.reference
                .devices
                .keys()
                .map(|key| Page::Detail(key.clone())),
        );
        pages.extend(
            a.reference
                .registers
                .keys()
                .map(|key| Page::Registers(key.clone())),
        );
        for page in pages {
            a.technician.page = page;
            let text = draw(&mut a, &ctx);
            assert!(text.contains("Polygon Device Configurator"));
            if !connected {
                assert!(!text.contains("21.5"));
            }
        }
        for index in 0..a.profiles.len() {
            a.catalog_view.selected = Some(index);
            a.loaded_config = Some(a.profiles[index].native_tsv.clone());
            a.technician.page = Page::Configurations;
            let text = draw(&mut a, &ctx);
            assert!(text.contains("Review changes"));
            assert!(text.contains("Program E5 bridge"));
            if connected && index == 0 {
                assert!(text.contains("Matches the E5 bridge"));
            }
        }
    }
    a.technician.page = Page::Detail("dpt146".into());
    a.technician.bridge_stale = true;
    let text = draw(&mut a, &ctx);
    assert!(text.contains("stale"));
    a.programming_blocked = true;
    a.technician.page = Page::Configurations;
    a.file_message = "Saved selection".into();
    a.backup_error = "Disk unavailable".into();
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Check recovered console"));
    a.technician.page = Page::Overview;
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Disk unavailable"));
}
#[test]
fn live_history_chart_preserves_gaps_and_shows_receipt_times() {
    let mut a = app();
    let ctx = egui::Context::default();
    brand::typography(&ctx);
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    a.technician.page = technician_view::Page::History;
    let mut r = adapter_result(1);
    a.history.adapter(&r, "first receipt", 1000);
    a.history.adapter(&r, "second receipt", 2000);
    a.history.gap(&r.port, "Disconnected", "lost receipt", 3000);
    r.values.insert(4, 22.0);
    a.history.adapter(&r, "recovered receipt", 40000);
    a.history.discarded = 1;
    let text = draw(&mut a, &ctx);
    assert!(text.contains("first receipt"));
    assert!(text.contains("recovered receipt"));
    assert!(text.contains("Disconnected"));
    assert!(text.contains("1 oldest samples"));
    assert!(text.contains("21.5"));
    assert!(text.contains("22"));
}

#[test]
fn configuration_load_is_transactional_and_reports_storage_failures() {
    let mut a = app();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("gui-config-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    a.storage_root = Ok(dir.join("settings"));
    let path = dir.join("input.tsv");
    std::fs::write(&path, &a.profiles[0].native_tsv).unwrap();
    assert!(a.load_configuration(&path));
    assert!(dir.join("settings/Selected.tsv").is_file());
    let old = a.loaded_config.clone();
    std::fs::write(&path, "invalid").unwrap();
    assert!(!a.load_configuration(&path));
    assert_eq!(a.loaded_config, old);
    a.storage_root = Err("read-only disk".into());
    std::fs::write(&path, &a.profiles[1].native_tsv).unwrap();
    assert!(a.load_configuration(&path));
    assert!(a.file_message.contains("could not remember"));
    a.handle_action(technician_view::Action::ChooseConfig("wattnode".into()));
    assert!(a.library_open);
    assert!(a.file_message.contains("Could not remember"));
    a.handle_action(technician_view::Action::OpenArtifact("unknown-file".into()));
    assert!(a.status.contains("Cannot open"));
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

pub(crate) fn click(a: &mut Configurator, ctx: &egui::Context, label: &str) -> egui::FullOutput {
    fn locate(shapes: &[egui::epaint::ClippedShape], label: &str) -> Option<egui::Pos2> {
        shapes.iter().find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                Some(t.pos + t.galley.size() / 2.0)
            }
            _ => None,
        })
    }
    a.auto_paused = true;
    a.last_scan = Instant::now();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 2400.0),
            )),
            ..Default::default()
        },
        |ctx| a.frame(ctx),
    );
    let pos = locate(&output.shapes, label).unwrap_or_else(|| panic!("Missing {label}"));
    let mut output = output;
    for pressed in [true, false] {
        a.last_scan = Instant::now();
        output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 2400.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
            |ctx| a.frame(ctx),
        );
    }
    output
}
#[test]
fn navigation_and_configuration_buttons_change_state_and_copy_the_exact_table() {
    use technician_view::Page;
    let mut a = app();
    let ctx = egui::Context::default();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("gui-buttons-{}", std::process::id()));
    a.storage_root = Ok(root.clone());
    for (label, page) in [
        ("History", Page::History),
        ("References", Page::References),
        ("Diagnostics", Page::Console),
        ("Devices", Page::Overview),
        ("Configuration", Page::Configurations),
    ] {
        click(&mut a, &ctx, label);
        assert!(a.technician.page == page, "{label}");
    }
    for index in 0..a.profiles.len() {
        let label = a.profiles[index].info.model.clone();
        click(&mut a, &ctx, &label);
        assert_eq!(
            a.loaded_config.as_ref(),
            Some(&a.profiles[index].native_tsv)
        );
        assert_eq!(
            modbus_configurator::config_file::load(&root.join("Selected.tsv")).unwrap(),
            a.profiles[index].native_tsv
        );
    }
    click(&mut a, &ctx, "File tools");
    let output = click(&mut a, &ctx, "Copy configuration table");
    assert!(output.platform_output.commands.iter().any(
        |c| matches!(c, egui::OutputCommand::CopyText(t) if Some(t) == a.loaded_config.as_ref())
    ));
    a.file_message = "Saved".into();
    click(&mut a, &ctx, "Dismiss message");
    assert!(a.file_message.is_empty());
    a.status = "Finished".into();
    click(&mut a, &ctx, "Dismiss");
    assert!(a.status.is_empty());
    a.result = Some(read(&a, 21.5));
    a.bridge_source = Some(port());
    a.ports = vec![port()];
    click(&mut a, &ctx, "Program E5 bridge");
    assert!(a.programming && a.programming_blocked && a.active.is_some());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn bundled_references_extract_exact_bytes_and_reject_unlisted_paths() {
    let a = app();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    for path in a
        .reference
        .devices
        .values()
        .filter_map(|d| d.artifact.as_ref())
    {
        let output = extract_reference_artifact(path, &root).unwrap();
        let original = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert_eq!(
            std::fs::read(&output).unwrap(),
            std::fs::read(original).unwrap()
        );
        std::fs::remove_dir_all(output.parent().unwrap()).unwrap();
    }
    assert!(extract_reference_artifact("../Selected.tsv", &root).is_err());
}

#[test]
fn adapter_views_keep_received_values_and_time_across_disconnects() {
    use technician_view::Page;
    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    for key in ["dpt146", "hmd65", "wattnode"] {
        let mut a = app();
        let mut result = adapter_result(1);
        result.key = key.into();
        for register in &a.reference.registers[key] {
            if let Some(address) = register.first_pdu() {
                result.values.insert(address, 21.5);
                a.technician
                    .adapter_times
                    .insert(address, "2026-09-10 14:30:00".into());
            }
        }
        a.adapter = Some(result.clone());
        for connected in [true, false] {
            a.ports = if connected {
                vec![result.port.clone()]
            } else {
                vec![]
            };
            a.technician.adapter_stale = !connected;
            a.technician.page = Page::Overview;
            let text = draw(&mut a, &ctx);
            assert!(text.contains("21.50"));
            a.technician.page = Page::Detail(key.into());
            let text = draw(&mut a, &ctx);
            assert!(text.contains("2026-09-10 14:30:00"));
            if !connected {
                assert!(text.contains("Stale"));
            }
            a.technician.page = Page::Registers(key.into());
            assert!(draw(&mut a, &ctx).contains("21.5"));
        }
    }
}
#[test]
fn reference_navigation_opens_registers_and_help_in_the_same_context() {
    use technician_view::Page;
    let mut a = app();
    let ctx = egui::Context::default();
    for key in [
        "dpt146", "hmd65", "wattnode", "iaq_plus", "adapter", "bridge",
    ] {
        a.technician.page = Page::References;
        let label = a.reference.devices[key].name.clone();
        click(&mut a, &ctx, &label);
        assert!(a.technician.page == Page::Detail(key.into()));
        assert!(a.technician.reference_context);
        let reference_text = draw(&mut a, &ctx);
        assert!(reference_text.contains("Model reference"));
        assert!(!reference_text.contains("Disconnected"));
        assert!(!reference_text.contains("No readings yet"));
        click(&mut a, &ctx, "Setup & troubleshooting");
        let text = draw(&mut a, &ctx);
        assert!(a.technician.page == Page::Troubleshooting(Some(key.into())));
        assert!(text.contains(if matches!(key, "iaq_plus" | "adapter") {
            "• TBD"
        } else {
            "Troubleshooting steps"
        }));
        click(&mut a, &ctx, "‹ Device details");
        if a.reference.registers.contains_key(key) {
            click(&mut a, &ctx, "Register map");
            assert!(a.technician.page == Page::Registers(key.into()));
            let map_text = draw(&mut a, &ctx);
            assert!(map_text.contains("Manufacturer register definitions"));
            assert!(!map_text.contains("Show native values alongside display units"));
        }
    }
}

#[test]
fn screenshot_sequence_restores_the_users_configuration_and_polling_preference() {
    let mut a = app();
    let ctx = egui::Context::default();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("capture-app-{}", std::process::id()));
    a.offline = true;
    a.auto_paused = true;
    a.loaded_config = Some(a.profiles[1].native_tsv.clone());
    a.config_source = "My selection".into();
    a.catalog_view.selected = Some(1);
    let original = a.loaded_config.clone();
    a.capture = Some(capture_views::Capture::new(
        root.clone(),
        &a.reference,
        &a.profiles,
    ));
    for _ in 0..150 {
        a.last_scan = Instant::now();
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Screenshot {
                    viewport_id: egui::ViewportId::ROOT,
                    user_data: Default::default(),
                    image: std::sync::Arc::new(egui::ColorImage::new(
                        [2, 2],
                        vec![egui::Color32::WHITE; 4],
                    )),
                }],
                ..Default::default()
            },
            |ctx| a.frame(ctx),
        );
        if a.capture.is_none() {
            break;
        }
    }
    assert!(a.capture.is_none());
    assert!(root.join("complete.json").exists());
    assert_eq!(a.loaded_config, original);
    assert_eq!(a.config_source, "My selection");
    assert_eq!(a.catalog_view.selected, Some(1));
    assert!(a.auto_paused);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn backup_menu_loads_previous_configuration_without_programming() {
    let mut a = app();
    let ctx = egui::Context::default();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("gui-backups-{}", std::process::id()));
    a.storage_root = Ok(root.clone());
    a.bridge_source = Some(port());
    a.ports = vec![port()];
    a.result = Some(read(&a, 21.5));
    modbus_configurator::config_file::backup(&root, &port(), &a.profiles[0].native_tsv).unwrap();
    a.technician.page = technician_view::Page::Configurations;
    click(&mut a, &ctx, "Load backup");
    let label = format!(
        "Latest backup · {} points",
        a.profiles[0].native_tsv.lines().skip(1).count()
    );
    click(&mut a, &ctx, &label);
    assert_eq!(a.loaded_config.as_ref(), Some(&a.profiles[0].native_tsv));
    assert_eq!(a.config_source, "Latest backup");
    assert!(!a.programming);
    assert!(a.active.is_none());
    std::fs::remove_dir_all(root).unwrap();
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
        "E5 bridge point table programmed and verified."
    );
    start(&mut a);
    a.auto_request = true;
    let previous_status = a.status.clone();
    let result = read(&a, 22.5);
    send(&mut a, EventKind::BridgeResult { result });
    assert_eq!(a.status, previous_status);
}

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
fn configuration_review_names_points_that_will_be_removed() {
    let mut a = app();
    let ctx = egui::Context::default();
    ctx.memory_mut(|m| m.set_everything_is_visible(true));
    let mut current = read(&a, 21.5);
    current.native_tsv = a
        .profiles
        .iter()
        .max_by_key(|p| p.rows.len())
        .unwrap()
        .native_tsv
        .clone();
    let selected = a
        .profiles
        .iter()
        .min_by_key(|p| p.rows.len())
        .unwrap()
        .native_tsv
        .clone();
    let old = modbus_configurator::bridge::parse_export(
        &current.native_tsv,
        current.native_tsv.lines().skip(1).count(),
    )
    .unwrap();
    let new =
        modbus_configurator::bridge::parse_export(&selected, selected.lines().skip(1).count())
            .unwrap();
    assert!(old.len() > new.len());
    a.result = Some(current);
    a.loaded_config = Some(selected);
    a.technician.page = technician_view::Page::Configurations;
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Remove points:"));
    assert!(text.contains("Selected: Remove"));
    for item in old.keys().filter(|k| !new.contains_key(k)) {
        assert!(text.contains(&format!("Point {item}")));
    }
}

#[test]
fn renamed_brand_progress_and_unit_conversion_preserve_native_readings() {
    assert_eq!(
        transfer_progress("Programming selected point table (3/8)"),
        Some(0.375)
    );
    for stage in [
        "Reading native E5 bridge point table",
        "bad (9/8)",
        "bad (0/0)",
        "bad (x/4)",
    ] {
        assert_eq!(transfer_progress(stage), None);
    }
    let icon = brand::icon();
    assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    for (unit, index, value, expected) in [
        ("°C", 1, 0.0, 32.0),
        ("°C", 2, -273.15, 0.0),
        ("bara", 1, 1.0, 100.0),
        ("bara", 3, 1.0, 14.503773773),
        ("ppmv", 1, 10000.0, 1.0),
        ("g/m³", 1, 1.0, 1000.0),
        ("g/kg", 1, 1.0, 1000.0),
        ("kJ/kg", 1, 1.0, 1000.0),
        ("W", 1, 1000.0, 1.0),
        ("kWh", 1, 1.0, 1000.0),
        ("V", 1, 1000.0, 1.0),
        ("Vac", 1, 1000.0, 1.0),
        ("A", 1, 1.0, 1000.0),
        ("Hz", 1, 1000.0, 1.0),
        ("bar", 1, 1.0, 100.0),
    ] {
        let (_, scale, offset) = units::choices(unit)[index];
        assert!((value * scale + offset - expected).abs() < 1e-8);
    }
    assert!(units::choices("%RH").is_empty());
    assert!(units::choices("code").is_empty());
    let mut a = app();
    start(&mut a);
    let result = read(&a, 0.0);
    send(&mut a, EventKind::BridgeResult { result });
    a.technician.page = technician_view::Page::Detail("dpt146".into());
    let ctx = egui::Context::default();
    click(&mut a, &ctx, "Units: °C");
    let text = draw(&mut a, &ctx);
    assert!(text.contains("32.00 °F"), "{text}");
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 0.0);
    a.technician.page = technician_view::Page::Registers("dpt146".into());
    assert!(draw(&mut a, &ctx).contains("Units: °F"));
}
#[test]
fn switching_sensor_profile_warns_on_overview_and_configured_device_page() {
    let mut a = app();
    start(&mut a);
    let result = read(&a, 21.5);
    send(&mut a, EventKind::BridgeResult { result });
    a.active = Some(42);
    let mut result = read(&a, 9999.0);
    result.native_tsv = a
        .profiles
        .iter()
        .find(|p| p.info.id == "hmd65")
        .unwrap()
        .native_tsv
        .clone();
    result.exceptions.push(PointException {
        item: 2,
        code: 2,
        message: "Illegal Data Address".into(),
    });
    send(&mut a, EventKind::BridgeResult { result });
    let ctx = egui::Context::default();
    for page in [
        technician_view::Page::Overview,
        technician_view::Page::Detail("hmd65".into()),
    ] {
        a.technician.page = page;
        let text = draw(&mut a, &ctx);
        assert!(text.contains("Possible configuration mismatch"), "{text}");
        assert!(text.contains("DPT146 to HMD65"));
    }
    a.active = Some(42);
    let result = a.result.clone().unwrap();
    send(&mut a, EventKind::BridgeResult { result });
    assert!(a.technician.configuration_change.is_some());
}

#[test]
fn reference_cards_have_equal_widths_at_supported_window_sizes() {
    for width in [960.0, 1180.0] {
        let mut a = app();
        a.technician.page = technician_view::Page::References;
        let ctx = egui::Context::default();
        a.auto_paused = true;
        a.last_scan = Instant::now();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 1000.0),
                )),
                ..Default::default()
            },
            |ctx| a.frame(ctx),
        );
        let cards: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Rect(r)
                    if r.stroke.width > 0.0
                        && r.rect.width() > width - 100.0
                        && r.rect.height() > 30.0
                        && r.rect.height() < 150.0
                        && r.rect.bottom() < 936.0 =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(cards.len(), 7, "{cards:?}");
        for rect in &cards {
            assert!((rect.width() - cards[0].width()).abs() < 0.1);
            assert!(rect.right() <= width);
        }
    }
}

#[test]
fn status_bar_keeps_content_fixed_and_text_clear_of_progress_track() {
    for size in [egui::vec2(960.0, 640.0), egui::vec2(1180.0, 760.0)] {
        let mut a = app();
        let ctx = egui::Context::default();
        a.auto_paused = true;
        let mut heading_position = None;
        for (active, status) in [
            (false, ""),
            (
                true,
                "Reading all configured Modbus points through the E5 bridge",
            ),
            (true, "Programming selected point table (3/8)"),
            (false, "Backup saved"),
        ] {
            a.active = active.then_some(42);
            a.status = status.into();
            a.last_scan = Instant::now();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| a.frame(ctx),
            );
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::epaint::Shape::Text(t) => Some(t),
                    _ => None,
                })
                .collect();
            let heading = texts
                .iter()
                .find(|t| t.galley.text() == "Polygon Device Configurator")
                .unwrap()
                .pos;
            if let Some(previous) = heading_position {
                assert_eq!(heading, previous);
            } else {
                heading_position = Some(heading);
            }
            let label = if status.is_empty() {
                "Polling paused"
            } else {
                status
            };
            let text = texts.iter().find(|t| t.galley.text() == label).unwrap();
            assert!(text.pos.y >= size.y - 64.0);
            let track = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::epaint::Shape::Rect(r)
                        if r.rect.width() > size.x - 100.0
                            && r.rect.height() <= 8.0
                            && r.rect.top() > size.y - 64.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .next()
                .unwrap();
            assert!(text.pos.y + text.galley.size().y < track.top());
            assert_eq!(texts.iter().filter(|t| t.galley.text() == label).count(), 1);
        }
    }
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
    assert!(draw(&mut a, &ctx).contains("Program E5 bridge queued"));
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
fn activity_log_groups_repeats_and_keeps_recent_events() {
    let mut a = app();
    a.technician.page = technician_view::Page::Console;
    for n in 0..65 {
        a.record_activity(format!("Event {n}"));
    }
    assert_eq!(a.replay_log.len(), 64);
    assert!(a.replay_log[0].ends_with(" | Event 1"));
    a.replay_log = vec![
        "E5 bridge verified".into(),
        "E5 bridge verified".into(),
        "Backup saved".into(),
    ];
    let ctx = egui::Context::default();
    click(&mut a, &ctx, "Activity log");
    let text = draw(&mut a, &ctx);
    assert!(text.contains("2 times"));
    assert_eq!(text.matches("E5 bridge verified").count(), 1);
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
fn regional_units_convert_and_preserve_individual_overrides() {
    use units::Preset;
    for (region, expected) in [
        (Some("US"), Preset::Us),
        (Some("GB"), Preset::Uk),
        (Some("DE"), Preset::Eu),
        (None, Preset::Eu),
    ] {
        assert_eq!(Preset::System.resolve(region), expected);
        assert_eq!(Preset::Uk.resolve(region), Preset::Uk);
    }
    for (preset, native, input, unit, expected) in [
        (Preset::Us, "°C", 20.0, "°F", 68.0),
        (Preset::Us, "bara", 1.0, "psia", 14.503773773),
        (Preset::Eu, "bara", 1.0, "kPa(a)", 100.0),
        (Preset::Uk, "bara", 1.0, "bara", 1.0),
        (Preset::Us, "kJ/kg", 1.0, "Btu/lb", 0.4299226139294927),
    ] {
        let (label, scale, offset) = units::choices(native)[preset.index(native)];
        assert_eq!(label, unit);
        assert!((input * scale + offset - expected).abs() < 1e-9);
    }
    let mut a = app();
    start(&mut a);
    a.technician.unit_preset = Preset::Us;
    a.technician.page = technician_view::Page::Registers("dpt146".into());
    let result = read(&a, 20.0);
    let native = result.native_tsv.clone();
    send(&mut a, EventKind::BridgeResult { result });
    let ctx = egui::Context::default();
    assert!(draw(&mut a, &ctx).contains("Units: °F"));
    click(&mut a, &ctx, "Units: °F");
    a.technician.unit_preset = Preset::Eu;
    assert!(draw(&mut a, &ctx).contains("Units: K"));
    a.technician.reset_unit_overrides();
    assert!(!draw(&mut a, &ctx).contains("Units: K"));
    assert_eq!(a.result.as_ref().unwrap().native_tsv, native);
    assert_eq!(a.result.as_ref().unwrap().readings[0].value, 20.0);
    let preferences = Preferences {
        units: Preset::Us,
        ..Default::default()
    };
    let restored: Preferences =
        serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
    assert_eq!(restored.units, Preset::Us);
}

#[test]
fn troubleshooting_tab_opens_application_guidance_without_hardware_actions() {
    let mut a = app();
    let ctx = egui::Context::default();
    click(&mut a, &ctx, "Troubleshooting");
    assert!(a.technician.page == technician_view::Page::Troubleshooting(None));
    let text = draw(&mut a, &ctx);
    for title in [
        "USB device is missing or unavailable",
        "Readings are not updating",
        "Backup, load or programming failed",
        "Theme, units or settings look wrong",
    ] {
        assert!(text.contains(title));
    }
    assert!(a.active.is_none());
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

#[test]
fn adapter_page_displays_sensor_readings_and_retains_them_after_disconnect() {
    let mut a = app();
    let result = adapter_result(1);
    a.ports = vec![result.port.clone()];
    a.active = Some(42);
    send(&mut a, EventKind::AdapterResult { result });
    a.technician.page = technician_view::Page::Detail("adapter".into());
    a.technician
        .adapter_times
        .insert(4, "September 10, 2026 17:30:00".into());
    a.technician.unit_preset = crate::units::Preset::Eu;
    let ctx = egui::Context::default();
    let text = draw(&mut a, &ctx);
    assert!(text.contains("Sensor: Vaisala DPT146"), "{text}");
    assert!(text.contains("21.50"), "{text}");
    assert!(text.contains("September 10, 2026 17:30:00"));
    assert!(text.contains("Register table"));
    assert!(!text.contains("No readings yet"));
    a.ports.clear();
    let text = draw(&mut a, &ctx);
    assert!(text.contains("21.50"));
    assert!(text.contains("Stale"));
}

#[test]
fn ati_register_rows_clear_the_full_wrapped_description() {
    for width in [960.0, 1920.0] {
        let mut a = app();
        a.technician.page = technician_view::Page::Registers("ati-f12".into());
        a.auto_paused = true;
        a.last_scan = Instant::now();
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 4000.0),
                )),
                ..Default::default()
            },
            |ctx| a.frame(ctx),
        );
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        let registers = &a.reference.registers["ati-f12"];
        for pair in registers[..5].windows(2) {
            let description = texts
                .iter()
                .find(|t| t.galley.text() == pair[0].description)
                .unwrap();
            let next = texts
                .iter()
                .find(|t| t.galley.text() == pair[1].name)
                .unwrap();
            assert!(
                description.pos.y + description.galley.size().y <= next.pos.y,
                "Overlapping ATI rows at width {width}"
            );
        }
    }
}

#[test]
fn device_manuals_extract_as_pdf_files_offline() {
    let root = std::env::temp_dir().join(format!("manual-check-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for key in [
        "bridge", "dpt146", "hmd65", "wattnode", "ati-f12", "adapter",
    ] {
        let manuals = modbus_configurator::reference::manuals(key);
        assert!(!manuals.is_empty());
        for (_, path) in manuals {
            let output = extract_reference_artifact(path, &root).unwrap();
            assert!(std::fs::read(output).unwrap().starts_with(b"%PDF"));
        }
    }
    std::fs::remove_dir_all(root).unwrap();
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
