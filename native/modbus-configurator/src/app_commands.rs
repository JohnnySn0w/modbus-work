//! Foreground command queue and exclusive hardware request ownership.
use super::*;

impl Configurator {
    /// Select a unique Modbus Bridge route; return false when it is ambiguous or missing.
    pub(super) fn select_bridge_route(&mut self) -> bool {
        let candidates: Vec<_> = self
            .ports
            .iter()
            .filter(|p| p.usb_vid == Some(0x0483) && p.usb_pid == Some(0x5740))
            .collect();
        if candidates.iter().any(|p| p.port == self.selected) {
            return true;
        }
        if candidates.len() == 1 {
            self.selected = candidates[0].port.clone();
            return true;
        }
        self.status =
            "Select one Synetica console in Console diagnostics before starting this action."
                .into();
        false
    }
    /// Translate device-page actions into navigation or guarded hardware requests.
    pub(super) fn handle_action(&mut self, action: technician_view::Action) {
        use technician_view::Action;
        match action {
            Action::Refresh => {
                self.auto_paused = false;
                self.last_fetch = Instant::now() - Duration::from_secs(5);
                if self.scan_pending.is_none() {
                    self.scan_pending = self.request(Operation::Inventory, false);
                }
            }
            Action::BackupBridge => {
                if self.select_bridge_route() {
                    self.hardware(Operation::BridgeExport);
                }
            }
            Action::ReadBridge => {
                if !self
                    .ports
                    .iter()
                    .any(modbus_configurator::adapter::is_bridge)
                {
                    if let Some(route) = modbus_configurator::adapter::choose_route(
                        &self.ports,
                        self.preferred_route
                            .as_ref()
                            .filter(|p| modbus_configurator::adapter::is_adapter(p)),
                        modbus_configurator::adapter::is_adapter,
                    )
                    .cloned()
                    {
                        self.selected = route.port;
                        self.hardware(Operation::AdapterRead);
                    }
                } else if self.select_bridge_route() {
                    self.hardware(Operation::BridgeReadAll);
                }
            }
            Action::ChooseConfig(key) => {
                self.catalog_view.selected = self
                    .profiles
                    .iter()
                    .position(|p| p.info.id == modbus_configurator::reference::catalog_id(&key));
                if let Some(index) = self.catalog_view.selected {
                    self.loaded_config = Some(self.profiles[index].native_tsv.clone());
                    self.config_source = self.profiles[index].info.model.clone();
                    self.remember_selection();
                }
                self.library_open = true;
            }
            Action::OpenArtifact(path) => {
                self.status = match open_reference_artifact(&path) {
                    Ok(()) => String::new(),
                    Err(e) => format!("Cannot open reference artifact: {e}"),
                };
            }
        }
    }
    /// Submit a correlated service request; return its ID only when accepted.
    pub(super) fn request(&mut self, operation: Operation, hardware: bool) -> Option<u64> {
        if hardware {
            self.service
                .set_communication(self.preferences.communication);
        }
        self.next_id += 1;
        let expected_identity = matches!(
            operation,
            Operation::BridgeExport
                | Operation::BridgeNamedBackup { .. }
                | Operation::BridgeReadAll
                | Operation::BridgeProgram { .. }
                | Operation::BridgeLineSettings { .. }
        )
        .then(|| Identity {
            model: "ENL-MOD-32".into(),
            firmware: "3.6".into(),
        });
        if self
            .service
            .send(Command {
                request_id: self.next_id,
                port: hardware.then(|| self.selected.clone()),
                expected_identity,
                operation,
            })
            .is_err()
        {
            self.status = "Hardware service stopped. Restart the application.".into();
            return None;
        }
        Some(self.next_id)
    }
    /// Report manual work, allowing background polling to remain visually quiet.
    pub(super) fn foreground_busy(&self) -> bool {
        self.queued.is_some() || (self.active.is_some() && !self.auto_request)
    }
    /// Describe queued manual work in the status bar.
    pub(super) fn queued_label(operation: &Operation) -> &'static str {
        match operation {
            Operation::BridgeLineSettings { .. } => {
                "Line settings queued · waiting for the current read to finish"
            }
            Operation::BridgeProgram { .. } => {
                "Program Modbus Bridge queued · waiting for the current read to finish"
            }
            Operation::BridgeExport | Operation::BridgeNamedBackup { .. } => {
                "Backup queued · waiting for the current read to finish"
            }
            _ => "Action queued · waiting for the current read to finish",
        }
    }
    /// Start queued work only if the physical USB route still matches.
    pub(super) fn start_queued(&mut self) {
        if self.active.is_some() {
            return;
        }
        if let Some((operation, route)) = self.queued.take() {
            if self.programming_blocked
                || !self
                    .ports
                    .iter()
                    .any(|p| modbus_configurator::adapter::same_route(p, &route))
            {
                self.status =
                    "Queued action cancelled: the USB interface is unavailable or needs review."
                        .into();
                return;
            }
            self.selected = route.port;
            self.hardware(operation);
        }
    }
    /// Serialize manual hardware work behind any active poll.
    pub(super) fn hardware(&mut self, operation: Operation) {
        self.hardware_with_activity(operation, false);
    }
    /// Start automatic reads quietly while recording explicit manual operations.
    pub(super) fn hardware_with_activity(&mut self, operation: Operation, automatic: bool) {
        if !automatic {
            self.record_activity(format!(
                "{} | {} | {}",
                if self.active.is_some() {
                    "Requested while busy"
                } else {
                    "Starting"
                },
                self.selected,
                Self::operation_description(&operation)
            ));
        }
        if self.active.is_some() {
            if self.auto_request
                && self.queued.is_none()
                && let Some(route) = self.ports.iter().find(|p| p.port == self.selected).cloned()
            {
                self.queued = Some((operation, route));
            }
            return;
        }
        self.auto_request = false;
        self.line_applying = matches!(operation, Operation::BridgeLineSettings { .. });
        if matches!(
            operation,
            Operation::BridgeProgram { .. } | Operation::BridgeLineSettings { .. }
        ) {
            self.programming = true;
            self.programming_blocked = true;
        }
        if matches!(operation, Operation::ClosePort) {
            self.auto_paused = true;
        }
        self.last_fetch = Instant::now();
        self.status = if self.programming {
            "Preparing Modbus Bridge programming…"
        } else {
            "Reading device…"
        }
        .into();
        self.active = self.request(operation, true);
    }
}
