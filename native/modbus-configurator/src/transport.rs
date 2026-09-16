use std::io::{self, Read, Write};
use std::time::Duration;

/// Implementations must honor the read/write timeout. Ok(0) means disconnected.
pub trait Transport: Send {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize>;
    fn write(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn continuous_receive(&self) -> bool {
        false
    }
    fn requires_menu_selection_prompt(&self) -> bool {
        false
    }
    fn resume_receive(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn reconnect(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
}

// One worker owns one physical port from open until release/disconnect. It keeps
// draining input even while the GUI is idle or writing a backup.
struct SerialTransport {
    writes: std::sync::mpsc::SyncSender<WriteRequest>,
    received: std::sync::mpsc::Receiver<Vec<u8>>,
    pending: std::collections::VecDeque<u8>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    fault: std::sync::Arc<std::sync::Mutex<Option<(io::ErrorKind, String)>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
trait Connection: Read + Write + Send {
    fn resume_receive(&mut self) -> io::Result<()>;
}
type WriteRequest = (Option<Vec<u8>>, std::sync::mpsc::Sender<io::Result<()>>);
impl SerialTransport {
    fn spawn(mut port: impl Connection + 'static) -> io::Result<Self> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex, mpsc};
        let (writes, commands) = mpsc::sync_channel::<WriteRequest>(4);
        let (output, received) = mpsc::sync_channel::<Vec<u8>>(1024);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let fault = Arc::new(Mutex::new(None));
        let failure = fault.clone();
        let worker = std::thread::Builder::new()
            .name("Modbus Bridge-serial-io".into())
            .spawn(move || {
                let fail = |e: io::Error| {
                    *failure.lock().unwrap() = Some((e.kind(), e.to_string()));
                };
                while !stopping.load(Ordering::Acquire) {
                    if let Ok((bytes, ack)) = commands.try_recv() {
                        let result = match bytes {
                            Some(bytes) => port.write_all(&bytes),
                            None => port.resume_receive(),
                        };
                        let failed = result.is_err();
                        if let Err(e) = &result {
                            fail(io::Error::new(e.kind(), e.to_string()));
                        }
                        let _ = ack.send(result);
                        if failed {
                            break;
                        }
                    }
                    let mut bytes = [0; 64];
                    match port.read(&mut bytes) {
                        Ok(0) => {
                            fail(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Modbus Bridge USB connection closed",
                            ));
                            break;
                        }
                        Ok(n) => {
                            if output.try_send(bytes[..n].to_vec()).is_err() {
                                fail(io::Error::other(
                                    "Modbus Bridge receive queue overflow; transaction rejected",
                                ));
                                break;
                            }
                        }
                        Err(e)
                            if matches!(
                                e.kind(),
                                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                            ) => {}
                        Err(e) => {
                            fail(e);
                            break;
                        }
                    }
                }
                // Port is dropped here, once, after the worker has stopped.
            })?;
        Ok(Self {
            writes,
            received,
            pending: Default::default(),
            stop,
            fault,
            worker: Some(worker),
        })
    }
    fn check_fault(&self) -> io::Result<()> {
        if let Some((kind, message)) = &*self.fault.lock().unwrap() {
            Err(io::Error::new(*kind, message.clone()))
        } else {
            Ok(())
        }
    }
}
impl Drop for SerialTransport {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Transport for SerialTransport {
    fn requires_menu_selection_prompt(&self) -> bool {
        true
    }
    fn continuous_receive(&self) -> bool {
        true
    }
    fn resume_receive(&mut self) -> io::Result<()> {
        self.check_fault()?;
        let (ack, done) = std::sync::mpsc::channel();
        self.writes
            .send((None, ack))
            .map_err(|_| io::Error::from(io::ErrorKind::NotConnected))?;
        done.recv_timeout(Duration::from_secs(3)).map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "USB receive recovery deadline expired",
            )
        })?
    }
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.check_fault()?;
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.pending.is_empty() {
            let packet = self
                .received
                .recv_timeout(Duration::from_millis(50))
                .map_err(|e| match e {
                    std::sync::mpsc::RecvTimeoutError::Timeout => {
                        io::Error::from(io::ErrorKind::TimedOut)
                    }
                    _ => io::Error::new(io::ErrorKind::NotConnected, "Bridge I/O worker stopped"),
                })?;
            self.pending.extend(packet);
        }
        let count = bytes.len().min(self.pending.len());
        for byte in &mut bytes[..count] {
            *byte = self.pending.pop_front().unwrap();
        }
        Ok(count)
    }
    fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.check_fault()?;
        let (ack, done) = std::sync::mpsc::channel();
        self.writes
            .send((Some(bytes.to_vec()), ack))
            .map_err(|_| io::Error::from(io::ErrorKind::NotConnected))?;
        done.recv_timeout(Duration::from_secs(3)).map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "Modbus Bridge command write deadline expired",
            )
        })?
    }
}
struct Port(Box<dyn serialport::SerialPort>);
impl Connection for Port {
    fn resume_receive(&mut self) -> io::Result<()> {
        // Same setting on the same USB console handle; downstream Modbus settings
        // are untouched. Bench recovery of a pending CDC reply, not a device reset.
        self.0.set_baud_rate(115200).map_err(io::Error::other)
    }
}
impl Read for Port {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        // This queries queue state through ClearCommError on Windows, without
        // purging input or closing the handle.
        self.0.bytes_to_read().map_err(io::Error::other)?;
        self.0.read(b)
    }
}
impl Write for Port {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.write(b)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Drop for Port {
    fn drop(&mut self) {
        let _ = self.0.write_data_terminal_ready(false);
        let _ = self.0.write_request_to_send(false);
    }
}
pub fn open(port: &str) -> io::Result<Box<dyn Transport>> {
    let mut serial = serialport::new(port, 115200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(io::Error::other)?;
    serial
        .write_request_to_send(false)
        .map_err(io::Error::other)?;
    serial
        .write_data_terminal_ready(true)
        .map_err(io::Error::other)?;
    Ok(Box::new(SerialTransport::spawn(Port(serial))?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    struct Fake {
        rx: mpsc::Receiver<Vec<u8>>,
        tx: mpsc::Sender<Vec<u8>>,
        drops: Arc<AtomicUsize>,
        writes: Arc<AtomicUsize>,
    }
    impl Connection for Fake {
        fn resume_receive(&mut self) -> io::Result<()> {
            self.tx.send(b"recovered".to_vec()).unwrap();
            Ok(())
        }
    }
    impl Read for Fake {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let v = self
                .rx
                .recv_timeout(Duration::from_millis(5))
                .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))?;
            b[..v.len()].copy_from_slice(&v);
            Ok(v.len())
        }
    }
    impl Write for Fake {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            self.tx.send(b"reply".to_vec()).unwrap();
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Drop for Fake {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn one_connection_drains_idle_input_and_closes_once_after_many_commands() {
        let (tx, rx) = mpsc::channel();
        let drops = Arc::new(AtomicUsize::new(0));
        let writes = Arc::new(AtomicUsize::new(0));
        let mut transport = SerialTransport::spawn(Fake {
            rx,
            tx: tx.clone(),
            drops: drops.clone(),
            writes: writes.clone(),
        })
        .unwrap();
        assert!(transport.continuous_receive());
        assert!(transport.requires_menu_selection_prompt());
        assert_eq!(transport.read(&mut []).unwrap(), 0);
        assert_eq!(
            transport.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        tx.send(b"idle banner".to_vec()).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let mut buffer = [0; 64];
        assert_eq!(transport.read(&mut buffer).unwrap(), 11);
        for _ in 0..50 {
            transport.write(b"A\r").unwrap();
            let n = transport.read(&mut buffer).unwrap();
            assert_eq!(&buffer[..n], b"reply");
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }
        assert_eq!(writes.load(Ordering::SeqCst), 50);
        transport.resume_receive().unwrap();
        let n = transport.read(&mut buffer).unwrap();
        assert_eq!(&buffer[..n], b"recovered");
        assert_eq!(writes.load(Ordering::SeqCst), 50);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(transport);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
#[path = "tests/transport_failures.rs"]
mod failure_tests;
