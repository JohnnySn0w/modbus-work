//! E5 bridge service scenarios using the shared scripted transport.
use super::*;

#[test]
fn delayed_usb_reply_is_recovered_without_resending_a_command() {
    struct ReopenReply {
        script: Script,
        waiting: bool,
        reopened: Arc<AtomicUsize>,
    }
    impl Transport for ReopenReply {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if self.waiting {
                std::thread::sleep(Duration::from_millis(1));
                return Err(io::ErrorKind::TimedOut.into());
            }
            self.script.read(bytes)
        }
        fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.script.write(bytes)?;
            self.waiting = true;
            Ok(())
        }
        fn reconnect(&mut self) -> io::Result<()> {
            self.reopened.fetch_add(1, Ordering::SeqCst);
            self.waiting = false;
            Ok(())
        }
    }
    let f = fixture();
    let expected: Vec<_> = f
        .exchanges
        .iter()
        .map(|(command, _)| command.clone())
        .collect();
    let script = Script::new(f, 1024);
    let writes = script.writes.clone();
    let reopened = Arc::new(AtomicUsize::new(0));
    let mut session = BridgeSession::new(
        Box::new(ReopenReply {
            script,
            waiting: false,
            reopened: reopened.clone(),
        }),
        Timing {
            response: Duration::from_millis(10),
            ..timing()
        },
    );
    // Export avoids the independently longer Read All deadline.
    session
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(*writes.lock().unwrap(), expected[..6]);
    assert_eq!(reopened.load(Ordering::SeqCst), 6);
}

