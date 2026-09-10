use super::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
struct Port {
    input: VecDeque<io::Result<Vec<u8>>>,
    writes: Arc<Mutex<Vec<u8>>>,
    fail: &'static str,
}
impl Read for Port {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        match self.input.pop_front() {
            Some(Ok(chunk)) => {
                b[..chunk.len()].copy_from_slice(&chunk);
                Ok(chunk.len())
            }
            Some(Err(e)) => Err(e),
            None => {
                std::thread::sleep(Duration::from_millis(2));
                Err(io::ErrorKind::TimedOut.into())
            }
        }
    }
}
impl Write for Port {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.fail == "write" {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.writes.lock().unwrap().extend(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl RtuPort for Port {
    fn clear_input(&self) -> io::Result<()> {
        if self.fail == "clear" {
            Err(io::ErrorKind::PermissionDenied.into())
        } else {
            Ok(())
        }
    }
}
#[test]
fn rtu_preserves_fragmented_packets_and_propagates_faults_without_retries() {
    for (function, payload) in [
        (3, vec![0x41, 0xac]),
        (0x11, b"DPT146".to_vec()),
        (0x83, vec![]),
    ] {
        let mut response = vec![
            1,
            function,
            if function == 0x83 {
                2
            } else {
                payload.len() as u8
            },
        ];
        response.extend(&payload);
        response.extend(crc(&response).to_le_bytes());
        let writes = Arc::new(Mutex::new(vec![]));
        let mut bus = SerialBus {
            port: Port {
                input: response.chunks(1).map(|b| Ok(b.to_vec())).collect(),
                writes: writes.clone(),
                fail: "",
            },
            last: Instant::now(),
        };
        let requested = if function == 0x11 { 0x11 } else { 3 };
        let result = bus.request(1, requested, 0x1234, 1);
        if function == 0x83 {
            assert!(result.unwrap_err().to_string().contains("exception 2"));
        } else {
            assert_eq!(result.unwrap(), payload);
        }
        let mut expected = vec![1, requested];
        if requested == 3 {
            expected.extend([0x12, 0x34, 0, 1]);
        }
        expected.extend(crc(&expected).to_le_bytes());
        assert_eq!(*writes.lock().unwrap(), expected);
    }
    for fail in ["clear", "write", "read", "timeout", "invalid"] {
        let writes = Arc::new(Mutex::new(vec![]));
        let mut bus = SerialBus {
            port: Port {
                input: if fail == "read" {
                    VecDeque::from([Err(io::ErrorKind::ConnectionReset.into())])
                } else {
                    VecDeque::new()
                },
                writes: writes.clone(),
                fail,
            },
            last: Instant::now(),
        };
        let err = bus
            .request(if fail == "invalid" { 0 } else { 1 }, 3, 0, 1)
            .unwrap_err();
        assert_eq!(
            err.kind(),
            match fail {
                "clear" => io::ErrorKind::PermissionDenied,
                "write" => io::ErrorKind::BrokenPipe,
                "read" => io::ErrorKind::ConnectionReset,
                "timeout" => io::ErrorKind::TimedOut,
                _ => io::ErrorKind::Other,
            }
        );
        assert_eq!(
            writes.lock().unwrap().len(),
            if ["read", "timeout"].contains(&fail) {
                8
            } else {
                0
            }
        );
    }
}
