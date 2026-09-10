//! Validated native tables and crash-resistant Windows/local file persistence.
use crate::{bridge::parse_export, contract::PortInfo};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
const HEADER: &str = "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead";
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn directory() -> io::Result<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(|p| PathBuf::from(p).join("Polygon").join("Device Configurator"))
        .ok_or_else(|| io::Error::other("Local application data directory is unavailable."))
}
/// Preserve existing selections/backups during the product rename; never overwrite new files.
pub fn migrate_previous_installation() -> io::Result<()> {
    let new = directory()?;
    let base = new
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("Invalid data directory"))?;
    migrate_from(&base.join("ExactAire").join("Modbus"), &new)
}
pub fn migrate_from(old: &Path, new: &Path) -> io::Result<()> {
    if !old.try_exists()? {
        return Ok(());
    }
    fs::create_dir_all(new)?;
    let selected = old.join("Selected.tsv");
    if selected.is_file() && !new.join("Selected.tsv").try_exists()? {
        fs::copy(&selected, new.join("Selected.tsv"))?;
    }
    let backups = old.join("Backups");
    if backups.is_dir() {
        for device in fs::read_dir(backups)? {
            let device = device?;
            if !device.file_type()?.is_dir() {
                continue;
            }
            let destination = new.join("Backups").join(device.file_name());
            fs::create_dir_all(&destination)?;
            for file in fs::read_dir(device.path())? {
                let file = file?;
                if file.file_type()?.is_file()
                    && file.path().extension().is_some_and(|e| e == "tsv")
                    && !destination.join(file.file_name()).try_exists()?
                {
                    fs::copy(file.path(), destination.join(file.file_name()))?;
                }
            }
        }
    }
    Ok(())
}
pub fn normalize(text: &str) -> io::Result<String> {
    if text.len() > 32768 {
        return Err(io::Error::other("Configuration exceeds 32 KiB."));
    }
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next() != Some(HEADER) {
        return Err(io::Error::other(
            "Expected an eight-column native E5 bridge TSV file.",
        ));
    }
    let rows: Vec<_> = lines.filter(|l| !l.trim().is_empty()).collect();
    if rows
        .iter()
        .any(|l| !l.as_bytes().first().is_some_and(u8::is_ascii_digit))
    {
        return Err(io::Error::other("Unexpected text in the point table."));
    }
    let parsed =
        parse_export(&rows.join("\n"), rows.len()).map_err(|e| io::Error::other(e.message))?;
    Ok(format!(
        "{HEADER}\r\n{}",
        parsed
            .values()
            .map(|r| format!("{r}\r\n"))
            .collect::<String>()
    ))
}
pub fn load(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(32769)
        .read_to_string(&mut text)?;
    normalize(&text)
}
fn unique() -> String {
    format!(
        "{:020}-{:06}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn save(path: &Path, text: &str) -> io::Result<()> {
    let text = normalize(text)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(".polygon-{}-{}.tmp", std::process::id(), unique()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub fn backup(root: &Path, port: &PortInfo, text: &str) -> io::Result<PathBuf> {
    let text = normalize(text)?;
    let folder = backup_folder(root, port)?;
    fs::create_dir_all(&folder)?;
    let paths = history(root, port)?;
    if let Some(last) = paths.first()
        && load(last).is_ok_and(|old| old == text)
    {
        return Ok(last.clone());
    }
    let path = folder.join(format!("{}.tsv", unique()));
    save(&path, &text)?;
    Ok(path)
}
fn backup_folder(root: &Path, port: &PortInfo) -> io::Result<PathBuf> {
    let serial = port.serial_number.as_deref().filter(|s| !s.trim().is_empty())
        .ok_or_else(|| io::Error::other("USB serial unavailable: cannot safely associate automatic backups with this device. Save the TSV explicitly."))?;
    if port.usb_vid.is_none() || port.usb_pid.is_none() {
        return Err(io::Error::other("USB identity unavailable for backup"));
    }
    // Encode identity, never interpret USB descriptors as filesystem paths.
    let identity = format!("{:?}-{:?}-{}", port.usb_vid, port.usb_pid, serial);
    if identity.len() > 100 {
        return Err(io::Error::other(
            "USB identity is too long for a backup folder.",
        ));
    }
    Ok(root.join("Backups").join(
        identity
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
    ))
}
/// Newest-first history for one USB identity; a missing folder is an empty history.
pub fn history(root: &Path, port: &PortInfo) -> io::Result<Vec<PathBuf>> {
    let folder = backup_folder(root, port)?;
    if !folder.try_exists()? {
        return Ok(vec![]);
    }
    let mut paths = vec![];
    for entry in fs::read_dir(&folder)? {
        let entry = entry?;
        // Interrupted writes, folders and links are not committed backup files.
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|e| e == "tsv") {
            paths.push(entry.path());
        }
    }
    paths.sort();
    paths.reverse();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    const TABLE: &str =
        "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead\n1\t1\tHold\t4\tF32\tHL\t1\tInt\n";
    fn folder() -> PathBuf {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("file-tests")
            .join(unique());
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn missing_usb_identity_never_uses_com_number_as_backup_identity() {
        let port = PortInfo {
            port: "COM7".into(),
            usb_vid: Some(0x483),
            usb_pid: Some(0x5740),
            serial_number: None,
            description: String::new(),
            identity: None,
            busy: false,
        };
        assert!(backup(&folder(), &port, TABLE).is_err());
    }
    #[test]
    fn loads_windows_bom_and_rejects_trailing_junk_and_bad_fields() {
        assert_eq!(
            normalize(&format!("\u{feff}{}", TABLE.replace('\n', "\r\n"))).unwrap(),
            normalize(TABLE).unwrap()
        );
        for bad in [
            format!("{TABLE}not a row"),
            TABLE.replace("\tHL\t", "\tXX\t"),
            TABLE.replace("1\t1\t", "1\t0\t"),
            "a".repeat(32769),
        ] {
            assert!(normalize(&bad).is_err());
        }
    }
    #[test]
    fn atomic_save_replaces_existing_and_invalid_save_preserves_it() {
        let dir = folder();
        let path = dir.join("table.tsv");
        save(&path, TABLE).unwrap();
        let updated = TABLE.replace("\t4\t", "\t6\t");
        save(&path, &updated).unwrap();
        assert_eq!(load(&path).unwrap(), normalize(&updated).unwrap());
        assert!(save(&path, "broken").is_err());
        assert_eq!(load(&path).unwrap(), normalize(&updated).unwrap());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    }
    #[test]
    fn failed_commit_cleans_temporary_file() {
        let dir = folder();
        let target = dir.join("directory");
        fs::create_dir(&target).unwrap();
        assert!(save(&target, TABLE).is_err());
        assert!(target.is_dir());
        assert_eq!(fs::read_dir(dir).unwrap().count(), 1);
    }
    #[test]
    fn backups_deduplicate_then_preserve_change_and_revert_history() {
        let root = folder();
        let mut port = PortInfo {
            port: "COM5".into(),
            usb_vid: Some(0x483),
            usb_pid: Some(0x5740),
            serial_number: Some("../../unit".into()),
            description: String::new(),
            identity: None,
            busy: false,
        };
        assert!(history(&root, &port).unwrap().is_empty());
        let a = backup(&root, &port, TABLE).unwrap();
        assert!(a.starts_with(root.join("Backups")));
        assert_eq!(a, backup(&root, &port, TABLE).unwrap());
        port.port = "COM9".into();
        assert_eq!(a, backup(&root, &port, TABLE).unwrap());
        let b = backup(&root, &port, &TABLE.replace("\t4\t", "\t6\t")).unwrap();
        let c = backup(&root, &port, TABLE).unwrap();
        assert_eq!(
            history(&root, &port).unwrap(),
            vec![c.clone(), b.clone(), a.clone()]
        );
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_eq!(load(&a).unwrap(), normalize(TABLE).unwrap());
        assert_eq!(fs::read_dir(a.parent().unwrap()).unwrap().count(), 3);
    }
}
