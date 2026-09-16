//! Compose sensor selections into one Modbus Bridge table without changing wire addresses.
use crate::catalog::{Profile, table_rows};
use std::collections::BTreeSet;

/// One physical sensor, including repeated models at different slave addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub profile: usize,
    pub slave: u8,
    pub points: BTreeSet<u8>,
}

impl Device {
    /// Select the reviewed entries for a model; callers may deselect individual entries.
    pub fn new(profile: usize, slave: u8, profiles: &[Profile]) -> Self {
        Self {
            profile,
            slave,
            points: profiles[profile].rows.keys().copied().collect(),
        }
    }
}

/// Validate the entire network before assigning unique bridge point numbers.
pub fn compose(devices: &[Device], profiles: &[Profile]) -> Result<String, String> {
    if devices.is_empty() || devices.len() > 32 {
        return Err("Add between one and 32 devices.".into());
    }
    let mut slaves = BTreeSet::new();
    let mut rows = Vec::new();
    for device in devices {
        if !(1..=247).contains(&device.slave) || !slaves.insert(device.slave) {
            return Err("Each device needs a different slave address from 1 to 247.".into());
        }
        let profile = profiles
            .get(device.profile)
            .ok_or("Unknown sensor model.")?;
        if device.points.is_empty() {
            return Err("Select at least one register entry for each device.".into());
        }
        for point in &device.points {
            let row = profile.rows.get(point).ok_or("Unknown register entry.")?;
            let mut fields: Vec<String> = row.split('\t').map(str::to_owned).collect();
            fields[0] = (rows.len() + 1).to_string();
            fields[1] = device.slave.to_string();
            rows.push(fields.join("\t"));
        }
    }
    if rows.len() > 32 {
        return Err(format!(
            "{} of 32 entries selected. Deselect {} to continue.",
            rows.len(),
            rows.len() - 32
        ));
    }
    let table = format!(
        "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\r\n{}\r\n",
        rows.join("\r\n")
    );
    table_rows(&table)?;
    Ok(table)
}

/// Recover an editable selection only when every row has one unambiguous model match.
/// Unknown/custom rows are never silently dropped or assigned a guessed model.
pub fn recognize(table: &str, profiles: &[Profile]) -> Option<Vec<Device>> {
    let rows = table_rows(table).ok()?;
    let slaves: Vec<u8> = rows
        .values()
        .filter_map(|r| r.split('\t').nth(1)?.parse().ok())
        .fold(Vec::new(), |mut ids, id| {
            if !ids.contains(&id) {
                ids.push(id);
            }
            ids
        });
    if slaves.is_empty() || slaves.len() > 32 {
        return None;
    }
    let mut devices = Vec::new();
    for slave in slaves {
        let actual: Vec<_> = rows
            .values()
            .filter(|r| r.split('\t').nth(1) == Some(&slave.to_string()))
            .collect();
        let candidates: Vec<_> = profiles
            .iter()
            .enumerate()
            .filter_map(|(index, profile)| {
                let mut points = BTreeSet::new();
                for row in &actual {
                    let matches: Vec<_> = profile
                        .rows
                        .iter()
                        .filter(|(_, candidate)| {
                            candidate.split('\t').skip(2).eq(row.split('\t').skip(2))
                        })
                        .collect();
                    if matches.len() != 1 || !points.insert(*matches[0].0) {
                        return None;
                    }
                }
                Some(Device {
                    profile: index,
                    slave,
                    points,
                })
            })
            .collect();
        if candidates.len() != 1 {
            return None;
        }
        devices.push(candidates.into_iter().next()?);
    }
    Some(devices)
}