#[test]
fn inventory_and_cancellation_remain_responsive_while_port_is_busy() {
    let script = Script::new(
        Fixture {
            initial: String::new(),
            exchanges: vec![],
        },
        1,
    );
    let drops = script.drops.clone();
    let backend = FakeBackend::new(vec![script]);
    let service = Service::with_backend(
        backend.clone(),
        Timing {
            response: Duration::from_secs(2),
            ..timing()
        },
    )
    .unwrap();
    service.send(command(1, Operation::BridgeExport)).unwrap();
    loop {
        let event = service.events.recv_timeout(Duration::from_secs(1)).unwrap();
        if matches!(event.kind, EventKind::Progress { stage } if stage == "Checking E5 bridge console and identity")
        {
            break;
        }
    }
    service.send(command(2, Operation::BridgeExport)).unwrap();
    assert!(matches!(
        terminal(&service, 2),
        EventKind::Error {
            code: ErrorCode::PortBusy,
            ..
        }
    ));
    service.send(command(3, Operation::Inventory)).unwrap();
    assert!(
        matches!(terminal(&service, 3), EventKind::PortSnapshot { ports } if ports.len() == 1 && ports[0].busy)
    );
    service
        .send(command(4, Operation::Cancel { request_id: 1 }))
        .unwrap();
    assert!(matches!(
        terminal(&service, 1),
        EventKind::Error {
            code: ErrorCode::Cancelled,
            ..
        }
    ));
    assert_eq!(backend.opens.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn unplugged_inventory_releases_idle_session() {
    let script = Script::new(fixture(), 6);
    let drops = script.drops.clone();
    let backend = FakeBackend::new(vec![script]);
    let service = Service::with_backend(backend.clone(), timing()).unwrap();
    service.send(command(1, Operation::BridgeReadAll)).unwrap();
    assert!(matches!(
        terminal(&service, 1),
        EventKind::BridgeResult { .. }
    ));
    backend.present.store(false, Ordering::SeqCst);
    service.send(command(2, Operation::Inventory)).unwrap();
    assert!(matches!(terminal(&service, 2), EventKind::PortSnapshot { ports } if ports.is_empty()));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn inventory_never_opens_a_port_and_zero_cancel_target_is_rejected() {
    let backend = FakeBackend::new(vec![]);
    let service = Service::with_backend(backend.clone(), timing()).unwrap();
    service.send(command(1, Operation::Inventory)).unwrap();
    assert!(matches!(
        terminal(&service, 1),
        EventKind::PortSnapshot { .. }
    ));
    service
        .send(command(2, Operation::Cancel { request_id: 0 }))
        .unwrap();
    assert!(matches!(
        terminal(&service, 2),
        EventKind::Error {
            code: ErrorCode::InvalidRequest,
            ..
        }
    ));
    assert_eq!(backend.opens.load(Ordering::SeqCst), 0);
}

#[test]
fn actual_backup_storage_failure_prevents_any_programming_write() {
    let (fixture, current, target) = programming_fixture();
    let script = Script::new(fixture, 17);
    let writes = script.writes.clone();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("blocked-backup-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let blocked = root.join("not-a-directory");
    std::fs::write(&blocked, "existing file must survive").unwrap();
    let port = modbus_configurator::contract::PortInfo {
        port: "COM27".into(),
        usb_vid: Some(0x483),
        usb_pid: Some(0x5740),
        serial_number: Some("storage-test".into()),
        description: String::new(),
        identity: None,
        busy: false,
    };
    let error = BridgeSession::new(Box::new(script), timing())
        .program(
            &identity(),
            &target,
            &current,
            &AtomicBool::new(false),
            |_| {},
            |table| modbus_configurator::config_file::backup(&blocked, &port, table).map(|_| ()),
        )
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::UnsafeState));
    assert!(
        error
            .message
            .contains("Backup failed; no changes were made")
    );
    assert!(
        !writes
            .lock()
            .unwrap()
            .iter()
            .any(|w| w == "I\r" || w.contains('\t'))
    );
    assert_eq!(
        std::fs::read_to_string(blocked).unwrap(),
        "existing file must survive"
    );
}

#[test]
fn replaced_usb_is_rejected_without_an_inventory_command_or_serial_write() {
    struct Replaced {
        inner: Arc<FakeBackend>,
        changed: AtomicBool,
    }
    impl Backend for Replaced {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            let mut ports = self.inner.inventory()?;
            for p in &mut ports {
                p.serial_number = Some(
                    if self.changed.load(Ordering::SeqCst) {
                        "replacement"
                    } else {
                        "original"
                    }
                    .into(),
                );
            }
            Ok(ports)
        }
        fn open(&self, p: &str) -> io::Result<Box<dyn Transport>> {
            self.inner.open(p)
        }
    }
    let script = Script::new(fixture(), 67);
    let drops = script.drops.clone();
    let writes = script.writes.clone();
    let backend = Arc::new(Replaced {
        inner: FakeBackend::new(vec![script]),
        changed: AtomicBool::new(false),
    });
    let service = Service::with_backend(backend.clone(), timing()).unwrap();
    service.send(command(1, Operation::BridgeReadAll)).unwrap();
    assert!(matches!(
        terminal(&service, 1),
        EventKind::BridgeResult { .. }
    ));
    let count = writes.lock().unwrap().len();
    backend.changed.store(true, Ordering::SeqCst);
    service.send(command(2, Operation::BridgeReadAll)).unwrap();
    assert!(matches!(
        terminal(&service, 2),
        EventKind::Error {
            code: ErrorCode::IdentityMismatch,
            ..
        }
    ));
    assert_eq!(writes.lock().unwrap().len(), count);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn service_program_requires_durable_backup_and_reports_storage_before_result() {
    struct Stored {
        inner: Arc<FakeBackend>,
        mode: &'static str,
    }
    impl Backend for Stored {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            self.inner.inventory()
        }
        fn open(&self, p: &str) -> io::Result<Box<dyn Transport>> {
            self.inner.open(p)
        }
        fn backup(&self, _: &PortInfo, _: &str) -> io::Result<Option<std::path::PathBuf>> {
            match self.mode {
                "missing" => Ok(None),
                "failed" => Err(io::Error::other("disk full")),
                _ => Ok(Some("test-backup.tsv".into())),
            }
        }
    }
    for mode in ["stored", "missing", "failed"] {
        let (f, reviewed, target) = programming_fixture();
        let script = Script::new(f, 67);
        let writes = script.writes.clone();
        let service = Service::with_backend(
            Arc::new(Stored {
                inner: FakeBackend::new(vec![script]),
                mode,
            }),
            timing(),
        )
        .unwrap();
        service
            .send(Command {
                request_id: 1,
                port: Some("COM5".into()),
                expected_identity: Some(identity()),
                operation: Operation::BridgeProgram {
                    target: target.clone(),
                    reviewed,
                },
            })
            .unwrap();
        let mut backup_seen = false;
        loop {
            let event = service.events.recv_timeout(Duration::from_secs(3)).unwrap();
            match event.kind {
                EventKind::Backup { path, error } => {
                    assert!(path.is_some());
                    assert!(error.is_none());
                    backup_seen = true;
                }
                EventKind::BridgeResult { result } => {
                    assert_eq!(mode, "stored");
                    assert!(backup_seen);
                    assert_eq!(result.native_tsv, target);
                    break;
                }
                EventKind::Error {
                    code, recoverable, ..
                } => {
                    assert_ne!(mode, "stored");
                    assert!(matches!(code, ErrorCode::UnsafeState));
                    assert!(!recoverable);
                    assert!(!backup_seen);
                    assert!(!writes.lock().unwrap().iter().any(|w| w == "I\r"));
                    break;
                }
                _ => {}
            }
        }
    }
}

#[test]
fn failed_automatic_backup_does_not_discard_verified_readings() {
    struct NoSpace(Arc<FakeBackend>);
    impl Backend for NoSpace {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            self.0.inventory()
        }
        fn open(&self, p: &str) -> io::Result<Box<dyn Transport>> {
            self.0.open(p)
        }
        fn backup(&self, _: &PortInfo, _: &str) -> io::Result<Option<std::path::PathBuf>> {
            Err(io::Error::other("disk full"))
        }
    }
    let service = Service::with_backend(
        Arc::new(NoSpace(FakeBackend::new(vec![Script::new(fixture(), 67)]))),
        timing(),
    )
    .unwrap();
    service.send(command(1, Operation::BridgeReadAll)).unwrap();
    let mut reported = false;
    loop {
        match service
            .events
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
            .kind
        {
            EventKind::Backup { path, error } => {
                assert!(path.is_none());
                assert!(error.unwrap().contains("disk full"));
                reported = true;
            }
            EventKind::BridgeResult { result } => {
                assert!(reported);
                assert_eq!(result.successful_reads, Some(2));
                break;
            }
            EventKind::Error { message, .. } => panic!("Read incorrectly failed: {message}"),
            _ => {}
        }
    }
}

