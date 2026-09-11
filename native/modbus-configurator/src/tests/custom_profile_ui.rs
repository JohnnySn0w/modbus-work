//! Profile controls persist interpretations and protect existing libraries on failure.
use super::*;
use crate::gui_event_tests::{app, click, draw, read};

#[test]
fn save_load_and_duplicate_name_preserve_the_selected_interpretation() {
    let mut a = app();
    a.last_scan = std::time::Instant::now();
    let root = std::env::temp_dir().join(format!("profile-controls-{}", std::process::id()));
    a.technician.load_custom_profiles(&root, &a.profiles);
    let result = read(&a, 12.5);
    let table = result.native_tsv.clone();
    a.result = Some(result);
    a.technician.page = Page::Detail("slave:1".into());
    a.technician.custom_profile_name = "Named sensor".into();
    let ctx = egui::Context::default();
    ctx.style_mut(|s| s.animation_time = 0.0);
    click(&mut a, &ctx, "Custom profiles");
    click(&mut a, &ctx, "Save custom profile");
    assert_eq!(a.technician.saved_profiles.len(), 1);
    let original = std::fs::read(root.join("Custom-profiles.json")).unwrap();
    assert!(draw(&mut a, &ctx).contains("Saved profile: Named sensor"));
    click(&mut a, &ctx, "Save custom profile");
    assert!(draw(&mut a, &ctx).contains("already saved"));
    assert_eq!(
        std::fs::read(root.join("Custom-profiles.json")).unwrap(),
        original
    );
    a.technician.custom_profile_name.clear();
    click(&mut a, &ctx, "Load custom profile");
    click(&mut a, &ctx, "Named sensor");
    assert!(matches!(
        a.technician.network_types.get(&(table, 1)),
        Some(network_readings::TypeChoice::Profile(0))
    ));
    assert!(draw(&mut a, &ctx).contains("Using profile: Named sensor"));
    assert!(a.queued.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn corrupted_library_disables_saving_without_overwriting_the_file() {
    let mut view = TechnicianView::default();
    let root = std::env::temp_dir().join(format!("profile-corrupt-ui-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("Custom-profiles.json");
    std::fs::write(&path, b"broken file").unwrap();
    view.load_custom_profiles(&root, &modbus_configurator::catalog::bundled().unwrap());
    assert!(view.custom_profile_root.is_none());
    assert!(
        view.custom_profile_message
            .contains("Could not load custom profiles")
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"broken file");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn hmd65_bridge_values_keep_reference_mapping_with_one_based_tables() {
    let a = app();
    for index in [1, 3] {
        let profile = &a.profiles[index];
        let mut result = read(&a, 45.0);
        result.native_tsv = profile.native_tsv.clone();
        let address = profile.point_register(1).unwrap().range().unwrap().0;
        let register = a.reference.registers["hmd65"]
            .iter()
            .find(|r| r.first_pdu() == Some(address))
            .unwrap();
        assert_eq!(value(Some(profile), Some(&result), register), Some(45.0));
        let error = a.reference.registers["hmd65"]
            .iter()
            .find(|r| r.name == "Error code")
            .unwrap();
        assert_eq!(value(Some(profile), Some(&result), error), None);
    }
}
