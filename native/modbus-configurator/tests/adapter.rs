use modbus_configurator::{
    adapter::*,
    bridge::Timing,
    contract::*,
    service::{Backend, Service},
    transport::Transport,
};
use std::{
    collections::BTreeMap,
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
fn port(bridge: bool) -> PortInfo {
    PortInfo {
        port: if bridge { "COM5" } else { "COM3" }.into(),
        usb_vid: Some(if bridge { 0x0483 } else { 0x0403 }),
        usb_pid: Some(if bridge { 0x5740 } else { 0x6001 }),
        serial_number: Some("fixture".into()),
        description: "test".into(),
        identity: None,
        busy: false,
    }
}
struct Fixture {
    values: BTreeMap<(u8, u16, u16), Vec<u8>>,
}
impl Bus for Fixture {
    fn request(&mut self, slave: u8, function: u8, addr: u16, count: u16) -> io::Result<Vec<u8>> {
        assert!([1, 127, 240].contains(&slave));
        assert!([3, 0x11].contains(&function));
        self.values
            .get(&(function, addr, count))
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "fixture timeout"))
    }
}
fn f32low(v: f32) -> Vec<u8> {
    let b = v.to_be_bytes();
    vec![b[2], b[3], b[0], b[1]]
}
fn dpt() -> Box<dyn Bus> {
    Box::new(Fixture {
        values: BTreeMap::from([
            ((3, 44, 2), f32low(1.01)),
            ((3, 20, 2), f32low(11000.0)),
            ((3, 512, 2), vec![0, 1, 0, 1]),
            ((3, 4, 2), f32low(23.5)),
            ((3, 6, 2), f32low(8.5)),
            ((3, 10, 2), f32low(8.6)),
            ((3, 512, 1), vec![0, 1]),
            ((3, 513, 1), vec![0, 1]),
            ((3, 515, 2), vec![0, 2, 0, 1]),
        ]),
    })
}
#[test]
fn dpt_reads_all_eight_real_registers_and_stops_at_first_match() {
    let mut opens = 0;
    let r = poll(
        port(false),
        |s| {
            opens += 1;
            assert_eq!(s.slave, 1);
            assert!(s.two_stops);
            Ok(dpt())
        },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(opens, 1);
    assert_eq!(r.key, "dpt146");
    assert_eq!(r.values.len(), 8);
    assert!(r.errors.is_empty());
    assert_eq!(r.values[&4], 23.5);
    assert_eq!(r.values[&515], 65538.0);
}
#[test]
fn absent_devices_use_only_python_candidates() {
    let mut seen = vec![];
    assert!(
        poll(
            port(false),
            |s| {
                seen.push((s.slave, s.even, s.two_stops));
                Ok(Box::new(Fixture {
                    values: BTreeMap::new(),
                }))
            },
            || Ok(())
        )
        .is_err()
    );
    assert_eq!(
        seen,
        [
            (1, false, true),
            (1, false, false),
            (1, false, false),
            (127, false, false),
            (240, true, false)
        ]
    );
}
#[test]
fn bridge_or_changed_usb_prevents_any_adapter_open() {
    for ports in [vec![port(false), port(true)], vec![]] {
        assert!(
            poll(
                port(false),
                |_| panic!("must not open"),
                || guard(&ports, &port(false))
            )
            .is_err()
        );
    }
    let mut replacement = port(false);
    replacement.serial_number = Some("replacement".into());
    assert!(guard(&[replacement], &port(false)).is_err());
}
#[test]
fn bridge_appearing_mid_batch_stops_before_next_request() {
    let mut checks = 0;
    let mut opens = 0;
    assert!(
        poll(
            port(false),
            |_| {
                opens += 1;
                Ok(dpt())
            },
            || {
                checks += 1;
                if checks == 3 {
                    Err(io::Error::other("Modbus Bridge attached"))
                } else {
                    Ok(())
                }
            }
        )
        .is_err()
    );
    assert_eq!(opens, 1);
}
#[test]
fn ambiguous_hmd_byte_order_is_not_a_device_identity() {
    let mut opens = 0;
    assert!(
        poll(
            port(false),
            |_| {
                opens += 1;
                Ok(Box::new(Fixture {
                    values: BTreeMap::from([((3, 0, 16), vec![0; 32]), ((3, 512, 7), vec![0; 14])]),
                }))
            },
            || Ok(())
        )
        .is_err()
    );
    assert_eq!(opens, 5);
}
struct GuardBackend {
    opens: Arc<AtomicUsize>,
}
impl Backend for GuardBackend {
    fn inventory(&self) -> Result<Vec<PortInfo>, String> {
        Ok(vec![port(false), port(true)])
    }
    fn open(&self, _: &str) -> io::Result<Box<dyn Transport>> {
        panic!("Modbus Bridge transport must not be used")
    }
    fn open_adapter(&self, _: &str, _: Settings) -> io::Result<Box<dyn Bus>> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        Ok(dpt())
    }
}
#[test]
fn service_enforces_bus_guard_even_for_explicit_adapter_requests() {
    let opens = Arc::new(AtomicUsize::new(0));
    let service = Service::with_backend(
        Arc::new(GuardBackend {
            opens: opens.clone(),
        }),
        Timing::default(),
    )
    .unwrap();
    service
        .send(Command {
            request_id: 1,
            port: Some("COM3".into()),
            expected_identity: None,
            operation: Operation::AdapterRead,
        })
        .unwrap();
    let event = service.events.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        event.kind,
        EventKind::Error {
            code: ErrorCode::UnsafeState,
            ..
        }
    ));
    assert_eq!(opens.load(Ordering::SeqCst), 0);
}

