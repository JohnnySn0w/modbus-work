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
            Ok(preferences) => self.preferences = preferences,
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
        ui.weak("Polygon charcoal and grey surfaces, cyan highlights and orange accents.");
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
        ui.add(egui::Label::new("Applies to device readings and register tables. Individual unit choices take priority. History, exports and E5 bridge configurations retain native units.").wrap());
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
        if theme_changed || polling_changed || units_changed {
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
