//! User-named display profiles match complete register definitions, independent of slave IDs.
use crate::{
    catalog::{Profile, table_rows},
    config_file,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedProfile {
    pub name: String,
    pub model_id: String,
    pub points: Vec<String>,
}

/// Capture one slave's full point definitions without its route or point numbering.
fn signature(table: &str, slave: u8) -> Result<Vec<String>, String> {
    let mut points: Vec<_> = table_rows(table)?
        .values()
        .filter(|row| row.split('\t').nth(1).and_then(|id| id.parse::<u8>().ok()) == Some(slave))
        .map(|row| row.split('\t').skip(2).collect::<Vec<_>>().join("\t"))
        .collect();
    points.sort();
    if points.is_empty() {
        return Err("No register entries for this slave.".into());
    }
    Ok(points)
}

impl SavedProfile {
    /// Record the operator's model selection; this does not establish physical identity.
    pub fn new(name: &str, model_id: &str, table: &str, slave: u8) -> Result<Self, String> {
        config_file::validate_backup_name(name).map_err(|e| e.to_string())?;
        Ok(Self {
            name: name.trim().into(),
            model_id: model_id.into(),
            points: signature(table, slave)?,
        })
    }

    pub fn matches(&self, table: &str, slave: u8) -> bool {
        signature(table, slave).is_ok_and(|points| points == self.points)
    }

    /// Validate stored content before using it or replacing an existing profile library.
    fn validate(&self, catalog: &[Profile]) -> Result<(), String> {
        if !catalog.iter().any(|p| p.info.id == self.model_id) {
            return Err("Saved device type is not in the catalog.".into());
        }
        let rows = self
            .points
            .iter()
            .enumerate()
            .map(|(i, row)| format!("{}\t1\t{row}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        let rebuilt = Self::new(
            &self.name,
            &self.model_id,
            &format!("Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\n{rows}\n"),
            1,
        )?;
        if rebuilt != *self {
            return Err("Saved register profile is not normalized.".into());
        }
        Ok(())
    }
}

/// Select a saved type only when every matching profile agrees about the model.
pub fn matching_model(
    saved: &[SavedProfile],
    table: &str,
    slave: u8,
    catalog: &[Profile],
) -> Option<usize> {
    let models: std::collections::BTreeSet<_> = saved
        .iter()
        .filter(|p| p.matches(table, slave))
        .map(|p| &p.model_id)
        .collect();
    if models.len() != 1 {
        return None;
    }
    catalog
        .iter()
        .position(|p| Some(&p.info.id) == models.first().copied())
}

/// Load a bounded local library; missing storage means no saved profiles.
pub fn load(root: &Path, catalog: &[Profile]) -> io::Result<Vec<SavedProfile>> {
    let path = root.join("Custom-profiles.json");
    let mut bytes = Vec::new();
    match fs::File::open(path) {
        Ok(file) => {
            file.take(262145).read_to_end(&mut bytes)?;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e),
    }
    if bytes.len() > 262144 {
        return Err(io::Error::other("Custom profile library is too large."));
    }
    let saved: Vec<SavedProfile> = serde_json::from_slice(&bytes)?;
    for profile in &saved {
        profile.validate(catalog).map_err(io::Error::other)?;
    }
    Ok(saved)
}

/// Atomically save the library, preserving the previous file if writing fails.
pub fn save(root: &Path, saved: &[SavedProfile], catalog: &[Profile]) -> io::Result<()> {
    for profile in saved {
        profile.validate(catalog).map_err(io::Error::other)?;
    }
    let bytes = serde_json::to_vec_pretty(saved)?;
    if bytes.len() > 262144 {
        return Err(io::Error::other("Custom profile library is too large."));
    }
    fs::create_dir_all(root)?;
    let temporary = root.join(format!(
        ".custom-profiles-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, root.join("Custom-profiles.json"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
