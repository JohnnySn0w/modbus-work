use modbus_configurator::{
    bridge::{BridgeSession, Timing, parse_export},
    contract::*,
    service::{Backend, Service},
    transport::Transport,
};
use serde::Deserialize;
use std::{
    collections::VecDeque,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Deserialize)]
struct Fixture {
    initial: String,
    exchanges: Vec<(String, String)>,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/bridge-read-all.json")).unwrap()
}
fn identity() -> Identity {
    Identity {
        model: "ENL-MOD-32".into(),
        firmware: "3.6".into(),
    }
}

#[test]
fn captured_truncated_detailed_output_is_not_accepted_as_a_complete_read() {
    let mut f = fixture();
    let (_, response) = f
        .exchanges
        .iter_mut()
        .find(|(command, _)| command == "A\r")
        .unwrap();
    *response = include_str!("fixtures/bridge-detailed-exceptions.txt").into();
    let error = BridgeSession::new(Box::new(Script::new(f, 67)), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::InvalidResponse));
    assert!(error.message.contains("reading without a point header"));
}
fn timing() -> Timing {
    Timing {
        response: Duration::from_millis(150),
        quiet: Duration::from_millis(2),
        operation: Duration::from_secs(2),
    }
}

#[test]
fn steady_polling_reuses_table_and_only_issues_read_commands() {
    let mut f = fixture();
    let read = f.exchanges[6..].to_vec();
    for _ in 0..20 {
        f.exchanges.extend(read.clone());
    }
    let script = Script::new(f, 67);
    let writes = script.writes.clone();
    let mut session = BridgeSession::new(Box::new(script), timing());
    let mut exports = 0;
    for _ in 0..21 {
        let r = session
            .poll_with_export(
                &identity(),
                true,
                &AtomicBool::new(false),
                |_| {},
                |_| exports += 1,
            )
            .unwrap();
        assert_eq!(r.successful_reads, Some(2));
    }
    assert_eq!(exports, 1);
    assert_eq!(writes.lock().unwrap().len(), 8 + 20 * 2);
}

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
fn validated_export_is_available_for_backup_even_if_measurements_fail() {
    let mut f = fixture();
    let (_, response) = f
        .exchanges
        .iter_mut()
        .find(|(command, _)| command == "A\r")
        .unwrap();
    *response = include_str!("fixtures/bridge-detailed-exceptions.txt").into();
    let mut backed_up = None;
    let result = BridgeSession::new(Box::new(Script::new(f, 67)), timing()).run_with_export(
        &identity(),
        true,
        &AtomicBool::new(false),
        |_| {},
        |table| backed_up = Some(table.to_owned()),
    );
    assert!(result.is_err());
    let table = backed_up.unwrap();
    assert_eq!(parse_export(&table, 2).unwrap().len(), 2);
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
fn password_only_console_requires_redrawn_identity_before_derived_login() {
    let mut f = fixture();
    f.exchanges.insert(0, ("\r".into(), f.initial.clone()));
    f.initial = "Password:".into();
    BridgeSession::new(Box::new(Script::new(f, 32)), timing())
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn complete_mixed_read_is_returned_only_with_matching_bridge_summary() {
    let mut f = fixture();
    let index = f.exchanges.iter().position(|(c, _)| c == "A\r").unwrap();
    f.exchanges[index].1 = "--- [1] ID:1 Reg:Hold Addr:4 Data:F32 HL\n--- Reading: 23.5\n--- [3] ID:1 Reg:Hold Addr:512 Data:U16 HH\n--- Exception: [2] 'Illegal Data Address'\nModbus read completed\nPress a key to continue".into();
    f.exchanges[index + 1].1 = "Modbus Configuration Menu:\n1/1 (OK/Exceptions)".into();
    let result = BridgeSession::new(Box::new(Script::new(f.clone(), 64)), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(result.successful_reads, Some(1));
    assert_eq!(result.exceptions[0].item, 3);
    f.exchanges[index + 1].1 = "Modbus Configuration Menu:\n2/0 (OK/Exceptions)".into();
    assert!(
        BridgeSession::new(Box::new(Script::new(f, 64)), timing())
            .run(&identity(), true, &AtomicBool::new(false), |_| {})
            .is_err()
    );
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
fn banner_without_password_does_not_send_speculative_input() {
    let mut f = fixture();
    f.initial = format!(
        "Synetica - enLink :: Wireless Sensor Networks\n{}",
        f.initial.replace("Password:", "")
    );
    f.exchanges.clear();
    let script = Script::new(f, 32);
    let writes = script.writes.clone();
    assert!(
        BridgeSession::new(Box::new(script), timing())
            .run(&identity(), false, &AtomicBool::new(false), |_| {})
            .is_err()
    );
    assert!(writes.lock().unwrap().is_empty());
}

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
fn fresh_main_menu_logs_off_before_identity_and_configuration_access() {
    let mut f = fixture();
    f.exchanges.insert(0, ("X\r".into(), f.initial.clone()));
    f.initial = "enLink Main Menu:\r\nX - Exit and log off\r\nEnter Selection:".into();
    let script = Script::new(f, 16);
    let mut session = BridgeSession::new(Box::new(script), timing());
    session
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
}

#[test]
fn read_all_selects_detailed_mode_when_the_device_requests_options() {
    let mut f = fixture();
    let index = f.exchanges.iter().position(|(c, _)| c == "A\r").unwrap();
    let result = f.exchanges[index].1.clone();
    f.exchanges[index].1 =
        "Read All Data Points Options:\r\nN - Normal\r\nD - Detailed\r\nEnter Selection [Normal]:"
            .into();
    f.exchanges.insert(index + 1, ("D\r".into(), result));
    BridgeSession::new(Box::new(Script::new(f, 17)), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
}

struct Script {
    pending: VecDeque<u8>,
    exchanges: VecDeque<(String, String)>,
    chunk: usize,
    writes: Arc<Mutex<Vec<String>>>,
    drops: Arc<AtomicUsize>,
    disconnect: bool,
    idle_reads: usize,
    response_delay: usize,
}
impl Script {
    fn new(f: Fixture, chunk: usize) -> Self {
        Self {
            pending: f.initial.bytes().collect(),
            exchanges: f.exchanges.into(),
            chunk,
            writes: Arc::default(),
            drops: Arc::default(),
            disconnect: false,
            idle_reads: 0,
            response_delay: 0,
        }
    }
}
impl Transport for Script {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self.disconnect {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.idle_reads > 0 {
            self.idle_reads -= 1;
            std::thread::sleep(Duration::from_millis(1));
            return Err(io::ErrorKind::TimedOut.into());
        }
        let count = self.chunk.min(bytes.len()).min(self.pending.len());
        if count == 0 {
            std::thread::sleep(Duration::from_millis(1));
            return Err(io::ErrorKind::TimedOut.into());
        }
        for byte in &mut bytes[..count] {
            *byte = self.pending.pop_front().unwrap();
        }
        Ok(count)
    }
    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        assert!(
            self.pending.is_empty(),
            "command sent before response drained"
        );
        let actual = String::from_utf8(bytes.to_vec()).unwrap();
        self.writes.lock().unwrap().push(actual.clone());
        let (expected, response) = self
            .exchanges
            .pop_front()
            .expect("unexpected console command");
        assert_eq!(actual, expected);
        self.pending.extend(response.bytes());
        self.idle_reads = self.response_delay;
        Ok(())
    }
}
impl Drop for Script {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn full_read_all_handles_arbitrary_chunks_and_sparse_point_indices() {
    for chunk in [1, 2, 17, 1024] {
        let script = Script::new(fixture(), chunk);
        let writes = script.writes.clone();
        let mut session = BridgeSession::new(Box::new(script), timing());
        let result = session
            .run(&identity(), true, &AtomicBool::new(false), |_| {})
            .unwrap();
        assert_eq!(result.successful_reads, Some(2));
        assert_eq!(
            result
                .readings
                .iter()
                .map(|r| (r.item, r.value))
                .collect::<Vec<_>>(),
            vec![(1, 22.5), (3, 1.0)]
        );
        assert!(
            result
                .native_tsv
                .starts_with("Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\r\n")
        );
        assert!(!result.native_tsv.contains("cafe"));
        assert_eq!(writes.lock().unwrap().len(), 8);
    }
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
fn missing_new_response_cannot_reuse_an_old_menu() {
    let mut f = fixture();
    f.exchanges[1].1.clear();
    let script = Script::new(f, 11);
    let writes = script.writes.clone();
    let error = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), false, &AtomicBool::new(false), |_| {})
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::Timeout));
    assert_eq!(*writes.lock().unwrap(), vec!["cafe\r", "C\r"]);
}

#[test]
fn invalid_readings_summary_and_export_are_rejected() {
    for (index, old, new) in [
        (3, "1\t1\tHold", "1\t0\tHold"),
        (3, "3\t1\tHold", "1\t1\tHold"),
        (6, "22.5", "NaN"),
        (6, "3\t1", "2\t1"),
        (7, "2/0", "1/1"),
    ] {
        let mut f = fixture();
        f.exchanges[index].1 = f.exchanges[index].1.replace(old, new);
        let error = BridgeSession::new(Box::new(Script::new(f, 5)), timing())
            .run(&identity(), true, &AtomicBool::new(false), |_| {})
            .unwrap_err();
        assert!(
            matches!(error.code, ErrorCode::InvalidResponse),
            "{}",
            error.message
        );
    }
    assert!(parse_export("1\t1\tHold\t4\tF32\tHL\t1\tInt", 2).is_err());
    assert!(
        parse_export("Press a key to continue", 0)
            .unwrap()
            .is_empty()
    );
}

struct FakeBackend {
    scripts: Mutex<VecDeque<Script>>,
    opens: AtomicUsize,
    present: AtomicBool,
}
impl FakeBackend {
    fn new(scripts: Vec<Script>) -> Arc<Self> {
        Arc::new(Self {
            scripts: Mutex::new(scripts.into()),
            opens: AtomicUsize::new(0),
            present: AtomicBool::new(true),
        })
    }
}
impl Backend for FakeBackend {
    fn inventory(&self) -> Result<Vec<PortInfo>, String> {
        Ok(if self.present.load(Ordering::SeqCst) {
            vec![PortInfo {
                port: "COM5".into(),
                usb_vid: Some(0x0483),
                usb_pid: Some(0x5740),
                serial_number: None,
                description: "Test console".into(),
                identity: None,
                busy: false,
            }]
        } else {
            vec![]
        })
    }
    fn open(&self, _: &str) -> io::Result<Box<dyn Transport>> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(
            self.scripts
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected reopen"),
        ))
    }
}
fn command(id: u64, operation: Operation) -> Command {
    let hardware = matches!(
        operation,
        Operation::BridgeExport | Operation::BridgeReadAll | Operation::ClosePort
    );
    Command {
        request_id: id,
        port: hardware.then(|| "com5".into()),
        expected_identity: hardware.then(identity),
        operation,
    }
}
fn terminal(service: &Service, id: u64) -> EventKind {
    loop {
        let event = service.events.recv_timeout(Duration::from_secs(3)).unwrap();
        if event.request_id == id
            && matches!(
                event.kind,
                EventKind::BridgeResult { .. }
                    | EventKind::Error { .. }
                    | EventKind::Result { .. }
                    | EventKind::PortSnapshot { .. }
            )
        {
            return event.kind;
        }
    }
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
fn delayed_output_is_awaited_without_retrying_commands() {
    let mut script = Script::new(fixture(), 1);
    script.idle_reads = 4;
    script.response_delay = 4;
    let writes = script.writes.clone();
    let result = BridgeSession::new(Box::new(script), timing())
        .run(&identity(), true, &AtomicBool::new(false), |_| {})
        .unwrap();
    assert_eq!(result.successful_reads, Some(2));
    assert_eq!(writes.lock().unwrap().len(), 8);
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

// Synthetic import exchanges follow tools/enlink_bridge_config.ps1. These are
// protocol tests, not evidence that programming has passed on physical hardware.
fn programming_fixture() -> (Fixture, String, String) {
    let mut f = fixture();
    let current = "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\r\n1\t1\tHold\t4\tF32\tHL\t1\tInt\r\n3\t1\tHold\t512\tU16\tHH\t1\tInt\r\n".to_string();
    let target = current.replace("3\t1\tHold\t512", "2\t1\tHold\t514");
    f.exchanges.truncate(6);
    let menu = "Modbus Import/Export Menu:\r\nE - Export 2/32\r\nEnter Selection:";
    f.exchanges.push(("M\r".into(), menu.into()));
    for (table, ack, delete) in [
        (&current, "deleted OK", true),
        (&target, "imported OK", false),
    ] {
        f.exchanges
            .push(("I\r".into(), "Finish the import with an empty line".into()));
        for row in table.lines().skip(1) {
            let mut fields: Vec<_> = row.split('\t').collect();
            if delete {
                fields[1] = "0";
            }
            f.exchanges.push((
                format!("{}\r", fields.join("\t")),
                format!("Item {} {ack}\r\n", fields[0]),
            ));
        }
        f.exchanges.push((
            "\r".into(),
            "Import Finished. Results:\r\nPress a key to continue:".into(),
        ));
        f.exchanges.push(("\r".into(), menu.into()));
    }
    f.exchanges
        .push(("E\r".into(), format!("{target}Press a key to continue")));
    f.exchanges.push(("\r".into(), menu.into()));
    f.exchanges
        .push(("X\r".into(), "Modbus Configuration Menu:\r\n".into()));
    (f, current, target)
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
    let captured = include_str!("fixtures/bridge-import-finished.txt");
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
