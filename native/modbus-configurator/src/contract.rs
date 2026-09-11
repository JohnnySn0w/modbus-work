use crate::console::PromptState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub model: String,
    pub firmware: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub request_id: u64,
    pub port: Option<String>,
    pub expected_identity: Option<Identity>,
    #[serde(flatten)]
    pub operation: Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "payload", rename_all = "snake_case")]
pub enum Operation {
    Inventory,
    Replay {
        chunks: Vec<String>,
    },
    BridgeExport,
    BridgeNamedBackup {
        name: String,
    },
    BridgeReadAll,
    BridgeProgram {
        target: String,
        reviewed: String,
    },
    BridgeLineSettings {
        target: crate::bridge::LineSettings,
        reviewed: crate::bridge::LineSettings,
    },
    AdapterRead,
    ClosePort,
    Cancel {
        request_id: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub port: String,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub serial_number: Option<String>,
    pub description: String,
    pub identity: Option<Identity>,
    pub busy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub request_id: u64,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload", rename_all = "snake_case")]
pub enum EventKind {
    LineSettings {
        settings: crate::bridge::LineSettings,
    },
    Backup {
        path: Option<String>,
        error: Option<String>,
    },
    AdapterResult {
        result: crate::adapter::AdapterResult,
    },
    BridgeResult {
        result: crate::bridge::BridgeResult,
    },
    PortSnapshot {
        ports: Vec<PortInfo>,
    },
    Progress {
        stage: String,
    },
    PromptState {
        state: PromptState,
    },
    Result {
        message: String,
    },
    Error {
        code: ErrorCode,
        message: String,
        recoverable: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InventoryUnavailable,
    InvalidRequest,
    PortBusy,
    Transport,
    Timeout,
    Cancelled,
    IdentityMismatch,
    UnsafeState,
    InvalidResponse,
    ProgrammingUncertain,
}
