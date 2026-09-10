//! Bounded, read-only USB-COMi/FTDI discovery, matching app/probe.py.
use crate::contract::PortInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    time::{Duration, Instant},
};

pub fn is_bridge(p: &PortInfo) -> bool {
    (p.usb_vid == Some(0x0483) && p.usb_pid == Some(0x5740))
        || p.description.to_ascii_uppercase().contains("STM32")
}
pub fn is_adapter(p: &PortInfo) -> bool {
    let name = p.description.to_ascii_uppercase();
    (p.usb_vid == Some(0x0403) && p.usb_pid == Some(0x6001))
        || ["USB-COMI", "USB COMI", "FTDI"]
            .iter()
            .any(|s| name.contains(s))
}
/// Physical USB identity, independent of the current COM route and driver label.
/// Missing serial numbers cannot establish continuity across reconnects.
pub fn same_device(a: &PortInfo, b: &PortInfo) -> bool {
    a.usb_vid.is_some()
        && a.usb_pid.is_some()
        && a.usb_vid == b.usb_vid
        && a.usb_pid == b.usb_pid
        && a.serial_number
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        && a.serial_number == b.serial_number
}
/// Resolve a selected identity only when unique. Never choose by COM ordering.
pub fn choose_route<'a>(
    ports: &'a [PortInfo],
    preferred: Option<&PortInfo>,
    candidate: fn(&PortInfo) -> bool,
) -> Option<&'a PortInfo> {
    let matches: Vec<_> = ports
        .iter()
        .filter(|p| candidate(p))
        .filter(|p| {
            preferred.is_none_or(|old| {
                if old
                    .serial_number
                    .as_deref()
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    same_device(old, p)
                } else {
                    same_route(old, p)
                }
            })
        })
        .collect();
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None
    }
}
/// Transport ownership requires both identity and current route to match.
pub fn same_route(a: &PortInfo, b: &PortInfo) -> bool {
    a.port.eq_ignore_ascii_case(&b.port)
        && a.usb_vid == b.usb_vid
        && a.usb_pid == b.usb_pid
        && a.serial_number == b.serial_number
        && a.description == b.description
}
pub fn guard(ports: &[PortInfo], expected: &PortInfo) -> io::Result<()> {
    if ports.iter().any(is_bridge) {
        return Err(io::Error::other(
            "USB adapter polling paused while a Synetica console is connected.",
        ));
    }
    if !ports
        .iter()
        .any(|p| same_route(p, expected) && is_adapter(p))
    {
        return Err(io::Error::other("USB adapter disconnected or changed."));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Settings {
    pub slave: u8,
    pub even: bool,
    pub two_stops: bool,
}
impl Settings {
    pub fn label(self) -> String {
        format!(
            "19200 8{}{} · address {}",
            if self.even { "E" } else { "N" },
            if self.two_stops { 2 } else { 1 },
            self.slave
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResult {
    pub port: PortInfo,
    pub key: String,
    pub settings: Settings,
    pub family_only: bool,
    /// Live decoded register values indexed by zero-based PDU address.
    pub values: BTreeMap<u16, f64>,
    pub errors: BTreeMap<u16, String>,
}

pub trait Bus: Send {
    fn request(&mut self, slave: u8, function: u8, address: u16, count: u16)
    -> io::Result<Vec<u8>>;
}
pub fn crc(bytes: &[u8]) -> u16 {
    let mut crc = 0xffff;
    for b in bytes {
        crc ^= u16::from(*b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}
pub fn validate(frame: &[u8], slave: u8, function: u8, count: u16) -> io::Result<Vec<u8>> {
    if frame.len() < 5
        || frame[0] != slave
        || crc(&frame[..frame.len() - 2])
            != u16::from_le_bytes([frame[frame.len() - 2], frame[frame.len() - 1]])
    {
        return Err(io::Error::other("Invalid Modbus address or checksum."));
    }
    if frame[1] == function | 0x80 {
        return Err(io::Error::other(format!("Modbus exception {}", frame[2])));
    }
    if frame[1] != function
        || frame.len() != usize::from(frame[2]) + 5
        || (function == 3 && u16::from(frame[2]) != count * 2)
    {
        return Err(io::Error::other("Invalid Modbus function or byte count."));
    }
    Ok(frame[3..frame.len() - 2].to_vec())
}
trait RtuPort: Read + Write + Send {
    fn clear_input(&self) -> io::Result<()>;
}
impl RtuPort for Box<dyn serialport::SerialPort> {
    fn clear_input(&self) -> io::Result<()> {
        self.clear(serialport::ClearBuffer::Input)
            .map_err(Into::into)
    }
}
struct SerialBus<P> {
    port: P,
    last: Instant,
}
pub fn open(port: &str, settings: Settings) -> io::Result<Box<dyn Bus>> {
    let port = serialport::new(port, 19200)
        .data_bits(serialport::DataBits::Eight)
        .parity(if settings.even {
            serialport::Parity::Even
        } else {
            serialport::Parity::None
        })
        .stop_bits(if settings.two_stops {
            serialport::StopBits::Two
        } else {
            serialport::StopBits::One
        })
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(20))
        .open()?;
    Ok(Box::new(SerialBus {
        port,
        last: Instant::now(),
    }))
}
impl<P: RtuPort> Bus for SerialBus<P> {
    fn request(
        &mut self,
        slave: u8,
        function: u8,
        address: u16,
        count: u16,
    ) -> io::Result<Vec<u8>> {
        if !(1..=247).contains(&slave)
            || ![3, 0x11].contains(&function)
            || (function == 3 && !(1..=125).contains(&count))
        {
            return Err(io::Error::other("Read-only Modbus request out of range."));
        }
        // Keep this handle open for the batch. The bridge's CDC reopen workaround is unrelated to RTU.
        std::thread::sleep(Duration::from_millis(3).saturating_sub(self.last.elapsed()));
        self.port.clear_input()?;
        let mut query = vec![slave, function];
        if function == 3 {
            query.extend(address.to_be_bytes());
            query.extend(count.to_be_bytes());
        }
        query.extend(crc(&query).to_le_bytes());
        self.port.write_all(&query)?;
        let deadline = Instant::now() + Duration::from_millis(160);
        let mut frame = vec![];
        while Instant::now() < deadline {
            let mut chunk = [0u8; 256];
            match self.port.read(&mut chunk) {
                Ok(n) => frame.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e),
            }
            if frame.len() >= 3 {
                let size = if frame[1] & 0x80 != 0 {
                    5
                } else {
                    usize::from(frame[2]) + 5
                };
                if frame.len() >= size {
                    self.last = Instant::now();
                    return validate(&frame, slave, function, count);
                }
            }
        }
        self.last = Instant::now();
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "No complete Modbus response.",
        ))
    }
}
fn float(raw: &[u8], low: bool) -> f64 {
    let b = if low {
        [raw[2], raw[3], raw[0], raw[1]]
    } else {
        [raw[0], raw[1], raw[2], raw[3]]
    };
    f64::from(f32::from_be_bytes(b))
}
fn word(raw: &[u8]) -> u16 {
    u16::from_be_bytes([raw[0], raw[1]])
}

/// The check runs before every open and every request, including hotplug and cancellation.
pub fn poll(
    port: PortInfo,
    mut open_bus: impl FnMut(Settings) -> io::Result<Box<dyn Bus>>,
    mut check: impl FnMut() -> io::Result<()>,
) -> io::Result<AdapterResult> {
    let mut interrupted = false;
    let check = std::cell::RefCell::new(|| {
        if interrupted {
            return Err(io::Error::other(
                "Adapter read stopped after the connection changed or cancellation was requested.",
            ));
        }
        let result = check();
        interrupted = result.is_err();
        result
    });
    let candidates = [
        ("dpt146", 1, false, true),
        ("hmd65", 1, false, false),
        ("wattnode", 1, false, false),
        ("wattnode", 127, false, false),
        ("dpt146", 240, true, false),
    ];
    let mut family = None;
    for (key, slave, even, two_stops) in candidates {
        (check.borrow_mut())()?;
        let settings = Settings {
            slave,
            even,
            two_stops,
        };
        let mut bus = open_bus(settings)?;
        let mut read = |function, addr, count| {
            (check.borrow_mut())()?;
            bus.request(slave, function, addr, count)
        };
        let identified = (|| -> io::Result<Option<bool>> {
            match key {
                "dpt146" => {
                    let p = read(3, 44, 2)?;
                    let m = read(3, 20, 2)?;
                    let s = read(3, 512, 2)?;
                    if p.len() != 4 || m.len() != 4 || s.len() != 4 {
                        return Ok(None);
                    }
                    Ok(((0.0..=12.0).contains(&float(&p, true))
                        && (0.0..=1_000_000.0).contains(&float(&m, true))
                        && word(&s) <= 1)
                        .then_some(true))
                }
                "hmd65" => {
                    let raw = read(3, 0, 16)?;
                    let status = read(3, 512, 7)?;
                    if raw.len() != 32
                        || status.len() != 14
                        || [0, 10, 12].iter().any(|i| word(&status[*i..]) > 0x1ff)
                    {
                        return Ok(None);
                    }
                    let orders: Vec<_> = [false, true]
                        .into_iter()
                        .filter(|low| {
                            let v: Vec<_> = raw.chunks_exact(4).map(|b| float(b, *low)).collect();
                            v.iter().all(|v| v.is_finite())
                                && (0.0..=100.0).contains(&v[0])
                                && (-80.0..=120.0).contains(&v[1])
                                && [2, 3, 6].iter().all(|i| (-120.0..=180.0).contains(&v[*i]))
                                && v[4] >= 0.0
                                && v[5] >= 0.0
                        })
                        .collect();
                    Ok(if orders.len() == 1 {
                        Some(orders[0])
                    } else {
                        None
                    })
                }
                _ => {
                    let id = read(0x11, 0, 0)?;
                    let id = String::from_utf8_lossy(&id);
                    if !id.contains("WattNode") && !id.contains("Continental Control Systems") {
                        return Ok(None);
                    }
                    let d = read(3, 1700, 8)?;
                    Ok((d.len() == 16
                        && word(&d[12..]) == 530
                        && (1000..1100).contains(&word(&d[14..]))
                        && d[..4].iter().any(|b| *b != 0))
                    .then_some(true))
                }
            }
        })();
        (check.borrow_mut())()?;
        let Ok(Some(low)) = identified else {
            continue;
        };
        let mut result = AdapterResult {
            port: port.clone(),
            key: key.into(),
            settings,
            family_only: key == "wattnode",
            values: BTreeMap::new(),
            errors: BTreeMap::new(),
        };
        let profiles = crate::catalog::bundled().map_err(io::Error::other)?;
        let profile = profiles
            .iter()
            .find(|p| p.info.id == crate::reference::catalog_id(key))
            .ok_or_else(|| io::Error::other("Missing register profile"))?;
        for row in profile.rows.values() {
            let f: Vec<_> = row.split('\t').collect();
            let addr = f[3].parse::<u16>().map_err(io::Error::other)?;
            let count = if f[4].ends_with("16") { 1 } else { 2 };
            let response = read(3, addr, count);
            (check.borrow_mut())()?;
            match response {
                Ok(raw) if raw.len() == usize::from(count) * 2 => {
                    let low = if f[4] == "F32" { low } else { f[5] == "HL" };
                    let v = match f[4] {
                        "F32" => float(&raw, low),
                        "U16" => f64::from(word(&raw)),
                        "S16" => f64::from(word(&raw) as i16),
                        kind => {
                            let b = if low {
                                [raw[2], raw[3], raw[0], raw[1]]
                            } else {
                                [raw[0], raw[1], raw[2], raw[3]]
                            };
                            if kind == "S32" {
                                f64::from(i32::from_be_bytes(b))
                            } else {
                                f64::from(u32::from_be_bytes(b))
                            }
                        }
                    };
                    if v.is_finite() {
                        result.values.insert(addr, v);
                    } else {
                        result.errors.insert(addr, "Non-finite measurement".into());
                    }
                }
                Ok(_) => {
                    result.errors.insert(addr, "Invalid register length".into());
                }
                Err(e) => {
                    result.errors.insert(addr, e.to_string());
                }
            }
        }
        if !result.family_only {
            return Ok(result);
        }
        family = Some(result);
    }
    family.ok_or_else(|| io::Error::other("No supported instrument responded on the USB adapter. Check power, wiring and serial settings."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc_and_strict_reply_validation() {
        assert_eq!(crc(&[1, 3, 0, 0, 0, 10]), 0xcdc5);
        let mut reply = vec![1, 3, 2, 0, 42];
        reply.extend(crc(&reply).to_le_bytes());
        assert_eq!(validate(&reply, 1, 3, 1).unwrap(), [0, 42]);
        assert!(validate(&reply, 2, 3, 1).is_err());
        assert!(validate(&reply, 1, 3, 2).is_err());
        reply[4] ^= 1;
        assert!(validate(&reply, 1, 3, 1).is_err());
        let mut error = vec![1, 0x83, 2];
        error.extend(crc(&error).to_le_bytes());
        assert!(
            validate(&error, 1, 3, 1)
                .unwrap_err()
                .to_string()
                .contains("exception 2")
        );
    }
}

#[cfg(test)]
#[path = "tests/rtu_io.rs"]
mod rtu_io_tests;
