//! Theme, display units, and polling preferences.
use super::*;

impl Configurator {
    /// Load saved preferences, retaining defaults when storage is unavailable or invalid.
    pub(super) fn load_preferences(&mut self) {
        let result = self
            .storage_root
            .clone()
            .map_err(std::io::Error::other)
            .and_then(|folder| match std::fs::read(folder.join("Settings.json")) {
                Ok(bytes) => serde_json::from_slice(&bytes).map_err(std::io::Error::other),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
                Err(e) => Err(e),
            });
        match result {
            Ok(mut preferences) => {
                preferences.communication = preferences.communication.bounded();
                self.preferences = preferences;
            }
            Err(e) => {
                self.preferences.automatic_polling = false;
                self.settings_message = format!(
                    "Could not load settings. Automatic polling is off until enabled here. {e}"
                );
            }
        }
    }
    /// Apply the selected theme, or follow the current system theme.
    pub(super) fn apply_theme(&self, ctx: &egui::Context) {
        let dark = self
            .preferences
            .dark_mode
            .unwrap_or_else(|| ctx.system_theme() == Some(egui::Theme::Dark));
        brand::apply_theme(ctx, dark);
    }
    /// Render and save preferences without interrupting in-flight hardware work.
    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.add_space(16.0);
        ui.strong("Appearance");
        let theme_changed = ui
            .horizontal(|ui| {
                let system = ui
                    .radio_value(&mut self.preferences.dark_mode, None, "System")
                    .changed();
                let light = ui
                    .radio_value(&mut self.preferences.dark_mode, Some(false), "Light")
                    .changed();
                let dark = ui
                    .radio_value(&mut self.preferences.dark_mode, Some(true), "Dark")
                    .changed();
                system || light || dark
            })
            .inner;
        ui.weak("System follows Windows and updates when its theme changes.");

        ui.add_space(24.0);
        ui.strong("Units");
        let units_changed = ui
            .horizontal(|ui| {
                let mut changed = false;
                for preset in [
                    units::Preset::System,
                    units::Preset::Us,
                    units::Preset::Uk,
                    units::Preset::Eu,
                ] {
                    changed |= ui
                        .radio_value(&mut self.preferences.units, preset, preset.label())
                        .changed();
                }
                changed
            })
            .inner;
        if units_changed {
            self.system_region = units::system_region();
        }
        let resolved = self
            .preferences
            .units
            .resolve(self.system_region.as_deref());
        self.technician.unit_preset = resolved;
        ui.label(format!(
            "Using {} units · Windows region: {}",
            resolved.label(),
            self.system_region
                .as_deref()
                .unwrap_or("unavailable; metric fallback")
        ));
        ui.weak(match resolved {
            units::Preset::Us => "°F · psia · Btu/lb",
            units::Preset::Uk => "°C · bar(a) · kJ/kg",
            _ => "°C · kPa(a) · kJ/kg",
        });
        ui.add(egui::Label::new("Applies to device readings and register tables. Individual unit choices take priority. History, exports and Modbus Bridge configurations retain native units.").wrap());
        if ui.button("Reset individual unit choices").clicked() {
            self.technician.reset_unit_overrides();
        }
        ui.add_space(24.0);
        ui.strong("Device readings");
        let polling_changed = ui
            .checkbox(&mut self.preferences.automatic_polling, "Automatic polling")
            .changed();
        ui.add(egui::Label::new("When off, the current operation finishes safely and no further automatic reads start. Manual reads and queued actions remain available.").wrap());
        if !self.preferences.automatic_polling && self.active.is_some() && self.auto_request {
            ui.label("Finishing the current read…");
        }
        if polling_changed && self.preferences.automatic_polling {
            if !self.programming_blocked {
                self.auto_paused = false;
            }
            self.last_fetch = Instant::now() - Duration::from_secs(5);
        }
        if theme_changed {
            self.apply_theme(ui.ctx());
        }
        ui.add_space(24.0);
        ui.strong("Advanced communication settings");
        ui.label("Host-side limits in seconds. Changes apply to the next operation; an active operation keeps its current limits. These do not change Modbus Bridge sensor settings.");
        let mut communication_changed = ui
            .checkbox(
                &mut self.preferences.communication.automatic_scan_budget,
                "Allow extra scan time for sensor retries",
            )
            .changed();
        ui.weak("Automatic scan budgeting uses at least 15 seconds per register entry plus 30 seconds, or more when the bridge reports longer sensor retries. Disable it to test an exact Read All deadline. A failed scan pauses automatic polling; check the console in Configuration before retrying.");
        egui::Grid::new("communication-settings").show(ui, |ui| {
            for (label, value, range) in [
                (
                    "Initial console response",
                    &mut self.preferences.communication.initial_seconds,
                    1..=120,
                ),
                (
                    "Menu and command response",
                    &mut self.preferences.communication.command_seconds,
                    1..=120,
                ),
                (
                    "Read All response (each phase)",
                    &mut self.preferences.communication.read_all_seconds,
                    5..=900,
                ),
                (
                    "Automatic retry delay after failure",
                    &mut self.preferences.communication.retry_seconds,
                    5..=300,
                ),
            ] {
                ui.label(label);
                communication_changed |= ui
                    .add(egui::DragValue::new(value).range(range).suffix(" seconds"))
                    .changed();
                ui.end_row();
            }
        });
        ui.weak("A silent initial connection can use two waits: the initial response and one wake attempt. A longer Read All limit allows more time; it does not prove the Modbus Bridge is still scanning.");
        if ui.button("Reset communication defaults").clicked() {
            self.preferences.communication = Default::default();
            communication_changed = true;
        }
        if communication_changed {
            self.record_activity(format!(
                "Communication settings changed: {:?}; applies to next operation",
                self.preferences.communication
            ));
        }
        if theme_changed || polling_changed || units_changed || communication_changed {
            self.settings_message = match self
                .storage_root
                .clone()
                .map_err(std::io::Error::other)
                .and_then(|folder| {
                    std::fs::create_dir_all(&folder)?;
                    std::fs::write(
                        folder.join("Settings.json"),
                        serde_json::to_vec_pretty(&self.preferences)
                            .map_err(std::io::Error::other)?,
                    )
                }) {
                Ok(()) => "Settings saved on this computer.".into(),
                Err(e) => format!("Applied for this session, but settings could not be saved: {e}"),
            };
        }
        if !self.settings_message.is_empty() {
            ui.add_space(16.0);
            ui.label(&self.settings_message);
        }
    }
}
