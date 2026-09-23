//! GUI configuration regression tests using the shared offline fixtures.
use super::*;

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
            assert!(text.contains("Program Modbus Bridge"));
            if connected && index == 0 {
                assert!(text.contains("Matches the Modbus Bridge"));
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
        let current = a.profiles[a.network_devices[0].profile].info.model.clone();
        click(&mut a, &ctx, &current);
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
    click(&mut a, &ctx, "Program Modbus Bridge");
    assert!(a.programming && a.programming_blocked && a.active.is_some());
    std::fs::remove_dir_all(root).unwrap();
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
