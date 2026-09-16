//! E5 bridge recovery scenarios using the shared scripted transport.
use super::*;

#[test]
fn stalled_receive_resumes_on_same_transport_without_resending_commands() {
    struct Paused {
        script: Script,
        started: bool,
        resumed: bool,
    }
    impl Transport for Paused {
        fn continuous_receive(&self) -> bool {
            true
        }
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            if self.started && !self.resumed {
                std::thread::sleep(Duration::from_millis(10));
                return Err(io::ErrorKind::TimedOut.into());
            }
            self.started = true;
            self.script.read(b)
        }
        fn write(&mut self, b: &[u8]) -> io::Result<()> {
            self.script.write(b)
        }
        fn resume_receive(&mut self) -> io::Result<()> {
            self.resumed = true;
            Ok(())
        }
        fn reconnect(&mut self) -> io::Result<()> {
            panic!("Must keep the physical connection open")
        }
    }
    let script = Script::new(fixture(), 15);
    let writes = script.writes.clone();
    let result = BridgeSession::new(
        Box::new(Paused {
            script,
            started: false,
            resumed: false,
        }),
        Timing {
            response: Duration::from_secs(2),
            quiet: Duration::from_millis(2),
            operation: Duration::from_secs(10),
        },
    )
    .run(&identity(), true, &AtomicBool::new(false), |_| {})
    .unwrap();
    assert_eq!(result.successful_reads, Some(2));
    assert_eq!(writes.lock().unwrap().len(), 8);
}

#[test]
fn partial_replies_survive_multiple_reopens_without_resending_input() {
    struct Segments {
        script: Script,
        active: bool,
        blocked: bool,
    }
    impl Transport for Segments {
        fn requires_menu_selection_prompt(&self) -> bool {
            true
        }
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if self.blocked {
                std::thread::sleep(Duration::from_millis(1));
                return Err(io::ErrorKind::TimedOut.into());
            }
            let result = self.script.read(bytes);
            if self.active && result.is_ok() && !self.script.pending.is_empty() {
                self.blocked = true;
            }
            result
        }
        fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.script.write(bytes)?;
            self.active = true;
            self.blocked = false;
            Ok(())
        }
        fn reconnect(&mut self) -> io::Result<()> {
            self.blocked = false;
            Ok(())
        }
    }
    let mut f = fixture();
    for (_, response) in &mut f.exchanges {
        if response.contains("Menu:") && !response.contains("Enter Selection") {
            response.push_str("\nEnter Selection:");
        }
    }
    let expected: Vec<_> = f
        .exchanges
        .iter()
        .map(|(command, _)| command.clone())
        .collect();
    let script = Script::new(f, 17);
    let writes = script.writes.clone();
    let result = BridgeSession::new(
        Box::new(Segments {
            script,
            active: false,
            blocked: false,
        }),
        timing(),
    )
    .run(&identity(), true, &AtomicBool::new(false), |_| {})
    .unwrap();
    assert_eq!(result.successful_reads, Some(2));
    assert_eq!(*writes.lock().unwrap(), expected);
}

#[test]
fn observed_submenu_exit_recovers_fresh_identity_without_config_commands() {
    let mut f = fixture();
    f.exchanges.insert(0, ("X\r".into(), f.initial.clone()));
    f.exchanges.insert(
        0,
        (
            "X\r".into(),
            "enLink Main Menu:\nX - Exit and log off".into(),
        ),
    );
    f.initial = "Modbus Configuration Menu:\nX  - Exit Menu".into();
    BridgeSession::new(Box::new(Script::new(f, 32)), timing())
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn stale_submenu_recovers_with_bounded_exit_commands() {
    let mut f = fixture();
    f.initial = f.initial.replace(
        "Password:",
        "enLink Main Menu:\r\nModbus Import/Export Menu:",
    );
    f.exchanges[0] = ("X\r".into(), "Modbus Configuration Menu:".into());
    f.exchanges
        .insert(1, ("X\r".into(), "enLink Main Menu:".into()));
    let mut session = BridgeSession::new(Box::new(Script::new(f, 3)), timing());
    session
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn actor_reuses_one_exclusive_session_and_release_closes_it() {
    let mut f = fixture();
    f.exchanges.truncate(6);
    let mut second = f.exchanges[1..].to_vec();
    f.exchanges.push(("X\r".into(), "enLink Main Menu:".into()));
    f.exchanges.append(&mut second);
    let script = Script::new(f, 9);
    let drops = script.drops.clone();
    let backend = FakeBackend::new(vec![script]);
    let service = Service::with_backend(backend.clone(), timing()).unwrap();
    for id in [1, 2] {
        service.send(command(id, Operation::BridgeExport)).unwrap();
        assert!(matches!(
            terminal(&service, id),
            EventKind::BridgeResult { .. }
        ));
    }
    assert_eq!(backend.opens.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    service.send(command(3, Operation::ClosePort)).unwrap();
    assert!(matches!(terminal(&service, 3), EventKind::Result { .. }));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn disconnect_drops_session_and_retry_opens_fresh_identity() {
    let mut disconnected = Script::new(fixture(), 4);
    disconnected.disconnect = true;
    let backend = FakeBackend::new(vec![disconnected, Script::new(fixture(), 4)]);
    let service = Service::with_backend(backend.clone(), timing()).unwrap();
    service.send(command(1, Operation::BridgeReadAll)).unwrap();
    assert!(matches!(
        terminal(&service, 1),
        EventKind::Error {
            code: ErrorCode::Transport,
            ..
        }
    ));
    service.send(command(2, Operation::BridgeReadAll)).unwrap();
    assert!(matches!(
        terminal(&service, 2),
        EventKind::BridgeResult { .. }
    ));
    assert_eq!(backend.opens.load(Ordering::SeqCst), 2);
}

#[test]
fn repeated_login_prompt_stops_at_recovery_limit() {
    let mut f = fixture();
    f.exchanges = vec![("cafe\r".into(), "Password:".into()); 8];
    let script = Script::new(f, 8);
    let writes = script.writes.clone();
    let error = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::UnsafeState));
    assert_eq!(writes.lock().unwrap().len(), 7);
}

#[test]
fn fresh_silent_console_receives_one_wake_before_identity_check() {
    let mut f = fixture();
    let banner = std::mem::take(&mut f.initial);
    f.exchanges.insert(0, ("\r".into(), banner));
    let script = Script::new(f, 11);
    let writes = script.writes.clone();
    let result = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(result.successful_reads, Some(2));
    assert_eq!(writes.lock().unwrap()[0], "\r");
    assert_eq!(writes.lock().unwrap().len(), 9);
}

#[test]
fn silent_wake_is_not_repeated_and_unknown_output_is_not_woken() {
    for initial in ["", "unrecognized console prompt"] {
        let exchanges = if initial.is_empty() {
            vec![("\r".into(), String::new())]
        } else {
            vec![]
        };
        let script = Script::new(
            Fixture {
                initial: initial.into(),
                exchanges,
            },
            16,
        );
        let writes = script.writes.clone();
        let error = BridgeSession::new(Box::new(script), timing())
            .run(&identity(), false, &AtomicBool::new(false), |_| {})
            .unwrap_err();
        assert!(matches!(error.code, ErrorCode::Timeout));
        assert_eq!(
            writes.lock().unwrap().len(),
            usize::from(initial.is_empty())
        );
    }
}
