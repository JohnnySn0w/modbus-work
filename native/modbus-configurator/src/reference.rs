//! Python application's generated UI/reference contract (no Python at runtime).
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct Reference {
    pub schema_version: u32,
    pub devices: BTreeMap<String, Device>,
    pub registers: BTreeMap<String, Vec<Register>>,
    pub premade: BTreeMap<String, String>,
    pub regions: BTreeMap<String, Region>,
    pub default_region: String,
}
#[derive(Debug, Deserialize)]
pub struct Device {
    pub key: String,
    pub name: String,
    pub subtitle: String,
    pub kind: String,
    pub status: String,
    pub description: String,
    pub color: String,
    pub readouts: Vec<Readout>,
    pub artifact: Option<String>,
    pub help_setup: Vec<String>,
    pub help_troubleshooting: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct Readout {
    pub label: String,
    pub unit: String,
}
#[derive(Debug, Deserialize)]
pub struct Register {
    pub name: String,
    pub logical: String,
    pub pdu: String,
    pub data_type: String,
    pub access: String,
    pub unit: String,
    pub description: String,
    pub readout_label: Option<String>,
    pub enum_values: Vec<(i64, String)>,
    pub bit_values: Vec<(i64, String)>,
    pub zero_meaning: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Region {
    pub key: String,
    pub display_name: String,
    pub frequency_mhz: String,
    pub enabled: bool,
    pub notes: String,
}

fn hex(value: i64) -> String {
    if value < 0 {
        format!("-{:X}", value.unsigned_abs())
    } else {
        format!("{value:X}")
    }
}
impl Register {
    pub fn first_pdu(&self) -> Option<u16> {
        self.pdu.split(['–', '-']).next()?.trim().parse().ok()
    }
    pub fn decode(&self, value: Option<i64>) -> String {
        let Some(value) = value else {
            let mut parts = vec![];
            if let Some(zero) = &self.zero_meaning {
                parts.push(format!("0: {zero}"));
            }
            parts.extend(
                self.enum_values
                    .iter()
                    .map(|(value, meaning)| format!("{value}: {meaning}")),
            );
            parts.extend(
                self.bit_values
                    .iter()
                    .map(|(mask, meaning)| format!("0x{}: {meaning}", hex(*mask))),
            );
            return if parts.is_empty() {
                "—".into()
            } else {
                parts.join("; ")
            };
        };
        if value == 0
            && let Some(zero) = &self.zero_meaning
        {
            return zero.clone();
        }
        if let Some((_, meaning)) = self.enum_values.iter().find(|(v, _)| *v == value) {
            return meaning.clone();
        }
        let mut active: Vec<_> = self
            .bit_values
            .iter()
            .filter(|(mask, _)| value & mask != 0)
            .map(|(_, meaning)| meaning.clone())
            .collect();
        if !active.is_empty() {
            let known = self.bit_values.iter().fold(0, |all, (mask, _)| all | mask);
            let unknown = value & !known;
            if unknown != 0 {
                active.push(format!("unknown bits 0x{}", hex(unknown)));
            }
            return active.join("; ");
        }
        if !self.enum_values.is_empty()
            || !self.bit_values.is_empty()
            || self.zero_meaning.is_some()
        {
            format!("Undocumented value {value} (0x{})", hex(value))
        } else {
            "—".into()
        }
    }
}
impl Reference {
    pub fn bundled() -> Result<Self, String> {
        let reference: Self = serde_json::from_str(include_str!("../assets/python-reference.json"))
            .map_err(|e| e.to_string())?;
        if reference.schema_version != 1
            || !reference.regions.contains_key(&reference.default_region)
        {
            return Err("Unsupported Python reference contract".into());
        }
        Ok(reference)
    }
}

pub fn catalog_id(key: &str) -> &str {
    if key == "wattnode" { "wnd-m1-mb" } else { key }
}

pub fn profile_matches(key: &str, id: &str) -> bool {
    id == catalog_id(key) || (key == "hmd65" && id == "hmd65-nonmetric")
}

/// Manufacturer PDFs bundled for offline reference.
pub fn manuals(key: &str) -> &'static [(&'static str, &'static str)] {
    match key {
        "bridge" => &[(
            "Hardware guide (PDF)",
            "docs/reference/e5-hardware-guide.pdf",
        )],
        "dpt146" => &[(
            "User guide (PDF)",
            "docs/reference/vaisala-dpt146-user-guide-M211372EN-E.pdf",
        )],
        "hmd65" => &[("User guide (PDF)", "docs/reference/hmd65-user-guide.pdf")],
        "wattnode" => &[
            (
                "Installation manual (PDF)",
                "docs/reference/wattnode-installation-manual.pdf",
            ),
            (
                "Reference manual (PDF)",
                "docs/reference/wattnode-reference-manual.pdf",
            ),
        ],
        "ati-f12" => &[(
            "F12/D operation manual — Rev K (PDF)",
            "docs/reference/ati-f12-operation-manual.pdf",
        )],
        "adapter" => &[(
            "Hardware and mode configuration (PDF)",
            "docs/reference/usb-comi-tb-manual.pdf",
        )],
        _ => &[],
    }
}
