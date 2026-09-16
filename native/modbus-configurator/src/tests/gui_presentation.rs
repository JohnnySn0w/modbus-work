//! GUI presentation regression tests using the shared offline fixtures.
use super::*;

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
fn renamed_brand_progress_and_unit_conversion_preserve_native_readings() {
    assert_eq!(
        transfer_progress("Programming selected point table (3/8)"),
        Some(0.375)
    );
    for stage in [
        "Reading native Modbus Bridge point table",
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
                "Reading all configured Modbus points through the Modbus Bridge",
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
