use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
struct Failing {
    mode: &'static str,
    writes: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}
impl Connection for Failing {
    fn resume_receive(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "recovery failed"))
    }
}
impl Read for Failing {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        match self.mode {
            "eof" => Ok(0),
            "read" => Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "device removed",
            )),
            "overflow" => {
                b[0] = 1;
                Ok(1)
            }
            _ => {
                std::thread::sleep(Duration::from_millis(1));
                Err(io::ErrorKind::TimedOut.into())
            }
        }
    }
}
impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "write failed"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for Failing {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
fn failing(mode: &'static str) -> (SerialTransport, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let writes = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    (
        SerialTransport::spawn(Failing {
            mode,
            writes: writes.clone(),
            drops: drops.clone(),
        })
        .unwrap(),
        writes,
        drops,
    )
}
#[test]
fn write_failure_is_latched_and_never_retried() {
    let (mut t, w, d) = failing("write");
    assert_eq!(
        t.write(b"I\r").unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(t.write(b"I\r").is_err());
    assert!(t.read(&mut [0; 1]).is_err());
    drop(t);
    assert_eq!(w.load(Ordering::SeqCst), 1);
    assert_eq!(d.load(Ordering::SeqCst), 1);
}
#[test]
fn recovery_failure_closes_worker_without_sending_commands() {
    let (mut t, w, d) = failing("recover");
    assert_eq!(
        t.resume_receive().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(t.write(b"A\r").is_err());
    drop(t);
    assert_eq!(w.load(Ordering::SeqCst), 0);
    assert_eq!(d.load(Ordering::SeqCst), 1);
}
#[test]
fn eof_read_error_and_overflow_latch_failure_and_drop_once() {
    for mode in ["eof", "read", "overflow"] {
        let (mut t, w, d) = failing(mode);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while t.check_fault().is_ok() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(t.read(&mut [0; 1]).is_err());
        assert!(t.write(b"A\r").is_err());
        drop(t);
        assert_eq!(w.load(Ordering::SeqCst), 0);
        assert_eq!(d.load(Ordering::SeqCst), 1);
    }
}
