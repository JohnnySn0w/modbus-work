//! Bundled, read-only catalog built from the repository's reviewed CSV/TSV artifacts.
use crate::{
    bridge::{BridgeResult, parse_export},
    contract::Identity,
};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};

const HEADER: &str = "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Validation {
    Validated,
    ToTest,
}
impl Validation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Validated => "Bench-validated configuration",
            Self::ToTest => "Documentation candidate — awaiting hardware",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileInfo {
    pub id: String,
    pub model: String,
    pub manufacturer: String,
    pub status: Validation,
    pub summary: String,
    pub serial: String,
    pub help: Vec<String>,
    pub source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    #[serde(rename = "register number")]
    pub manual: String,
    pub name: String,
    pub units: String,
    pub interpretation: String,
    pub decode: String,
    pub defaults: String,
}
impl Register {
    pub fn range(&self) -> Result<(u16, u16), String> {
        let parts: Vec<_> = self.manual.split(" - ").collect();
        if parts.len() > 2 {
            return Err("Invalid register range".into());
        }
        let first = parts[0]
            .parse::<u32>()
            .map_err(|_| "Invalid manual register")?;
        let last = parts
            .last()
            .unwrap()
            .parse::<u32>()
            .map_err(|_| "Invalid manual register")?;
        if first == 0 || last < first || last > 65536 {
            return Err("Register range outside Modbus address space".into());
        }
        Ok(((first - 1) as u16, (last - 1) as u16))
    }
    pub fn pdu_display(&self) -> String {
        match self.range() {
            Ok((first, last)) if first == last => first.to_string(),
            Ok((first, last)) => format!("{first} - {last}"),
            Err(_) => "Invalid".into(),
        }
    }
}

#[derive(Debug)]
pub struct Profile {
    pub info: ProfileInfo,
    pub registers: Vec<Register>,
    pub native_tsv: String,
    pub rows: BTreeMap<u8, String>,
    // Maps a bridge point to a register row only after range/type validation.
    points: BTreeMap<u8, usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PointDiff {
    pub item: u8,
    pub current: Option<String>,
    pub proposed: Option<String>,
}

pub fn table_rows(tsv: &str) -> Result<BTreeMap<u8, String>, String> {
    if !tsv.is_ascii() {
        return Err("Native E5 bridge table must be ASCII without a BOM".into());
    }
    let mut lines = tsv.lines();
    if lines.next() != Some(HEADER) {
        return Err("Native E5 bridge table header does not match the eight-column format".into());
    }
    let count = lines.filter(|line| !line.trim().is_empty()).count();
    parse_export(tsv, count).map_err(|error| error.message)
}

impl Profile {
    pub fn load(info: ProfileInfo, csv: &str, tsv: &str) -> Result<Self, String> {
        if info.id.is_empty() || info.model.is_empty() || info.help.is_empty() {
            return Err("Incomplete catalog metadata".into());
        }
        let mut reader = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
        let headers = reader.headers().map_err(|e| e.to_string())?;
        if headers.iter().collect::<Vec<_>>()
            != [
                "register number",
                "name",
                "units",
                "interpretation",
                "decode",
                "defaults",
            ]
        {
            return Err("Register CSV headers do not match the reviewed format".into());
        }
        let registers: Vec<Register> = reader
            .deserialize()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let mut occupied = HashSet::new();
        for register in &registers {
            if register.name.is_empty() {
                return Err("Register name is missing".into());
            }
            let (first, last) = register.range()?;
            for address in first..=last {
                if !occupied.insert(address) {
                    return Err("Register definitions overlap".into());
                }
            }
        }
        let rows = table_rows(tsv)?;
        if rows.is_empty() {
            return Err("A profile requires at least one configured E5 bridge point".into());
        }
        let mut points = BTreeMap::new();
        for (item, row) in &rows {
            let fields: Vec<_> = row.split('\t').collect();
            let address: u16 = fields[3].parse().map_err(|_| "Invalid point address")?;
            let width = if matches!(fields[4], "U16" | "S16") {
                1u32
            } else {
                2
            };
            let register = registers
                .iter()
                .position(|r| {
                    r.range().is_ok_and(|(first, last)| {
                        first == address && u32::from(last) + 1 == u32::from(address) + width
                    })
                })
                .ok_or_else(|| {
                    format!("Point {item} does not match a complete register definition")
                })?;
            let interpretation = registers[register].interpretation.to_ascii_lowercase();
            let expected_type = if interpretation.contains("ieee-754 float32") {
                Some("F32")
            } else if interpretation.contains("unsigned 32-bit") {
                Some("U32")
            } else if interpretation.contains("signed 32-bit") {
                Some("S32")
            } else {
                None
            };
            if expected_type.is_some_and(|expected| fields[4] != expected)
                || (width == 2
                    && interpretation.contains("low 16-bit word first")
                    && fields[5] != "HL")
                || fields[6].parse::<f64>().map_err(|_| "Invalid multiplier")? != 1.0
            {
                return Err(format!(
                    "Point {item} encoding or multiplier conflicts with the register documentation"
                ));
            }
            if points.values().any(|index| *index == register) {
                return Err("Multiple points map to the same instrument register".into());
            }
            points.insert(*item, register);
        }
        let native_tsv = format!(
            "{HEADER}\r\n{}",
            rows.values()
                .map(|row| format!("{row}\r\n"))
                .collect::<String>()
        );
        Ok(Self {
            info,
            registers,
            native_tsv,
            rows,
            points,
        })
    }