#[test]
fn route_selection_follows_unique_identity_not_com_sort_order() {
    use modbus_configurator::{
        adapter::{choose_route, is_adapter, same_device, same_route},
        contract::PortInfo,
    };
    let old = PortInfo {
        port: "COM5".into(),
        usb_vid: Some(0x0403),
        usb_pid: Some(0x6001),
        serial_number: Some("unit-a".into()),
        description: "FTDI".into(),
        identity: None,
        busy: false,
    };
    let mut moved = old.clone();
    moved.port = "COM17".into();
    moved.description = "Updated driver label".into();
    assert!(same_device(&old, &moved));
    assert!(!same_route(&old, &moved));
    let mut other = moved.clone();
    other.serial_number = Some("unit-b".into());
    other.port = "COM2".into();
    let ports = vec![other, moved.clone()];
    assert!(choose_route(&ports, None, is_adapter).is_none());
    assert_eq!(
        choose_route(&ports, Some(&old), is_adapter).unwrap().port,
        "COM17"
    );
    assert!(choose_route(&[moved.clone(), moved], Some(&old), is_adapter).is_none());
    let mut anonymous = old.clone();
    anonymous.serial_number = None;
    assert!(!same_device(&anonymous, &anonymous));
}

#[test]
fn service_adapter_actor_returns_live_values_and_transport_errors() {
    struct BackendFixture(bool);
    impl Backend for BackendFixture {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            Ok(vec![port(false)])
        }
        fn open(&self, _: &str) -> io::Result<Box<dyn Transport>> {
            panic!("Wrong transport")
        }
        fn open_adapter(&self, _: &str, _: Settings) -> io::Result<Box<dyn Bus>> {
            if self.0 {
                Ok(dpt())
            } else {
                Err(io::ErrorKind::PermissionDenied.into())
            }
        }
    }
    for succeeds in [true, false] {
        let service =
            Service::with_backend(Arc::new(BackendFixture(succeeds)), Timing::default()).unwrap();
        service
            .send(Command {
                request_id: 81,
                port: Some(port(false).port),
                expected_identity: None,
                operation: Operation::AdapterRead,
            })
            .unwrap();
        let event = service.events.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(event.request_id, 81);
        match event.kind {
            EventKind::AdapterResult { result } if succeeds => {
                assert_eq!(result.values[&4], 23.5);
                assert_eq!(result.values.len(), 8);
            }
            EventKind::Error {
                code: ErrorCode::Transport,
                recoverable: true,
                ..
            } if !succeeds => {}
            other => panic!("Unexpected {other:?}"),
        }
    }
}

