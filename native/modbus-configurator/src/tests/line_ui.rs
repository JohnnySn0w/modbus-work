//! Line-setting drafts survive polling and queue against the displayed target.
use super::*;
#[test]
fn line_settings_preserve_edits_and_queue_behind_polling() {
    let mut a = app();
    start(&mut a);
    a.last_scan = Instant::now();
    a.bridge_source = Some(port());
    a.result = Some(read(&a, 21.5));
    a.auto_request = true;
    let current = modbus_configurator::bridge::LineSettings {
        baud: 19200,
        data_bits: 8,
        parity: "None".into(),
        stop_bits: "2".into(),
        retries: 1,
        timeout_ms: 500,
        delay_ms: 150,
    };
    send(
        &mut a,
        EventKind::LineSettings {
            settings: current.clone(),
        },
    );
    assert_eq!(a.line_draft, Some(current.clone()));
    a.line_draft.as_mut().unwrap().baud = 38400;
    send(
        &mut a,
        EventKind::LineSettings {
            settings: current.clone(),
        },
    );
    assert_eq!(a.line_draft.as_ref().unwrap().baud, 38400);
    a.technician.page = technician_view::Page::Configurations;
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    click(&mut a, &ctx, "RS-485 line settings");
    click(&mut a, &ctx, "Apply line settings");
    assert!(
        matches!(&a.queued,Some((Operation::BridgeLineSettings{target,reviewed},route)) if target.baud==38400 && reviewed==&current && route.port==port().port),
        "queued={:?} active={:?} auto={} draft={:?} status={}",
        a.queued,
        a.active,
        a.auto_request,
        a.line_draft,
        a.status
    );
    assert!(!a.programming);
}
