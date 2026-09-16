//! Modbus Bridge programming scenarios using the shared scripted transport.
use super::*;

#[test]
fn unsupported_identity_and_unfinished_import_never_send_input() {
    for (initial, code) in [
        (
            fixture().initial.replace("3.6", "5.06"),
            ErrorCode::IdentityMismatch,
        ),
        (
            fixture().initial.replace("ENL-MOD-32", "IAQ"),
            ErrorCode::IdentityMismatch,
        ),
        (
            fixture()
                .initial
                .replace("Password:", "Finish the import with an empty line"),
            ErrorCode::UnsafeState,
        ),
        (
            "Modbus Configuration Menu:".into(),
            ErrorCode::IdentityMismatch,
        ),
    ] {
        let script = Script::new(
            Fixture {
                initial,
                exchanges: vec![],
            },
            7,
        );
        let writes = script.writes.clone();
        let error = BridgeSession::new(Box::new(script), timing())
            .run(&identity(), false, &AtomicBool::new(false), |_| {})
            .unwrap_err();
        assert_eq!(
            std::mem::discriminant(&error.code),
            std::mem::discriminant(&code)
        );
        assert!(writes.lock().unwrap().is_empty());
    }
}

#[test]
fn an_import_prompt_after_read_complete_is_not_acknowledged() {
    let mut f = fixture();
    f.exchanges[6]
        .1
        .push_str("\r\nFinish the import with an empty line");
    let script = Script::new(f, 16);
    let writes = script.writes.clone();
    let error = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::UnsafeState));
    assert_eq!(writes.lock().unwrap().last().unwrap(), "A\r");
}

#[test]
fn program_backs_up_before_mutation_and_verifies_replacement() {
    let (f, current, target) = programming_fixture();
    let script = Script::new(f, 7);
    let writes = script.writes.clone();
    let mut backed_up = false;
    let mut stages = vec![];
    let result = BridgeSession::new(Box::new(script), timing())
        .program(
            &identity(),
            &target,
            &current,
            &AtomicBool::new(false),
            |stage| stages.push(stage.to_owned()),
            |table| {
                assert_eq!(table, current);
                assert!(!writes.lock().unwrap().iter().any(|w| w == "I\r"));
                backed_up = true;
                Ok(())
            },
        )
        .unwrap();
    assert!(backed_up);
    for phase in [
        "Removing old point table",
        "Programming selected point table",
    ] {
        let counts: Vec<_> = stages
            .iter()
            .filter(|s| s.starts_with(&format!("{phase} (")))
            .cloned()
            .collect();
        assert_eq!(
            counts,
            vec![
                format!("{phase} (0/2)"),
                format!("{phase} (1/2)"),
                format!("{phase} (2/2)")
            ]
        );
    }
    assert_eq!(result.native_tsv, target);
    assert_eq!(result.successful_reads, None);
    assert_eq!(writes.lock().unwrap().len(), 20);
}

#[test]
fn program_refuses_invalid_or_stale_selection_and_failed_backup() {
    for failure in ["invalid", "stale", "backup", "cancelled", "identity"] {
        let (f, current, target) = programming_fixture();
        let script = Script::new(f, 17);
        let writes = script.writes.clone();
        let mut expected = identity();
        if failure == "identity" {
            expected.firmware = "unknown".into();
        }
        let error = BridgeSession::new(Box::new(script), timing())
            .program(
                &expected,
                if failure == "invalid" {
                    "invalid"
                } else {
                    &target
                },
                if failure == "stale" {
                    &target
                } else {
                    &current
                },
                &AtomicBool::new(failure == "cancelled"),
                |_| {},
                |_| Err(io::Error::other("disk full")),
            )
            .unwrap_err();
        assert!(!matches!(error.code, ErrorCode::ProgrammingUncertain));
        assert!(!writes.lock().unwrap().iter().any(|w| w == "I\r"));
    }
}

#[test]
fn program_missing_ack_and_readback_mismatch_stop_without_retry() {
    for bad in ["delete", "import", "readback"] {
        let (mut f, current, target) = programming_fixture();
        let index = if bad == "readback" {
            17
        } else if bad == "delete" {
            8
        } else {
            13
        };
        if bad == "readback" {
            f.exchanges[index].1 = format!("{current}Press a key to continue");
        } else {
            f.exchanges[index].1 = "Import rejected\r\n".into();
        }
        let script = Script::new(f, 11);
        let writes = script.writes.clone();
        let error = BridgeSession::new(Box::new(script), timing())
            .program(
                &identity(),
                &target,
                &current,
                &AtomicBool::new(false),
                |_| {},
                |_| Ok(()),
            )
            .unwrap_err();
        assert!(
            matches!(error.code, ErrorCode::ProgrammingUncertain),
            "{bad}: {error:?}"
        );
        assert_eq!(writes.lock().unwrap().len(), index + 1, "{bad}");
    }
}

