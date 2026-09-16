//! Bounded persistent diagnostics for support handoff, including Rust panics.
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const LIMIT: u64 = 5 * 1024 * 1024;
static LOG: OnceLock<Result<Mutex<Log>, String>> = OnceLock::new();

struct Log {
    path: PathBuf,
    error: Option<String>,
}

impl Log {
    /// Rotate before writing; retain the current file and one previous file.
    fn append(&mut self, text: &str) -> io::Result<()> {
        if self.path.metadata().map(|m| m.len()).unwrap_or(0) >= LIMIT {
            let previous = self.path.with_extension("previous.log");
            match std::fs::remove_file(&previous) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
            std::fs::rename(&self.path, previous)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(
            file,
            "{} | {}",
            modbus_configurator::last_good::timestamp(),
            text
        )?;
        file.flush()
    }

    /// Combine retained files in chronological order for a single handoff file.
    fn snapshot(&self) -> io::Result<String> {
        if let Some(error) = &self.error {
            return Err(io::Error::other(error.clone()));
        }
        let mut text = String::new();
        let previous = self.path.with_extension("previous.log");
        match std::fs::read_to_string(previous) {
            Ok(old) => text.push_str(&old),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
        text.push_str(&std::fs::read_to_string(&self.path)?);
        Ok(text)
    }
}

/// Initialize before starting workers, retaining logs across application restarts.
pub(super) fn initialize() {
    LOG.get_or_init(|| {
        let directory = modbus_configurator::config_file::directory()
            .map_err(|e| e.to_string())?
            .join("Logs");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        // Keep ten recent process logs (and their rotation files) across restarts.
        let mut files = session_files(&directory).map_err(|e| e.to_string())?;
        while files.len() >= 10 {
            let old = files.remove(0);
            std::fs::remove_file(&old).map_err(|e| e.to_string())?;
            let _ = std::fs::remove_file(old.with_extension("previous.log"));
        }
        Ok(Mutex::new(Log {
            path: directory.join(format!("diagnostics-{}.log", std::process::id())),
            error: None,
        }))
    });
    write(&format!(
        "Session started | version {} | {} {} | process {} | offline {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::process::id(),
        std::env::args_os().any(|arg| arg == "--offline")
    ));
    write(&format!(
        "Embedded build | {}",
        option_env!("POLYGON_BUILD_ID").unwrap_or("development build")
    ));
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
        && let Ok(bytes) = std::fs::read(parent.join("build-info.json"))
        && let Ok(info) = serde_json::from_slice::<serde_json::Value>(&bytes)
    {
        write(&format!(
            "Build | version {} | commit {} | modified source {} | executable SHA-256 {}",
            info["version"], info["source_commit"], info["source_dirty"], info["exe_sha256"]
        ));
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write(&format!(
            "Rust panic | {info}\n{}",
            std::backtrace::Backtrace::force_capture()
        ));
        previous(info);
    }));
}

/// Logging failures must not interrupt serial work; expose them during export.
pub(super) fn write(text: &str) {
    if let Some(Ok(log)) = LOG.get()
        && let Ok(mut log) = log.lock()
        && let Err(error) = log.append(text)
    {
        log.error = Some(error.to_string());
    }
}

/// Export retained diagnostics; report unavailable logging instead of an empty success.
pub(super) fn export(destination: &Path) -> io::Result<()> {
    let log = LOG
        .get()
        .ok_or_else(|| io::Error::other("Diagnostic logging is not initialized"))?
        .as_ref()
        .map_err(|error| io::Error::other(error.clone()))?;
    let log = log
        .lock()
        .map_err(|_| io::Error::other("Diagnostic log is unavailable"))?;
    let directory = log
        .path
        .parent()
        .ok_or_else(|| io::Error::other("Missing log directory"))?;
    if destination.parent().and_then(|p| p.canonicalize().ok()) == directory.canonicalize().ok() {
        return Err(io::Error::other(
            "Export outside the diagnostic log directory",
        ));
    }
    log.snapshot()?;
    let mut text = String::from(
        "Polygon Device Configurator diagnostics\nLocal timestamps; retained sessions, oldest first.\n",
    );
    for path in session_files(directory)? {
        text.push_str(&Log { path, error: None }.snapshot()?);
    }
    std::fs::write(destination, text)
}

/// Find only application-owned session files, in last-write order.
fn session_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<_> = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("diagnostics-")
                        .and_then(|s| s.strip_suffix(".log"))
                        .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
                })
        })
        .collect();
    files.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistent_support_handoff_survives_a_caught_panic() {
        const CHILD: &str = "POLYGON_LOG_TEST_CHILD";
        if let Some(root) = std::env::var_os(CHILD) {
            let root = PathBuf::from(root);
            initialize();
            write("Sensor request completed");
            assert!(std::panic::catch_unwind(|| panic!("diagnostic test panic")).is_err());
            let output = root.join("support.txt");
            export(&output).unwrap();
            let text = std::fs::read_to_string(output).unwrap();
            for expected in [
                "Session started",
                "Embedded build",
                "Sensor request completed",
                "Rust panic",
                "diagnostic test panic",
                "retained session",
            ] {
                assert!(text.contains(expected), "Missing {expected}");
            }
            let directory = modbus_configurator::config_file::directory()
                .unwrap()
                .join("Logs");
            assert_eq!(session_files(&directory).unwrap().len(), 10);
            assert!(directory.join("unrelated.txt").exists());
            assert!(export(&directory.join("overwrite.txt")).is_err());
            assert!(export(&root.join("missing/support.txt")).is_err());
            return;
        }
        // A separate process isolates the global log and panic hook from parallel tests.
        let root =
            std::env::temp_dir().join(format!("support-handoff-test-{}", std::process::id()));
        let directory = root.join("Polygon/Device Configurator/Logs");
        std::fs::create_dir_all(&directory).unwrap();
        for id in 0..11 {
            std::fs::write(
                directory.join(format!("diagnostics-{id}.log")),
                "retained session\n",
            )
            .unwrap();
        }
        std::fs::write(directory.join("unrelated.txt"), "keep").unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "diagnostic_log::tests::persistent_support_handoff_survives_a_caught_panic",
                "--nocapture",
            ])
            .env(CHILD, &root)
            .env("LOCALAPPDATA", &root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rotated_log_exports_in_order_and_reports_write_failures() {
        let root = std::env::temp_dir().join(format!("diagnostic-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut log = Log {
            path: root.join("test.log"),
            error: None,
        };
        std::fs::write(&log.path, "old\n").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&log.path)
            .unwrap()
            .set_len(LIMIT)
            .unwrap();
        log.append("new event").unwrap();
        let text = log.snapshot().unwrap();
        assert!(text.starts_with("old\n"));
        assert!(text.ends_with(" (local) | new event\n"));
        assert!(log.path.metadata().unwrap().len() < LIMIT);
        log.error = Some("disk write failed".into());
        assert!(
            log.snapshot()
                .unwrap_err()
                .to_string()
                .contains("disk write failed")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