#[test]
fn inventory_and_open_errors_are_reported_without_worker_panics() {
    struct Broken {
        inventory_fails: bool,
        opens: Arc<AtomicUsize>,
    }
    impl Backend for Broken {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            if self.inventory_fails {
                Err("inventory failed".into())
            } else {
                FakeBackend::new(vec![]).inventory()
            }
        }
        fn open(&self, _: &str) -> io::Result<Box<dyn Transport>> {
            self.opens.fetch_add(1, Ordering::SeqCst);
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "busy elsewhere",
            ))
        }
    }
    for inventory_fails in [true, false] {
        let opens = Arc::new(AtomicUsize::new(0));
        let service = Service::with_backend(
            Arc::new(Broken {
                inventory_fails,
                opens: opens.clone(),
            }),
            timing(),
        )
        .unwrap();
        service.send(command(1, Operation::BridgeReadAll)).unwrap();
        let result = terminal(&service, 1);
        if inventory_fails {
            assert!(matches!(
                result,
                EventKind::Error {
                    code: ErrorCode::InventoryUnavailable,
                    ..
                }
            ));
            assert_eq!(opens.load(Ordering::SeqCst), 0);
        } else {
            assert!(matches!(
                result,
                EventKind::Error {
                    code: ErrorCode::Transport,
                    ..
                }
            ));
            assert_eq!(opens.load(Ordering::SeqCst), 1);
        }
    }
}

#[test]
fn repeated_usb_disconnects_and_com_moves_revalidate_without_replaying_commands() {
    struct Moving {
        inner: Arc<FakeBackend>,
        route: Mutex<String>,
    }
    impl Backend for Moving {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            let mut ports = self.inner.inventory()?;
            for p in &mut ports {
                p.port = self.route.lock().unwrap().clone();
                p.serial_number = Some("same-physical-unit".into());
            }
            Ok(ports)
        }
        fn open(&self, p: &str) -> io::Result<Box<dyn Transport>> {
            assert_eq!(p, *self.route.lock().unwrap());
            self.inner.open(p)
        }
    }
    let scripts: Vec<_> = (0..8).map(|_| Script::new(fixture(), 9)).collect();
    let counters: Vec<_> = scripts
        .iter()
        .map(|s| (s.drops.clone(), s.writes.clone()))
        .collect();
    let backend = Arc::new(Moving {
        inner: FakeBackend::new(scripts),
        route: Mutex::new("COM5".into()),
    });
    let service = Service::with_backend(backend.clone(), timing()).unwrap();
    for (cycle, (drops, writes)) in counters.iter().enumerate() {
        let route = format!("COM{}", 5 + cycle * 7);
        *backend.route.lock().unwrap() = route.clone();
        backend.inner.present.store(true, Ordering::SeqCst);
        let id = cycle as u64 * 2 + 1;
        let mut read = command(id, Operation::BridgeReadAll);
        read.port = Some(route);
        service.send(read).unwrap();
        assert!(
            matches!(terminal(&service,id),EventKind::BridgeResult {result} if result.successful_reads==Some(2))
        );
        backend.inner.present.store(false, Ordering::SeqCst);
        service.send(command(id + 1, Operation::Inventory)).unwrap();
        assert!(
            matches!(terminal(&service,id+1),EventKind::PortSnapshot {ports} if ports.is_empty())
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(writes.lock().unwrap().len(), 8);
    }
    assert_eq!(backend.inner.opens.load(Ordering::SeqCst), 8);
}
