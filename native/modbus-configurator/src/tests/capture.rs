use super::*;
#[test]
fn capture_waits_for_readiness_and_commits_all_views_once() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!(
            "capture-test-{}-{}",
            std::process::id(),
            modbus_configurator::history::now_ms()
        ));
    let reference = modbus_configurator::reference::Reference::bundled().unwrap();
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    let mut c = Capture::new(root.clone(), &reference, &profiles);
    let ctx = egui::Context::default();
    assert!(c.update(&ctx, false).is_none());
    assert!(!c.started);
    for _ in 0..150 {
        let mut input = egui::RawInput::default();
        if c.frames == 5 {
            input.events.push(egui::Event::Screenshot {
                viewport_id: egui::ViewportId::ROOT,
                user_data: Default::default(),
                image: std::sync::Arc::new(egui::ColorImage::new(
                    [2, 2],
                    vec![egui::Color32::WHITE; 4],
                )),
            });
        }
        let _ = ctx.run(input, |ctx| {
            c.update(ctx, true);
        });
        if c.done {
            break;
        }
    }
    assert!(c.done);
    let names: Vec<String> =
        serde_json::from_slice(&std::fs::read(root.join("complete.json")).unwrap()).unwrap();
    assert_eq!(names.len(), 23);
    for name in &names {
        let img = image::open(root.join(name)).unwrap();
        assert_eq!((img.width(), img.height()), (2, 2));
    }
    assert!(c.update(&ctx, true).is_none());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 24);
    std::fs::remove_dir_all(root).unwrap();
}