#[test]
fn hmd_and_wattnode_identification_decode_real_payloads_and_keep_register_errors() {
    for key in ["hmd65", "wattnode"] {
        let mut opens = 0;
        let result = poll(
            port(false),
            |_| {
                opens += 1;
                let mut values = BTreeMap::new();
                if key == "hmd65" && opens == 2 {
                    let mut raw = vec![];
                    for value in [
                        f32::from_bits(0x4248ffff),
                        23.5,
                        8.5,
                        8.5,
                        1.0,
                        1.0,
                        8.5,
                        1.0,
                    ] {
                        raw.extend(value.to_be_bytes());
                    }
                    values.insert((3, 0, 16), raw);
                    values.insert((3, 512, 7), vec![0; 14]);
                    values.insert((3, 0, 2), 50.0_f32.to_be_bytes().to_vec());
                    values.insert((3, 2, 2), f32::NAN.to_be_bytes().to_vec());
                    values.insert((3, 4, 2), vec![0]);
                } else if key == "wattnode" && opens == 3 {
                    values.insert((0x11, 0, 0), b"WattNode".to_vec());
                    let mut id = vec![0; 16];
                    id[0] = 1;
                    id[12..14].copy_from_slice(&530_u16.to_be_bytes());
                    id[14..16].copy_from_slice(&1001_u16.to_be_bytes());
                    values.insert((3, 1700, 8), id);
                    values.insert((3, 0, 2), f32low(1.0));
                }
                Ok(Box::new(Fixture { values }))
            },
            || Ok(()),
        )
        .unwrap();
        assert_eq!(result.key, key);
        assert_eq!(result.family_only, key == "wattnode");
        assert!(!result.errors.is_empty());
        if key == "hmd65" {
            assert_eq!(result.values[&0], 50.0);
            assert_eq!(result.errors[&2], "Non-finite measurement");
            assert_eq!(result.errors[&4], "Invalid register length");
            assert_eq!(opens, 2);
        } else {
            assert_eq!(opens, 5);
        }
    }
}

#[test]
fn adapter_replaced_between_selection_and_worker_start_is_not_opened() {
    struct Replaced {
        scans: AtomicUsize,
        opens: AtomicUsize,
    }
    impl Backend for Replaced {
        fn inventory(&self) -> Result<Vec<PortInfo>, String> {
            let mut p = port(false);
            p.serial_number = Some(
                if self.scans.fetch_add(1, Ordering::SeqCst) < 1 {
                    "original"
                } else {
                    "replacement"
                }
                .into(),
            );
            Ok(vec![p])
        }
        fn open(&self, _: &str) -> io::Result<Box<dyn Transport>> {
            panic!("Unexpected Modbus Bridge open")
        }
        fn open_adapter(&self, _: &str, _: Settings) -> io::Result<Box<dyn Bus>> {
            self.opens.fetch_add(1, Ordering::SeqCst);
            Ok(dpt())
        }
    }
    let backend = Arc::new(Replaced {
        scans: AtomicUsize::new(0),
        opens: AtomicUsize::new(0),
    });
    let service = Service::with_backend(backend.clone(), Timing::default()).unwrap();
    service
        .send(Command {
            request_id: 91,
            port: Some(port(false).port),
            expected_identity: None,
            operation: Operation::AdapterRead,
        })
        .unwrap();
    let event = service.events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(event.kind, EventKind::Error { .. }), "{event:?}");
    assert_eq!(backend.opens.load(Ordering::SeqCst), 0);
}