    pub fn point_register(&self, item: u8) -> Option<&Register> {
        self.points.get(&item).map(|index| &self.registers[*index])
    }

    pub fn register_point(&self, index: usize) -> Option<(u8, String)> {
        self.points
            .iter()
            .find(|(_, register)| **register == index)
            .map(|(item, _)| {
                let fields: Vec<_> = self.rows[item].split('\t').collect();
                (*item, format!("{} {}", fields[4], fields[5]))
            })
    }

    pub fn matches(&self, result: &BridgeResult) -> bool {
        result.identity
            == (Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            })
            && table_rows(&result.native_tsv).is_ok_and(|rows| rows == self.rows)
    }

    /// Extra points do not change the meaning of an exact, complete profile
    /// subset. All eight fields of every profile row must still match.
    pub fn contains_points(&self, result: &BridgeResult) -> bool {
        result.identity
            == (Identity {
                model: "ENL-MOD-32".into(),
                firmware: "3.6".into(),
            })
            && table_rows(&result.native_tsv).is_ok_and(|rows| {
                self.rows
                    .iter()
                    .all(|(item, row)| rows.get(item) == Some(row))
            })
    }

    pub fn diff(&self, current: &str) -> Result<Vec<PointDiff>, String> {
        let current = table_rows(current)?;
        let mut ids: Vec<_> = current.keys().chain(self.rows.keys()).copied().collect();
        ids.sort_unstable();
        ids.dedup();
        Ok(ids
            .into_iter()
            .filter(|item| current.get(item) != self.rows.get(item))
            .map(|item| PointDiff {
                item,
                current: current.get(&item).cloned(),
                proposed: self.rows.get(&item).cloned(),
            })
            .collect())
    }
}

pub fn bundled() -> Result<Vec<Profile>, String> {
    let metadata: Vec<ProfileInfo> =
        serde_json::from_str(include_str!("../assets/catalog.json")).map_err(|e| e.to_string())?;
    let sources = [
        (
            "dpt146",
            include_str!(
                "../../../artifacts/register-tables/vaisala-dpt146-modbus-register-table.csv"
            ),
            include_str!("../../../artifacts/bridge-config/vaisala-dpt146-validated.tsv"),
        ),
        (
            "hmd65",
            include_str!(
                "../../../artifacts/register-tables/vaisala-hmd65-modbus-register-table.csv"
            ),
            include_str!("../../../artifacts/bridge-config/hmd65-documentation-test.tsv"),
        ),
        (
            "wnd-m1-mb",
            include_str!(
                "../../../artifacts/register-tables/wattnode-wnd-m1-mb-modbus-register-table.csv"
            ),
            include_str!(
                "../../../artifacts/bridge-config/wattnode-wnd-m1-mb-documentation-test.tsv"
            ),
        ),
        (
            "hmd65-nonmetric",
            include_str!("../assets/hmd65-nonmetric-registers.csv"),
            include_str!("../../../artifacts/bridge-config/hmd65-nonmetric-documentation-test.tsv"),
        ),
        (
            "ati-f12",
            include_str!("../assets/ati-f12-registers.csv"),
            include_str!(
                "../../../artifacts/bridge-config/ati-badger-f12-d12-documentation-test.tsv"
            ),
        ),
    ];
    if metadata.len() != sources.len() {
        return Err("Catalog source count does not match metadata".into());
    }
    let mut seen = HashSet::new();
    metadata
        .into_iter()
        .map(|info| {
            if !seen.insert(info.id.clone()) {
                return Err("Duplicate catalog identity".into());
            }
            let (_, csv, tsv) = sources
                .iter()
                .find(|(id, _, _)| *id == info.id)
                .ok_or("Unknown catalog source")?;
            Profile::load(info, csv, tsv)
        })
        .collect()
}
