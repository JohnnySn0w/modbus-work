use crate::{
    bridge::{BridgeSession, Timing},
    console::ConsoleParser,
    contract::*,
    transport::{self, Transport},
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

pub trait Backend: Send + Sync + 'static {
    fn backup_line_settings(
        &self,
        _port: &PortInfo,
        _settings: &crate::bridge::LineSettings,
    ) -> std::io::Result<std::path::PathBuf> {
        Err(std::io::Error::other(
            "Line-settings backup storage unavailable",
        ))
    }
    fn inventory(&self) -> Result<Vec<PortInfo>, String>;
    fn backup(
        &self,
        _port: &PortInfo,
        _table: &str,
    ) -> std::io::Result<Option<std::path::PathBuf>> {
        Ok(None)
    }
    fn open(&self, port: &str) -> std::io::Result<Box<dyn Transport>>;
    fn open_adapter(
        &self,
        _port: &str,
        _settings: crate::adapter::Settings,
    ) -> std::io::Result<Box<dyn crate::adapter::Bus>> {
        Err(std::io::Error::other("USB adapter connection unavailable"))
    }
}
// An explicit UI/packaging mode. It cannot enumerate or open real interfaces.
struct OfflineBackend;
impl Backend for OfflineBackend {
    fn inventory(&self) -> Result<Vec<PortInfo>, String> {
        Ok(vec![])
    }
    fn open(&self, _: &str) -> std::io::Result<Box<dyn Transport>> {
        Err(std::io::Error::other(
            "Hardware access is disabled in offline mode.",
        ))
    }
}
struct SerialBackend;
impl Backend for SerialBackend {
    fn backup_line_settings(
        &self,
        port: &PortInfo,
        settings: &crate::bridge::LineSettings,
    ) -> std::io::Result<std::path::PathBuf> {
        use std::io::Write;
        if !self
            .inventory()
            .map_err(std::io::Error::other)?
            .iter()
            .any(|p| crate::adapter::same_route(port, p))
        {
            return Err(std::io::Error::other(
                "E5 bridge disconnected before line-settings backup",
            ));
        }
        let root = crate::config_file::directory()?.join("Line settings");
        std::fs::create_dir_all(&root)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_nanos();
        let path = root.join(format!("before-change-{stamp}.json"));
        let mut file = std::fs::File::create_new(&path)?;
        let data = serde_json::to_vec_pretty(&serde_json::json!({"usb":port,"settings":settings}))
            .map_err(std::io::Error::other)?;
        file.write_all(&data)?;
        file.sync_all()?;
        Ok(path)
    }
    fn backup(&self, port: &PortInfo, table: &str) -> std::io::Result<Option<std::path::PathBuf>> {
        let inventory = self.inventory().map_err(std::io::Error::other)?;
        let info = inventory
            .iter()
            .find(|p| crate::adapter::same_route(p, port))
            .ok_or_else(|| std::io::Error::other("E5 bridge disconnected before backup."))?;
        crate::config_file::backup(&crate::config_file::directory()?, info, table).map(Some)
    }
    fn inventory(&self) -> Result<Vec<PortInfo>, String> {
        serialport::available_ports()
            .map_err(|e| e.to_string())
            .map(|ports| ports.into_iter().map(port_info).collect())
    }
    fn open(&self, port: &str) -> std::io::Result<Box<dyn Transport>> {
        transport::open(port)
    }
    fn open_adapter(
        &self,
        port: &str,
        settings: crate::adapter::Settings,
    ) -> std::io::Result<Box<dyn crate::adapter::Bus>> {
        crate::adapter::open(port, settings)
    }
}

