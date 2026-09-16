//! Shared scripted transports and fixtures for Modbus Bridge workflow tests.
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

fn timing() -> Timing {
    Timing {
        response: Duration::from_millis(150),
        quiet: Duration::from_millis(2),
        operation: Duration::from_secs(2),
    }
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

#[path = "bridge_workflow/programming.rs"]
mod programming;

#[path = "bridge_workflow/recovery.rs"]
mod recovery;

#[path = "bridge_workflow/service.rs"]
mod service;

#[path = "bridge_workflow/reading.rs"]
mod reading;
