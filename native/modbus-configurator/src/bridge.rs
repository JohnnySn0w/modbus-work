//! Firmware 3.6 console workflows through one persistent transport.
mod line_settings;
mod parsing;
pub use line_settings::LineSettings;
use parsing::{export_count, verify_summary};
pub use parsing::{parse_export, parse_point_report};

use crate::{
    console::{ConsoleParser, PromptState},
    contract::{ErrorCode, Identity},
    transport::Transport,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const HEADER: &str = "Item\tID\tReg\tAddr\tData\tWord\tMult\tRead";

#[derive(Debug)]
pub struct BridgeError {
    pub code: ErrorCode,
    pub message: String,
}
impl BridgeError {
    fn new(code: ErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl From<io::Error> for BridgeError {
    fn from(_: io::Error) -> Self {
        Self::new(
            ErrorCode::Transport,
            "Serial communication failed; the session was closed. Reconnect and retry.",
        )
    }
}
type Result<T> = std::result::Result<T, BridgeError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub item: u8,
    pub value: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointException {
    pub item: u8,
    pub code: u8,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResult {
    pub identity: Identity,
    pub native_tsv: String,
    pub readings: Vec<Reading>,
    pub successful_reads: Option<usize>,
    #[serde(default)]
    pub exceptions: Vec<PointException>,
}

#[derive(Clone, Copy)]
pub struct Timing {
    pub response: Duration,
    pub quiet: Duration,
    pub operation: Duration,
}
impl Default for Timing {
    fn default() -> Self {
        Self {
            response: Duration::from_secs(5),
            quiet: Duration::from_millis(200),
            operation: Duration::from_secs(90),
        }
    }
}

pub struct BridgeSession {
    transport: Box<dyn Transport>,
    parser: ConsoleParser,
    identity: Option<Identity>,
    login: Option<String>,
    timing: Timing,
    received_bytes: usize,
    cached_table: Option<(BridgeResult, Instant)>,
    import_ack: Option<&'static str>,
}

impl BridgeSession {
    fn response_ready(&self) -> bool {
        if let Some(ack) = self.import_ack {
            if ack == "Import Finished" {
                return self
                    .parser
                    .text()
                    .lines()
                    .any(|line| line.trim() == "Import Finished. Results:")
                    && self.parser.state() == PromptState::Continue;
            }
            return self
                .parser
                .text()
                .lines()
                .any(|line| line.trim().ends_with(ack));
        }
        let state = self.parser.state();
        if !self.transport.requires_menu_selection_prompt() {
            return state != PromptState::Unknown;
        }
        let last = self
            .parser
            .text()
            .trim_end()
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        match state {
            PromptState::MainMenu
            | PromptState::ModbusMenu
            | PromptState::ImportExport
            | PromptState::RadioMenu
            | PromptState::ReadOptions => {
                last.starts_with("enter selection") && last.ends_with(':')
            }
            PromptState::Password => last == "password:",
            PromptState::LineSettingInput => last.ends_with(':'),
            PromptState::Continue => {
                last == "press a key to continue" || last == "press a key to continue:"
            }
            PromptState::ImportInput => true,
            _ => false,
        }
    }
    pub fn new(transport: Box<dyn Transport>, timing: Timing) -> Self {
        Self {
            transport,
            parser: ConsoleParser::default(),
            identity: None,
            login: None,
            timing,
            received_bytes: 0,
            cached_table: None,
            import_ack: None,
        }
    }

    fn check(cancel: &AtomicBool, deadline: Instant) -> Result<()> {
        if cancel.load(Ordering::Acquire) {
            return Err(BridgeError::new(
                ErrorCode::Cancelled,
                "Operation cancelled; session closed.",
            ));
        }
        if Instant::now() >= deadline {
            return Err(BridgeError::new(
                ErrorCode::Timeout,
                "Console response deadline expired; session closed.",
            ));
        }
        Ok(())
    }

    // A recognized prompt must be followed by quiet input. This allows later
    // menu fields/identity/summary to arrive in separate serial reads.
    fn receive(
        &mut self,
        cancel: &AtomicBool,
        deadline: Instant,
        allow_silent: bool,
    ) -> Result<()> {
        self.receive_segment(cancel, deadline, allow_silent, false)
    }

    fn receive_segment(
        &mut self,
        cancel: &AtomicBool,
        deadline: Instant,
        allow_silent: bool,
        preserve: bool,
    ) -> Result<()> {
        self.received_bytes = 0;
        let mut last_data = Instant::now();
        let mut last_recovery = Instant::now();
        let mut recoveries = 0;
        let mut received = false;
        let mut total = if preserve {
            self.parser.text().len()
        } else {
            0
        };
        let mut bytes = [0; 1024];
        loop {
            Self::check(cancel, deadline)?;
            match self.transport.read(&mut bytes) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
                Ok(count) => {
                    self.received_bytes += count;
                    if !received && !preserve {
                        self.parser.reset();
                    }
                    received = true;
                    total += count;
                    if total > crate::console::MAX_RESPONSE_BYTES {
                        return Err(BridgeError::new(
                            ErrorCode::InvalidResponse,
                            "Console response exceeded the supported size.",
                        ));
                    }
                    self.parser.feed(&bytes[..count]);
                    last_data = Instant::now();
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    if self.transport.continuous_receive()
                        && (!allow_silent || received)
                        && !self.response_ready()
                        && last_data.elapsed() >= Duration::from_millis(500)
                        && last_recovery.elapsed() >= Duration::from_millis(500)
                        && recoveries < 16
                    {
                        if let Err(e) = self.transport.resume_receive()
                            && e.kind() != io::ErrorKind::Unsupported
                        {
                            return Err(e.into());
                        }
                        recoveries += 1;
                        last_recovery = Instant::now();
                    }
                    if !self.transport.continuous_receive()
                        && received
                        && last_data.elapsed() >= self.timing.quiet
                        && !self.response_ready()
                    {
                        return Err(BridgeError::new(
                            ErrorCode::Timeout,
                            "Console reply stalled before its final prompt.",
                        ));
                    }
                    if last_data.elapsed() >= self.timing.quiet
                        && ((received && self.response_ready()) || (allow_silent && !received))
                    {
                        return Ok(());
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn send(
        &mut self,
        line: &str,
        cancel: &AtomicBool,
        deadline: Instant,
        timeout: Duration,
    ) -> Result<()> {
        Self::check(cancel, deadline)?;
        if self.transport.continuous_receive() {
            self.parser.reset();
            self.transport.write(format!("{line}\r").as_bytes())?;
            return self.receive(cancel, deadline.min(Instant::now() + timeout), false);
        }
        let requesting_banner = line.is_empty()
            && self.parser.state() == PromptState::Password
            && self.identity.is_none();
        let logging_off = (line == "X"
            && matches!(
                self.parser.state(),
                PromptState::MainMenu
                    | PromptState::ModbusMenu
                    | PromptState::ImportExport
                    | PromptState::RadioMenu
            ))
            || (line.is_empty()
                && self
                    .parser
                    .text()
                    .to_ascii_lowercase()
                    .contains("modbus read completed")
                && matches!(
                    self.parser.state(),
                    PromptState::ReadComplete | PromptState::Continue
                ));
        // Clear the previous exchange before every command. Old prompts never
        // satisfy a new command, even if the new response never arrives.
        self.parser.reset();
        self.transport.write(format!("{line}\r").as_bytes())?;
        let mut response = self.receive(
            cancel,
            deadline.min(Instant::now() + timeout.min(self.timing.response)),
            false,
        );
        if response
            .as_ref()
            .is_err_and(|e| matches!(e.code, ErrorCode::Timeout))
            && self.identity.is_none()
        {
            self.observe_identity()?;
        }
        for _ in 0..32 {
            if response
                .as_ref()
                .is_err_and(|e| matches!(e.code, ErrorCode::Timeout))
                && (self.identity.is_some() || logging_off || requesting_banner)
            {
                Self::check(cancel, deadline)?;
                // Resume the same reply across bounded host reopens. Preserve any
                // received prefix, never resend the command, and validate the full result.
                if self.transport.reconnect().is_ok() {
                    response = self.receive_segment(
                        cancel,
                        deadline.min(Instant::now() + timeout.min(self.timing.response)),
                        false,
                        true,
                    );
                    continue;
                }
            }
            break;
        }
        response
    }

    fn observe_identity(&mut self) -> Result<()> {
        let field = |label: &str| {
            self.parser
                .text()
                .lines()
                .filter_map(|line| {
                    let (name, value) = line.trim().split_once(':')?;
                    name.eq_ignore_ascii_case(label)
                        .then(|| value.trim().to_string())
                })
                .next_back()
        };
        if let Some(model) = field("Model Number")
            && model != "ENL-MOD-32"
        {
            return Err(BridgeError::new(
                ErrorCode::IdentityMismatch,
                "The console is not an ENL-MOD-32 E5 bridge.",
            ));
        }
        if let Some(firmware) = field("Firmware Ver")
            && firmware != "3.6"
        {
            return Err(BridgeError::new(
                ErrorCode::IdentityMismatch,
                "This operation supports E5 bridge firmware 3.6 only.",
            ));
        }
        if let (Some(model), Some(firmware)) = (field("Model Number"), field("Firmware Ver")) {
            self.identity = Some(Identity { model, firmware });
        }
        if let Some(eui) = field("DevEui") {
            let normalized = eui.replace('-', "").to_ascii_lowercase();
            if normalized.len() == 16 && normalized.bytes().all(|b| b.is_ascii_hexdigit()) {
                self.login = Some(normalized[12..].into());
            }
        }
        Ok(())
    }

    fn require(&self, state: PromptState) -> Result<()> {
        if self.parser.state() != state {
            return Err(BridgeError::new(
                ErrorCode::UnsafeState,
                "The console did not reach the expected menu.",
            ));
        }
        Ok(())
    }

    pub fn run(
        &mut self,
        expected: &Identity,
        read_all: bool,
        cancel: &AtomicBool,
        progress: impl FnMut(&str),
    ) -> Result<BridgeResult> {
        self.run_with_export(expected, read_all, cancel, progress, |_| {})
    }

    pub fn run_with_export(
        &mut self,
        expected: &Identity,
        read_all: bool,
        cancel: &AtomicBool,
        mut progress: impl FnMut(&str),
        mut exported: impl FnMut(&str),
    ) -> Result<BridgeResult> {
        let deadline = Instant::now() + self.timing.operation;
        progress("Checking E5 bridge console and identity");
        let initial = self.receive(
            cancel,
            deadline.min(Instant::now() + self.timing.response),
            self.identity.is_some(),
        );
        match initial {
            Err(error)
                if matches!(error.code, ErrorCode::Timeout)
                    && self.identity.is_none()
                    && self.received_bytes == 0 =>
            {
                // One console wake on an explicit action, matching the existing
                // console helper. Never use it after any observed unknown/import
                // output, or as a retry for a failed navigation command.
                progress("Waking silent USB console once");
                self.send("", cancel, deadline, self.timing.response)?;
            }
            other => other?,
        }
        self.observe_identity()?;
        for _ in 0..4 {
            if self.identity.is_some() {
                break;
            }
            if self.parser.state() == PromptState::Password {
                // Observed firmware behavior: an empty login redraws the identity
                // banner. Never submit a derived login without that fresh banner.
                self.send("", cancel, deadline, self.timing.response)?;
                self.observe_identity()?;
                break;
            }
            if matches!(
                self.parser.state(),
                PromptState::Continue | PromptState::ReadComplete
            ) && self
                .parser
                .text()
                .to_ascii_lowercase()
                .contains("modbus read completed")
            {
                self.send("", cancel, deadline, self.timing.response)?;
                self.observe_identity()?;
                continue;
            }
            let options: Vec<_> = self
                .parser
                .text()
                .lines()
                .map(|line| {
                    line.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_ascii_lowercase()
                })
                .collect();
            let exit = match self.parser.state() {
                PromptState::MainMenu => "x - exit and log off",
                PromptState::ModbusMenu | PromptState::ImportExport | PromptState::RadioMenu => {
                    "x - exit menu"
                }
                _ => break,
            };
            if !options.iter().any(|line| line == exit) {
                break;
            }
            self.send("X", cancel, deadline, self.timing.response)?;
            self.observe_identity()?;
        }
        let identity = self.identity.clone().ok_or_else(|| {
            BridgeError::new(
                ErrorCode::IdentityMismatch,
                "No complete E5 bridge identity banner received. Reconnect the USB console and retry.",
            )
        })?;
        if &identity != expected {
            return Err(BridgeError::new(
                ErrorCode::IdentityMismatch,
                "E5 bridge identity does not match the requested model and firmware.",
            ));
        }
        // Known non-mutating menus can be left with X. An import prompt must
        // never receive a blank line: that would finalize someone else's import.
        for attempt in 0..8 {
            let command = match self.parser.state() {
                PromptState::MainMenu => break,
                PromptState::Password => self.login.clone().ok_or_else(|| {
                    BridgeError::new(
                        ErrorCode::IdentityMismatch,
                        "A complete DevEUI is required for console login.",
                    )
                })?,
                PromptState::ModbusMenu | PromptState::ImportExport | PromptState::RadioMenu => {
                    "X".into()
                }
                _ => {
                    return Err(BridgeError::new(
                        ErrorCode::UnsafeState,
                        "Console is in an unknown or unfinished operation. Restore the main menu manually and retry.",
                    ));
                }
            };
            if attempt == 7 {
                return Err(BridgeError::new(
                    ErrorCode::UnsafeState,
                    "Console menu recovery limit reached.",
                ));
            }
            self.send(&command, cancel, deadline, self.timing.response)?;
            self.observe_identity()?;
        }
        self.require(PromptState::MainMenu)?;
        progress("Reading native E5 bridge point table");
        self.send("C", cancel, deadline, self.timing.response)?;
        self.require(PromptState::ModbusMenu)?;
        self.send("M", cancel, deadline, self.timing.response)?;
        self.require(PromptState::ImportExport)?;
        let count = export_count(self.parser.text())?;
        self.send("E", cancel, deadline, self.timing.response)?;
        self.require(PromptState::Continue)?;
        let rows = parse_export(self.parser.text(), count)?;
        self.send("", cancel, deadline, self.timing.response)?;
        self.require(PromptState::ImportExport)?;
        self.send("X", cancel, deadline, self.timing.response)?;
        self.require(PromptState::ModbusMenu)?;
        let mut result = BridgeResult {
            identity,
            native_tsv: format!(
                "{HEADER}\r\n{}",
                rows.values()
                    .map(|row| format!("{row}\r\n"))
                    .collect::<String>()
            ),
            readings: vec![],
            successful_reads: None,
            exceptions: vec![],
        };
        exported(&result.native_tsv);
        self.cached_table = Some((result.clone(), Instant::now()));
        if read_all {
            self.read_points(&mut result, cancel, deadline, &mut progress)?;
        }
        progress("Read-only operation verified");
        Ok(result)
    }
    /// Replace a reviewed point table. The backup callback must durably save the
    /// freshly exported table before returning. Commands are never retransmitted.
    pub fn program(
        &mut self,
        expected: &Identity,
        target: &str,
        reviewed: &str,
        cancel: &AtomicBool,
        mut progress: impl FnMut(&str),
        backup: impl FnOnce(&str) -> io::Result<()>,
    ) -> Result<BridgeResult> {
        let validate = |text| {
            crate::config_file::normalize(text)
                .map_err(|e| BridgeError::new(ErrorCode::InvalidRequest, &e.to_string()))
        };
        let target = validate(target)?;
        let reviewed = validate(reviewed)?;
        if target.lines().count() < 2 {
            return Err(BridgeError::new(
                ErrorCode::InvalidRequest,
                "Select at least one point to program.",
            ));
        }
        let current = self.run(expected, false, cancel, &mut progress)?;
        if current.native_tsv != reviewed {
            return Err(BridgeError::new(
                ErrorCode::UnsafeState,
                "The E5 bridge table changed since review. Review the fresh table before programming; no changes were made.",
            ));
        }
        progress("Saving pre-programming point-table backup");
        backup(&current.native_tsv).map_err(|e| {
            BridgeError::new(
                ErrorCode::UnsafeState,
                &format!("Backup failed; no changes were made: {e}"),
            )
        })?;
        Self::check(cancel, Instant::now() + self.timing.operation)?;
        self.cached_table = None;
        let deadline = Instant::now() + self.timing.operation;
        let mut entered_import = false;
        let outcome = (|| {
            self.send("M", cancel, deadline, self.timing.response)?;
            self.require(PromptState::ImportExport)?;
            // The firmware helper's replacement protocol deletes existing rows
            // using ID 0, then imports the selected rows in a separate batch.
            let deleted: Vec<String> = current
                .native_tsv
                .lines()
                .skip(1)
                .map(|row| {
                    let mut fields: Vec<_> = row.split('\t').collect();
                    fields[1] = "0";
                    fields.join("\t")
                })
                .collect();
            for (rows, ack, stage) in [
                (deleted, "deleted OK", "Removing old point table"),
                (
                    target.lines().skip(1).map(str::to_owned).collect(),
                    "imported OK",
                    "Programming selected point table",
                ),
            ] {
                if rows.is_empty() {
                    continue;
                }
                progress(stage);
                entered_import = true;
                self.send("I", cancel, deadline, self.timing.response)?;
                self.require(PromptState::ImportInput)?;
                let total = rows.len();
                progress(&format!("{stage} (0/{total})"));
                for (index, row) in rows.into_iter().enumerate() {
                    self.import_ack = Some(ack);
                    let response = self.send(&row, cancel, deadline, self.timing.response);
                    self.import_ack = None;
                    response?;
                    progress(&format!("{stage} ({}/{total})", index + 1));
                }
                self.import_ack = Some("Import Finished");
                let response = self.send("", cancel, deadline, self.timing.response);
                self.import_ack = None;
                response?;
                self.send("", cancel, deadline, self.timing.response)?;
                self.require(PromptState::ImportExport)?;
            }
            progress("Verifying programmed table by export");
            let count = export_count(self.parser.text())?;
            self.send("E", cancel, deadline, self.timing.response)?;
            self.require(PromptState::Continue)?;
            let rows = parse_export(self.parser.text(), count)?;
            let actual = format!(
                "{HEADER}\r\n{}",
                rows.values()
                    .map(|r| format!("{r}\r\n"))
                    .collect::<String>()
            );
            if actual != target {
                return Err(BridgeError::new(
                    ErrorCode::InvalidResponse,
                    "Exported table does not match the selected configuration.",
                ));
            }
            self.send("", cancel, deadline, self.timing.response)?;
            self.require(PromptState::ImportExport)?;
            self.send("X", cancel, deadline, self.timing.response)?;
            self.require(PromptState::ModbusMenu)?;
            Ok(BridgeResult {
                identity: current.identity,
                native_tsv: actual,
                readings: vec![],
                successful_reads: None,
                exceptions: vec![],
            })
        })();
        let result = outcome.map_err(|e| {
            if entered_import {
                BridgeError::new(ErrorCode::ProgrammingUncertain, &format!("Programming stopped; the point table may be partially changed. Automatic polling is paused. Use the saved backup after recovering the console. {}", e.message))
            } else { e }
        })?;
        self.cached_table = Some((result.clone(), Instant::now()));
        progress("Point table programmed and verified; live readings will follow");
        Ok(result)
    }

    fn read_points(
        &mut self,
        result: &mut BridgeResult,
        cancel: &AtomicBool,
        deadline: Instant,
        progress: &mut impl FnMut(&str),
    ) -> Result<()> {
        let count = result
            .native_tsv
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count();
        let rows = parse_export(&result.native_tsv, count)?;
        if count == 0 {
            return Err(BridgeError::new(
                ErrorCode::InvalidResponse,
                "The E5 bridge has no configured points to read.",
            ));
        }
        progress("Reading all configured Modbus points through the E5 bridge");
        self.send("A", cancel, deadline, Duration::from_secs(60))?;
        if self.parser.state() == PromptState::ReadOptions {
            self.send("D", cancel, deadline, Duration::from_secs(60))?;
        }
        if !self
            .parser
            .text()
            .to_ascii_lowercase()
            .contains("modbus read completed")
        {
            return Err(BridgeError::new(
                ErrorCode::InvalidResponse,
                "E5 bridge Read All did not complete.",
            ));
        }
        if !matches!(
            self.parser.state(),
            PromptState::ReadComplete | PromptState::Continue
        ) {
            return Err(BridgeError::new(
                ErrorCode::UnsafeState,
                "Read All ended at an unexpected prompt.",
            ));
        }
        let (readings, exceptions) = parse_point_report(self.parser.text(), &rows)?;
        result.readings = readings;
        result.exceptions = exceptions;
        self.send("", cancel, deadline, self.timing.response)?;
        self.require(PromptState::ModbusMenu)?;
        verify_summary(
            self.parser.text(),
            result.readings.len(),
            result.exceptions.len(),
        )?;
        result.successful_reads = Some(result.readings.len());

        Ok(())
    }
    pub fn poll_with_export(
        &mut self,
        expected: &Identity,
        read_all: bool,
        cancel: &AtomicBool,
        mut progress: impl FnMut(&str),
        exported: impl FnMut(&str),
    ) -> Result<BridgeResult> {
        // A live table is refreshed once a minute, on explicit export, or after
        // session recovery. Measurement headers and summary validate cached rows.
        if read_all
            && self.cached_table.as_ref().is_some_and(|(table, at)| {
                &table.identity == expected && at.elapsed() < Duration::from_secs(60)
            })
        {
            let deadline = Instant::now() + self.timing.operation;
            self.receive(
                cancel,
                deadline.min(Instant::now() + self.timing.response),
                true,
            )?;
            if self.parser.state() == PromptState::ModbusMenu {
                let mut result = self.cached_table.as_ref().unwrap().0.clone();
                self.read_points(&mut result, cancel, deadline, &mut progress)?;
                return Ok(result);
            }
        }
        self.run_with_export(expected, read_all, cancel, progress, exported)
    }
}