struct Actor {
    usb: Option<PortInfo>,
    sender: Option<Sender<Command>>,
    active: Arc<AtomicU64>,
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Actor {
    fn spawn(
        port: String,
        backend: Arc<dyn Backend>,
        events: Sender<Event>,
        timing: Timing,
        usb: PortInfo,
    ) -> std::io::Result<Self> {
        let usb = Some(usb);
        let session_usb = usb.clone();
        let (sender, commands) = mpsc::channel::<Command>();
        let active = Arc::new(AtomicU64::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        let busy = active.clone();
        let cancel = cancelled.clone();
        let worker = thread::Builder::new()
            .name(format!("serial-{port}"))
            .spawn(move || {
                let mut session: Option<BridgeSession> = None;
                for command in commands {
                    let emit = |kind| {
                        let _ = events.send(Event {
                            request_id: command.request_id,
                            kind,
                        });
                    };
                    let result = if matches!(command.operation, Operation::ClosePort) {
                        session = None;
                        EventKind::Result {
                            message: format!("{port} console released."),
                        }
                    } else if matches!(command.operation, Operation::AdapterRead) {
                        session = None;
                        let result = (|| {
                            let ports = backend.inventory().map_err(std::io::Error::other)?;
                            let expected = session_usb.as_ref()
                                .filter(|old| ports.iter().any(|p| crate::adapter::same_route(old, p)))
                                .ok_or_else(|| std::io::Error::other("USB adapter disconnected or changed before polling"))?
                                .clone();
                            crate::adapter::poll(
                                expected.clone(),
                                |settings| backend.open_adapter(&port, settings),
                                || {
                                    if cancel.load(Ordering::Acquire) {
                                        return Err(std::io::Error::other(
                                            "Adapter read cancelled",
                                        ));
                                    }
                                    crate::adapter::guard(
                                        &backend.inventory().map_err(std::io::Error::other)?,
                                        &expected,
                                    )
                                },
                            )
                        })();
                        match result {
                            Ok(result) => EventKind::AdapterResult { result },
                            Err(error) => EventKind::Error {
                                code: ErrorCode::Transport,
                                message: error.to_string(),
                                recoverable: true,
                            },
                        }
                    } else {
                        let read_all = matches!(command.operation, Operation::BridgeReadAll);
                        let expected = command
                            .expected_identity
                            .as_ref()
                            .expect("validated by coordinator");
                        let result = (|| {
                            if cancel.load(Ordering::Acquire) {
                                return Err(crate::bridge::BridgeError {
                                    code: ErrorCode::Cancelled,
                                    message: "Operation cancelled before opening the port.".into(),
                                });
                            }
                            let current = backend.inventory().map_err(std::io::Error::other)?;
                            if !session_usb.as_ref().is_some_and(|old| current.iter().any(|p| crate::adapter::same_route(old, p))) {
                                return Err(crate::bridge::BridgeError {
                                    code: ErrorCode::IdentityMismatch,
                                    message: "USB device disconnected or changed; rediscover before retrying.".into(),
                                });
                            }
                            if session.is_none() {
                                emit(EventKind::Progress {
                                    stage: format!("Opening {port} E5 bridge console"),
                                });
                                session = Some(BridgeSession::new(backend.open(&port)?, timing));
                            }
                            if let Operation::BridgeLineSettings { target, reviewed } = &command.operation {
                                return session.as_mut().unwrap().configure_line(expected, target, reviewed, &cancel,
                                    |stage| emit(EventKind::Progress {stage:stage.into()}),
                                    |settings| {
                                        let info = session_usb.as_ref().ok_or_else(|| std::io::Error::other("USB identity unavailable"))?;
                                        let path = backend.backup_line_settings(info,settings)?;
                                        emit(EventKind::Progress { stage: format!("Previous line settings saved to {}", path.display()) });
                                        Ok(())
                                    });
                            }
                            if let Operation::BridgeProgram { target, reviewed } =
                                &command.operation
                            {
                                return session.as_mut().unwrap().program(
                                    expected,
                                    target,
                                    reviewed,
                                    &cancel,
                                    |stage| {
                                        emit(EventKind::Progress {
                                            stage: stage.into(),
                                        })
                                    },
                                    |table| {
                                        let info = session_usb.as_ref().ok_or_else(|| {
                                            std::io::Error::other(
                                                "USB identity unavailable for backup",
                                            )
                                        })?;
                                        let path =
                                            backend.backup(info, table)?.ok_or_else(|| {
                                                std::io::Error::other(
                                                    "Durable backup storage unavailable",
                                                )
                                            })?;
                                        emit(EventKind::Backup {
                                            path: Some(path.to_string_lossy().into()),
                                            error: None,
                                        });
                                        Ok(())
                                    },
                                );
                            }
                            session.as_mut().unwrap().poll_with_export(
                                expected,
                                read_all,
                                &cancel,
                                |stage| {
                                    emit(EventKind::Progress {
                                        stage: stage.into(),
                                    })
                                },
                                |table| match session_usb
                                    .as_ref()
                                    .ok_or_else(|| {
                                        std::io::Error::other("USB identity unavailable for backup")
                                    })
                                    .and_then(|info| backend.backup(info, table))
                                    .and_then(|path| match &command.operation {
                                        Operation::BridgeNamedBackup { name } => {
                                            let path = path.ok_or_else(|| std::io::Error::other("Backup storage unavailable"))?;
                                            crate::config_file::name_backup(&path, name).map(Some)
                                        }
                                        _ => Ok(path),
                                    })
                                {
                                    Ok(Some(path)) => emit(EventKind::Backup {
                                        path: Some(path.to_string_lossy().into()),
                                        error: None,
                                    }),
                                    Ok(None) => {}
                                    Err(e) => emit(EventKind::Backup {
                                        path: None,
                                        error: Some(e.to_string()),
                                    }),
                                },
                            )
                        })();
                        match result {
                            Ok(result) => {
                                if let Some(settings) = session.as_ref().and_then(BridgeSession::line_settings) {
                                    emit(EventKind::LineSettings { settings });
                                }
                                EventKind::BridgeResult { result }
                            },
                            Err(error) => {
                                session = None;
                                EventKind::Error {
                                    code: error.code,
                                    message: error.message,
                                    recoverable: !matches!(
                                        command.operation,
                                        Operation::BridgeProgram { .. } | Operation::BridgeLineSettings { .. }
                                    ),
                                }
                            }
                        }
                    };
                    busy.store(0, Ordering::Release);
                    emit(result);
                }
            })?;
        Ok(Self {
            usb,
            sender: Some(sender),
            active,
            cancelled,
            worker: Some(worker),
        })
    }
}
impl Drop for Actor {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub struct Service {
    commands: Option<Sender<Command>>,
    pub events: Receiver<Event>,
    worker: Option<JoinHandle<()>>,
}
impl Service {
    pub fn offline() -> std::io::Result<Self> {
        Self::with_backend(Arc::new(OfflineBackend), Timing::default())
    }

    pub fn start() -> std::io::Result<Self> {
        Self::with_backend(Arc::new(SerialBackend), Timing::default())
    }
    pub fn with_backend(backend: Arc<dyn Backend>, timing: Timing) -> std::io::Result<Self> {
        let (commands, requests) = mpsc::channel::<Command>();
        let (events, receiver) = mpsc::channel();
        let worker = thread::Builder::new().name("hardware-service".into()).spawn(move || {
            let mut actors: HashMap<String, Actor> = HashMap::new();
            for command in requests {
                let emit = |kind| { let _ = events.send(Event { request_id: command.request_id, kind }); };
                let error = |code, message: &str| emit(EventKind::Error { code, message: message.into(), recoverable: true });
                if command.request_id == 0 { error(ErrorCode::InvalidRequest, "Request identifiers must be nonzero."); continue; }
                match &command.operation {
                    Operation::Inventory | Operation::Replay { .. } | Operation::Cancel { .. }
                        if command.port.is_some() || command.expected_identity.is_some() => {
                        error(ErrorCode::InvalidRequest, "This operation does not accept a port or identity gate."); continue;
                    }
                    _ => {}
                }
                match &command.operation {
                    Operation::Inventory => match backend.inventory() {
                        Ok(mut ports) => {
                            actors.retain(|route, actor| ports.iter().any(|p| p.port.eq_ignore_ascii_case(route) && actor.usb.as_ref().is_some_and(|old| crate::adapter::same_route(old,p))));
                            for port in &mut ports {
                                port.busy = actors.get(&port.port.to_ascii_uppercase()).is_some_and(|a| a.active.load(Ordering::Acquire) != 0);
                            }
                            ports.sort_by(|a, b| a.port.cmp(&b.port));
                            emit(EventKind::PortSnapshot { ports });
                        }
                        Err(message) => error(ErrorCode::InventoryUnavailable, &message),
                    },
                    Operation::Replay { chunks } => {
                        emit(EventKind::Progress { stage: "Running offline console replay".into() });
                        let mut parser = ConsoleParser::default();
                        for chunk in chunks { emit(EventKind::PromptState { state: parser.feed(chunk.as_bytes()) }); }
                        emit(EventKind::Result { message: "Offline replay complete; no serial port opened.".into() });
                    }
                    Operation::Cancel { request_id } => {
                        if *request_id == 0 { error(ErrorCode::InvalidRequest, "A nonzero active request identifier is required."); continue; }
                        let actor = actors.values().find(|actor| actor.active.load(Ordering::Acquire) == *request_id);
                        if let Some(actor) = actor {
                            actor.cancelled.store(true, Ordering::Release);
                            emit(EventKind::Result { message: "Cancellation requested.".into() });
                        } else { error(ErrorCode::InvalidRequest, "The requested operation is no longer active."); }
                    }
                    Operation::BridgeExport | Operation::BridgeNamedBackup { .. } | Operation::BridgeReadAll | Operation::BridgeProgram { .. } | Operation::BridgeLineSettings { .. } | Operation::AdapterRead | Operation::ClosePort => {
                        let Some(port) = command.port.as_ref().map(|p| p.trim().to_ascii_uppercase()).filter(|p| !p.is_empty()) else {
                            error(ErrorCode::InvalidRequest, "Select a serial port."); continue;
                        };
                        let mut selected_usb = None;
                        if matches!(command.operation, Operation::AdapterRead) {
                            let ports = match backend.inventory() { Ok(ports)=>ports, Err(message)=>{error(ErrorCode::InventoryUnavailable,&message);continue;} };
                            let Some(candidate) = ports.iter().find(|p| p.port.eq_ignore_ascii_case(&port)) else {error(ErrorCode::IdentityMismatch,"USB adapter disconnected");continue;};
                            if let Err(e)=crate::adapter::guard(&ports,candidate) {error(ErrorCode::UnsafeState,&e.to_string());continue;}
                            selected_usb = Some(candidate.clone());
                        } else if !matches!(command.operation, Operation::ClosePort) {
                            if command.expected_identity.as_ref() != Some(&Identity { model: "ENL-MOD-32".into(), firmware: "3.6".into() }) {
                                error(ErrorCode::InvalidRequest, "An explicit ENL-MOD-32 firmware 3.6 identity gate is required."); continue;
                            }
                            let candidate = match backend.inventory() {
                                Ok(ports) => ports.into_iter().find(|p| p.port.eq_ignore_ascii_case(&port) && p.usb_vid == Some(0x0483) && p.usb_pid == Some(0x5740)),
                                Err(message) => { error(ErrorCode::InventoryUnavailable, &message); continue; }
                            };
                            if candidate.is_none() { error(ErrorCode::IdentityMismatch, "The selected serial port is not an available Synetica USB interface."); continue; }
                            selected_usb = candidate;
                        }
                        if !matches!(command.operation, Operation::ClosePort) && let Some(actor) = actors.get(&port) {
                            let current = match backend.inventory() { Ok(ports) => ports, Err(message) => { error(ErrorCode::InventoryUnavailable, &message); continue; } };
                            if !actor.usb.as_ref().is_some_and(|old| current.iter().any(|p| crate::adapter::same_route(old, p))) {
                                actors.remove(&port);
                                error(ErrorCode::IdentityMismatch, "USB device changed; previous session released. Rediscover before retrying.");
                                continue;
                            }
                        }
                        if actors.values().any(|a| a.active.load(Ordering::Acquire) == command.request_id) {
                            error(ErrorCode::InvalidRequest, "Request identifier is already active."); continue;
                        }
                        if !actors.contains_key(&port) {
                            if matches!(command.operation, Operation::ClosePort) {
                                emit(EventKind::Result { message: "Port is already released.".into() }); continue;
                            }
                            match Actor::spawn(port.clone(), backend.clone(), events.clone(), timing, selected_usb.expect("validated hardware route")) {
                                Ok(actor) => { actors.insert(port.clone(), actor); },
                                Err(_) => { error(ErrorCode::Transport, "Unable to start serial worker."); continue; }
                            }
                        }
                        let actor = actors.get(&port).unwrap();
                        if actor.active.compare_exchange(0, command.request_id, Ordering::AcqRel, Ordering::Acquire).is_err() {
                            error(ErrorCode::PortBusy, "A console operation is already running on this port."); continue;
                        }
                        actor.cancelled.store(false, Ordering::Release);
                        if actor.sender.as_ref().unwrap().send(command.clone()).is_err() {
                            error(ErrorCode::Transport, "Serial worker stopped; retry the operation.");
                            actors.remove(&port);
                        }
                    }
                }
            }
            for actor in actors.values() { actor.cancelled.store(true, Ordering::Release); }
        })?;
        Ok(Self {
            commands: Some(commands),
            events: receiver,
            worker: Some(worker),
        })
    }
    pub fn send(&self, command: Command) -> Result<(), Box<mpsc::SendError<Command>>> {
        self.commands
            .as_ref()
            .expect("service sender exists until drop")
            .send(command)
            .map_err(Box::new)
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.commands.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

// Keep OS enumeration at the boundary; normalize every route without assuming a COM number.
fn port_info(p: serialport::SerialPortInfo) -> PortInfo {
    let mut info = PortInfo {
        port: p.port_name,
        usb_vid: None,
        usb_pid: None,
        serial_number: None,
        description: "Serial interface".into(),
        identity: None,
        busy: false,
    };
    if let serialport::SerialPortType::UsbPort(usb) = p.port_type {
        info.usb_vid = Some(usb.vid);
        info.usb_pid = Some(usb.pid);
        info.serial_number = usb.serial_number;
        info.description = usb.product.unwrap_or_else(|| "USB serial interface".into());
    }
    info
}

#[cfg(test)]
#[path = "tests/service_boundaries.rs"]
mod boundary_tests;
