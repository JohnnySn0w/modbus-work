//! Fresh acquisition history. Retained display values must never enter this store.
use crate::{adapter::AdapterResult, bridge::BridgeResult, contract::PortInfo};
use serde::Serialize;
use std::collections::VecDeque;

pub const LIMIT: usize = 50_000;
#[derive(Clone, Serialize)]
pub struct Sample {
    pub timestamp: String,
    pub unix_ms: u64,
    pub source: String,
    pub definition: String,
    pub value: Option<f64>,
    pub status: String,
}
#[derive(Default)]
pub struct History {
    pub samples: VecDeque<Sample>,
    pub discarded: usize,
}
fn source(port: &PortInfo) -> String {
    match (&port.serial_number, port.usb_vid, port.usb_pid) {
        (Some(serial), Some(vid), Some(pid)) if !serial.trim().is_empty() => {
            format!("{vid:04x}:{pid:04x}/{serial}")
        }
        _ => format!("Unverified USB identity at {}", port.port),
    }
}
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
impl History {
    fn push(
        &mut self,
        source: &str,
        definition: String,
        value: Option<f64>,
        status: String,
        at: &str,
        ms: u64,
    ) {
        if self.samples.len() == LIMIT {
            self.samples.pop_front();
            self.discarded += 1;
        }
        self.samples.push_back(Sample {
            timestamp: at.into(),
            unix_ms: ms,
            source: source.into(),
            definition,
            value: value.filter(|v| v.is_finite()),
            status,
        });
    }
    pub fn bridge(&mut self, port: &PortInfo, result: &BridgeResult, at: &str, ms: u64) {
        if result.successful_reads.is_none() {
            return;
        }
        let source = source(port);
        let profiles = crate::catalog::bundled().unwrap_or_default();
        let devices = crate::network::recognize(&result.native_tsv, &profiles).unwrap_or_default();
        for row in result.native_tsv.lines().skip(1) {
            let Some(item) = row.split('\t').next().and_then(|s| s.parse::<u8>().ok()) else {
                continue;
            };
            let error = result.exceptions.iter().find(|e| e.item == item);
            let value = result
                .readings
                .iter()
                .find(|r| r.item == item)
                .map(|r| r.value)
                .filter(|v| v.is_finite() && error.is_none());
            let status = error.map_or_else(
                || {
                    if value.is_some() {
                        "Good".into()
                    } else {
                        "Missing reading".into()
                    }
                },
                |e| format!("Exception {}: {}", e.code, e.message),
            );
            let register = row
                .split('\t')
                .nth(1)
                .and_then(|id| id.parse::<u8>().ok())
                .and_then(|slave| devices.iter().find(|device| device.slave == slave))
                .and_then(|device| {
                    let profile = &profiles[device.profile];
                    profile
                        .rows
                        .iter()
                        .find(|(_, candidate)| {
                            candidate.split('\t').skip(2).eq(row.split('\t').skip(2))
                        })
                        .and_then(|(point, _)| profile.point_register(*point))
                });
            let name = register.map_or_else(
                || format!("Modbus Bridge point {item}"),
                |r| format!("{} ({})", r.name, r.units),
            );
            self.push(
                &source,
                format!("{name} · {}", row.replace('\t', " / ")),
                value,
                status,
                at,
                ms,
            );
        }
    }
    pub fn adapter(&mut self, result: &AdapterResult, at: &str, ms: u64) {
        let source = source(&result.port);
        let keys: std::collections::BTreeSet<_> = result
            .values
            .keys()
            .chain(result.errors.keys())
            .copied()
            .collect();
        for address in keys {
            let error = result.errors.get(&address);
            let value = result
                .values
                .get(&address)
                .copied()
                .filter(|v| v.is_finite() && error.is_none());
            self.push(
                &source,
                format!(
                    "{} · {} · PDU {address}",
                    result.key,
                    result.settings.label()
                ),
                value,
                error.cloned().unwrap_or_else(|| {
                    if value.is_some() {
                        "Good".into()
                    } else {
                        "Missing reading".into()
                    }
                }),
                at,
                ms,
            );
        }
    }
    pub fn gap(&mut self, port: &PortInfo, status: &str, at: &str, ms: u64) {
        let source = source(port);
        let definitions: std::collections::BTreeSet<_> = self
            .samples
            .iter()
            .filter(|s| s.source == source)
            .map(|s| s.definition.clone())
            .collect();
        for definition in definitions {
            self.push(&source, definition, None, status.into(), at, ms);
        }
    }
    pub fn export(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer.write_record([
            "timestamp_local",
            "unix_ms",
            "usb_identity",
            "point_definition",
            "value",
            "status",
        ])?;
        for s in &self.samples {
            writer.write_record([
                s.timestamp.clone(),
                s.unix_ms.to_string(),
                s.source.clone(),
                s.definition.clone(),
                s.value.map(|v| v.to_string()).unwrap_or_default(),
                s.status.clone(),
            ])?;
        }
        let bytes = writer.into_inner().map_err(std::io::Error::other)?;
        // Export to a sibling temporary file, flush before replacement.
        let temp = path.with_extension(format!("csv.{}.tmp", now_ms()));
        let mut owned = false;
        let result = (|| {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            owned = true;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, path)
        })();
        if owned && result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_is_a_gap_and_real_zero_is_preserved() {
        let p = PortInfo {
            port: "COM41".into(),
            usb_vid: Some(1),
            usb_pid: Some(2),
            serial_number: Some("unit".into()),
            description: String::new(),
            identity: None,
            busy: false,
        };
        let mut h = History::default();
        let mut r = BridgeResult {
            identity: crate::contract::Identity {
                model: "x".into(),
                firmware: "x".into(),
            },
            native_tsv: "header\n1\t1\tHold\t4\tF32\tHL\t1\tInt\n".into(),
            readings: vec![crate::bridge::Reading {
                item: 1,
                value: 0.0,
            }],
            successful_reads: Some(1),
            exceptions: vec![],
        };
        h.bridge(&p, &r, "time", 1);
        r.readings.clear();
        r.successful_reads = Some(0);
        h.bridge(&p, &r, "time", 2);
        assert_eq!(h.samples[0].value, Some(0.0));
        assert_eq!(h.samples[1].value, None);
        r.successful_reads = None;
        h.bridge(&p, &r, "time", 3);
        assert_eq!(h.samples.len(), 2);
        let mut moved = p.clone();
        moved.port = "COM99".into();
        h.gap(&moved, "Disconnected", "time", 4);
        assert_eq!(h.samples[2].source, h.samples[0].source);
        r.native_tsv = r.native_tsv.replace("\t4\t", "\t8\t");
        r.successful_reads = Some(0);
        h.bridge(&p, &r, "time", 5);
        assert_ne!(h.samples[3].definition, h.samples[0].definition);
        // Reordered, repeated models retain their own register names and units.
        let profiles = crate::catalog::bundled().unwrap();
        let index = profiles
            .iter()
            .position(|p| {
                p.point_register(1)
                    .is_some_and(|r| r.name == "Temperature" && r.units == "deg C")
            })
            .unwrap();
        r.native_tsv = crate::network::compose(
            &[
                crate::network::Device::new(index, 3, &profiles),
                crate::network::Device::new(index, 1, &profiles),
            ],
            &profiles,
        )
        .unwrap();
        let start = h.samples.len();
        h.bridge(&p, &r, "time", 6);
        let points = profiles[index].rows.len();
        assert!(h.samples[start].definition.contains("Temperature (deg C)"));
        assert!(h.samples[start].definition.contains(" / 3 / "));
        assert!(
            h.samples[start + points]
                .definition
                .contains("Temperature (deg C)")
        );
        assert!(h.samples[start + points].definition.contains(" / 1 / "));
    }
}