#[test]
fn program_cancellation_after_backup_never_enters_import() {
    let (f, current, target) = programming_fixture();
    let script = Script::new(f, 17);
    let writes = script.writes.clone();
    let cancel = AtomicBool::new(false);
    let error = BridgeSession::new(Box::new(script), timing())
        .program(
            &identity(),
            &target,
            &current,
            &cancel,
            |_| {},
            |_| {
                cancel.store(true, Ordering::Release);
                Ok(())
            },
        )
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::Cancelled));
    assert!(!writes.lock().unwrap().iter().any(|w| w == "I\r"));
}

#[test]
fn program_disconnect_or_cancel_after_first_write_never_retransmits() {
    struct Interrupted {
        script: Script,
        cancel: Arc<AtomicBool>,
        disconnect: bool,
    }
    impl Transport for Interrupted {
        fn continuous_receive(&self) -> bool {
            true
        }
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.script.read(bytes)
        }
        fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.script.write(bytes)?;
            if bytes.contains(&b'\t') {
                if self.disconnect {
                    self.script.disconnect = true;
                } else {
                    self.cancel.store(true, Ordering::Release);
                }
            }
            Ok(())
        }
        fn reconnect(&mut self) -> io::Result<()> {
            panic!("Programming must not reopen or retry");
        }
    }
    for disconnect in [true, false] {
        let (f, current, target) = programming_fixture();
        let script = Script::new(f, 13);
        let writes = script.writes.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let transport = Interrupted {
            script,
            cancel: cancel.clone(),
            disconnect,
        };
        let error = BridgeSession::new(Box::new(transport), timing())
            .program(&identity(), &target, &current, &cancel, |_| {}, |_| Ok(()))
            .unwrap_err();
        assert!(matches!(error.code, ErrorCode::ProgrammingUncertain));
        assert_eq!(writes.lock().unwrap().len(), 9);
    }
}

#[test]
fn captured_import_completion_requires_the_final_continue_prompt() {
    let captured = include_str!("../fixtures/bridge-import-finished.txt");
    for complete in [true, false] {
        let (mut fixture, current, target) = programming_fixture();
        fixture.exchanges[15].1 = if complete {
            captured.into()
        } else {
            captured.split("Press a key").next().unwrap().into()
        };
        let script = Script::new(fixture, 3);
        let writes = script.writes.clone();
        let result = BridgeSession::new(Box::new(script), timing()).program(
            &identity(),
            &target,
            &current,
            &AtomicBool::new(false),
            |_| {},
            |_| Ok(()),
        );
        if complete {
            assert_eq!(result.unwrap().native_tsv, target);
        } else {
            assert!(matches!(
                result.unwrap_err().code,
                ErrorCode::ProgrammingUncertain
            ));
            assert_eq!(writes.lock().unwrap().len(), 16);
        }
    }
}

#[test]
fn fresh_session_can_leave_previous_read_completion_without_import_writes() {
    let original = fixture();
    let mut f = original.clone();
    f.initial = "Modbus Read Completed\r\nPress a key to continue".into();
    f.exchanges.insert(0, ("\r".into(), original.initial));
    let script = Script::new(f, 67);
    let writes = script.writes.clone();
    let result = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(result.successful_reads, Some(2));
    assert_eq!(writes.lock().unwrap()[0], "\r");
}

#[test]
fn empty_program_target_and_oversized_responses_stop_before_mutation() {
    let (f, reviewed, _) = programming_fixture();
    let script = Script::new(f, 67);
    let writes = script.writes.clone();
    let error = BridgeSession::new(Box::new(script), timing())
        .program(
            &identity(),
            "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\n",
            &reviewed,
            &AtomicBool::new(false),
            |_| {},
            |_| panic!("No backup for invalid target"),
        )
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::InvalidRequest));
    assert!(writes.lock().unwrap().is_empty());
    let mut f = fixture();
    f.initial = "x".repeat(modbus_configurator::console::MAX_RESPONSE_BYTES + 1);
    let script = Script::new(f, 1024);
    let writes = script.writes.clone();
    // This tests the byte limit, not parser throughput under coverage instrumentation.
    let limit_timing = Timing {
        response: Duration::from_secs(5),
        operation: Duration::from_secs(10),
        ..timing()
    };
    let error = BridgeSession::new(Box::new(script), limit_timing)
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::InvalidResponse));
    assert!(writes.lock().unwrap().is_empty());
}

#[test]
fn failed_navigation_after_backup_before_import_is_not_a_partial_program() {
    let (mut f, reviewed, target) = programming_fixture();
    f.exchanges[6].1 = "Password:".into();
    let script = Script::new(f, 67);
    let writes = script.writes.clone();
    let error = BridgeSession::new(Box::new(script), timing())
        .program(
            &identity(),
            &target,
            &reviewed,
            &AtomicBool::new(false),
            |_| {},
            |_| Ok(()),
        )
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::UnsafeState));
    assert!(!writes.lock().unwrap().iter().any(|w| w == "I\r"));
}
