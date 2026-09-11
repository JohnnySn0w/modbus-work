//! Saved display profiles persist partial definitions without binding them to a COM port or slave.
use modbus_configurator::{
    catalog::bundled,
    custom_profile::{self, SavedProfile},
    network::{self, Device},
};

#[test]
fn saved_partial_profile_reloads_and_follows_registers_across_slaves() {
    let catalog = bundled().unwrap();
    let mut device = Device::new(0, 1, &catalog);
    device.points = device.points.iter().take(2).copied().collect();
    let table = network::compose(&[device.clone()], &catalog).unwrap();
    let saved = SavedProfile::new("DPT146 short set", &catalog[0].info.id, &table, 1).unwrap();
    let root = std::env::temp_dir().join(format!("custom-profile-{}", std::process::id()));
    custom_profile::save(&root, std::slice::from_ref(&saved), &catalog).unwrap();
    let loaded = custom_profile::load(&root, &catalog).unwrap();
    assert_eq!(loaded, vec![saved.clone()]);
    device.slave = 42;
    let moved = network::compose(&[device], &catalog).unwrap();
    assert!(saved.matches(&moved, 42));
    assert_eq!(
        custom_profile::matching_model(&loaded, &moved, 42, &catalog),
        Some(0)
    );
    assert!(!saved.matches(&moved.replace("\tF32\t", "\tS32\t"), 42));
    let bad = SavedProfile {
        model_id: "missing-model".into(),
        ..saved
    };
    assert!(custom_profile::save(&root, &[bad], &catalog).is_err());
    assert_eq!(custom_profile::load(&root, &catalog).unwrap(), loaded);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn conflicting_interpretations_require_selection_instead_of_guessing() {
    let catalog = bundled().unwrap();
    let table = &catalog[0].native_tsv;
    let a = SavedProfile::new("First interpretation", &catalog[0].info.id, table, 1).unwrap();
    let b = SavedProfile::new("Second interpretation", &catalog[1].info.id, table, 1).unwrap();
    assert_eq!(
        custom_profile::matching_model(&[a, b], table, 1, &catalog),
        None
    );
    assert!(SavedProfile::new("Empty", &catalog[0].info.id, table, 247).is_err());
    assert!(SavedProfile::new("../invalid", &catalog[0].info.id, table, 1).is_err());
}

#[test]
fn invalid_libraries_and_failed_replacements_preserve_existing_storage() {
    let catalog = bundled().unwrap();
    let root = std::env::temp_dir().join(format!("profile-invalid-{}", std::process::id()));
    assert!(custom_profile::load(&root, &catalog).unwrap().is_empty());
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("Custom-profiles.json");
    for bytes in [b"not json".to_vec(), vec![b' '; 262145]] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(custom_profile::load(&root, &catalog).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let mut profile =
        SavedProfile::new("Valid", &catalog[0].info.id, &catalog[0].native_tsv, 1).unwrap();
    profile.name = " Valid ".into();
    assert!(custom_profile::save(&root, &[profile.clone()], &catalog).is_err());
    profile.name = "Valid".into();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(custom_profile::save(&root, &[profile], &catalog).is_err());
    assert!(path.is_dir());
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        1,
        "Failed replacement must remove its temporary file"
    );
    assert!(custom_profile::load(&root, &catalog).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
