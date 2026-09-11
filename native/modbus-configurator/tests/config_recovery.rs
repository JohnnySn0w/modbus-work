use modbus_configurator::{config_file as files, contract::PortInfo};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Claim a separate directory for each test, independent of clock resolution.
fn root() -> PathBuf {
    let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    fs::create_dir_all(&parent).unwrap();
    loop {
        let path = parent.join(format!(
            "backup-recovery-{}-{}",
            std::process::id(),
            DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("Cannot create test directory {}: {error}", path.display()),
        }
    }
}

#[test]
fn parallel_recovery_tests_have_independent_directories() {
    let folders: Vec<_> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..16).map(|_| scope.spawn(root)).collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect()
    });
    assert_eq!(
        folders
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        16
    );
    for folder in folders {
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
        fs::remove_dir(folder).unwrap();
    }
}
#[test]
fn damaged_backup_and_interrupted_write_do_not_destroy_previous_configurations() {
    let root = root();
    let mut port = PortInfo {
        port: "COM5".into(),
        usb_vid: Some(0x483),
        usb_pid: Some(0x5740),
        serial_number: Some("recovery-unit".into()),
        description: "Fixture".into(),
        identity: None,
        busy: false,
    };
    let profiles = modbus_configurator::catalog::bundled().unwrap();
    let first = files::backup(&root, &port, &profiles[0].native_tsv).unwrap();
    let broken = files::backup(&root, &port, &profiles[1].native_tsv).unwrap();
    fs::write(&broken, b"incomplete configuration").unwrap();
    let folder = first.parent().unwrap();
    fs::write(folder.join(".exactaire-interrupted.tmp"), b"partial write").unwrap();
    fs::create_dir(folder.join("not-a-backup.tsv")).unwrap();
    assert_eq!(
        files::history(&root, &port).unwrap(),
        vec![broken.clone(), first.clone()]
    );
    port.port = "COM37".into();
    let recovered = files::backup(&root, &port, &profiles[0].native_tsv).unwrap();
    assert_ne!(first, recovered);
    assert_eq!(
        files::backup(&root, &port, &profiles[0].native_tsv).unwrap(),
        recovered
    );
    assert_eq!(files::load(&first).unwrap(), profiles[0].native_tsv);
    assert_eq!(fs::read(&broken).unwrap(), b"incomplete configuration");
    let selected = root.join("Selected.tsv");
    files::save(&selected, &profiles[1].native_tsv).unwrap();
    assert!(files::load(&broken).is_err());
    files::save(&selected, &files::load(&first).unwrap()).unwrap();
    assert_eq!(files::load(&selected).unwrap(), profiles[0].native_tsv);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn product_rename_migrates_backups_without_overwriting_newer_selection() {
    let root = root();
    let old = root.join("old");
    let new = root.join("new");
    files::migrate_from(&old, &new).unwrap();
    assert!(!new.exists());
    fs::create_dir_all(old.join("Backups/device")).unwrap();
    fs::write(old.join("Selected.tsv"), b"old selection").unwrap();
    fs::write(old.join("Backups/device/one.tsv"), b"backup").unwrap();
    fs::write(old.join("Backups/device/partial.tmp"), b"partial").unwrap();
    files::migrate_from(&old, &new).unwrap();
    assert_eq!(
        fs::read(new.join("Selected.tsv")).unwrap(),
        b"old selection"
    );
    assert_eq!(
        fs::read(new.join("Backups/device/one.tsv")).unwrap(),
        b"backup"
    );
    assert!(!new.join("Backups/device/partial.tmp").exists());
    fs::write(new.join("Selected.tsv"), b"new selection").unwrap();
    files::migrate_from(&old, &new).unwrap();
    assert_eq!(
        fs::read(new.join("Selected.tsv")).unwrap(),
        b"new selection"
    );
    assert_eq!(
        fs::read(old.join("Selected.tsv")).unwrap(),
        b"old selection"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_usb_descriptors_never_create_backup_directories() {
    let root = root();
    let table = &modbus_configurator::catalog::bundled().unwrap()[0].native_tsv;
    for (vid, pid, serial) in [
        (None, Some(1), "unit".to_string()),
        (Some(1), None, "unit".to_string()),
        (Some(1), Some(1), " ".to_string()),
        (Some(1), Some(1), "x".repeat(101)),
    ] {
        let port = PortInfo {
            port: "COM103".into(),
            usb_vid: vid,
            usb_pid: pid,
            serial_number: Some(serial),
            description: String::new(),
            identity: None,
            busy: false,
        };
        assert!(files::backup(&root, &port, table).is_err());
        assert!(files::history(&root, &port).is_err());
    }
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unreadable_configurations_and_missing_destination_leave_selection_intact() {
    let root = root();
    let table = &modbus_configurator::catalog::bundled().unwrap()[0].native_tsv;
    let selected = root.join("Selected.tsv");
    files::save(&selected, table).unwrap();
    for bytes in [vec![0xff, 0xfe], vec![b'a'; 32769]] {
        let invalid = root.join("invalid.tsv");
        fs::write(&invalid, bytes).unwrap();
        assert!(files::load(&invalid).is_err());
        assert_eq!(files::load(&selected).unwrap(), *table);
    }
    assert!(files::save(&root.join("missing/table.tsv"), table).is_err());
    assert!(!root.join("missing").exists());
    assert_eq!(files::load(&selected).unwrap(), *table);
    fs::remove_dir_all(root).unwrap();
}
