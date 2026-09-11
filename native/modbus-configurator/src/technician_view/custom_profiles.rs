//! Persistent user profiles reuse display interpretations without programming hardware.
use super::*;
use modbus_configurator::custom_profile::{self, SavedProfile};
use network_readings::TypeChoice;

impl TechnicianView {
    /// Load local profiles once at startup, keeping corrupted libraries intact.
    pub fn load_custom_profiles(&mut self, root: &std::path::Path, profiles: &[Profile]) {
        match custom_profile::load(root, profiles) {
            Ok(saved) => {
                self.saved_profiles = saved;
                self.custom_profile_root = Some(root.to_owned());
            }
            Err(error) => {
                self.custom_profile_root = None;
                self.custom_profile_message = format!("Could not load custom profiles: {error}");
            }
        }
    }

    /// Save the selected type and register set, or apply an existing matching profile.
    pub(super) fn custom_profile_controls(
        &mut self,
        ui: &mut egui::Ui,
        table: &str,
        slave: u8,
        profiles: &[Profile],
        selected: Option<usize>,
    ) {
        crate::brand::collapsing(ui, "Custom profiles", |ui| {
            ui.weak("Saved on this computer and reused for matching register sets, including other slave addresses. This does not program the E5 bridge.");
            ui.horizontal_wrapped(|ui| {
                ui.label("Profile name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.custom_profile_name)
                        .hint_text("e.g. DPT146 temperature only")
                        .char_limit(64)
                        .desired_width(270.0),
                );
                if ui
                    .add_enabled(
                        selected.is_some() && self.custom_profile_root.is_some(),
                        egui::Button::new("Save custom profile"),
                    )
                    .clicked()
                {
                    let selected = &profiles[selected.unwrap()];
                    let result = SavedProfile::new(
                        &self.custom_profile_name,
                        &selected.info.id,
                        table,
                        slave,
                    )
                    .and_then(|profile| {
                        if self
                            .saved_profiles
                            .iter()
                            .any(|p| p.name.eq_ignore_ascii_case(&profile.name))
                        {
                            return Err("That name is already saved. Choose another name.".into());
                        }
                        let mut saved = self.saved_profiles.clone();
                        saved.push(profile);
                        custom_profile::save(
                            self.custom_profile_root.as_ref().unwrap(),
                            &saved,
                            profiles,
                        )
                        .map_err(|e| e.to_string())?;
                        self.saved_profiles = saved;
                        Ok(())
                    });
                    self.custom_profile_message = match result {
                        Ok(()) => format!("Saved profile: {}", self.custom_profile_name.trim()),
                        Err(error) => format!("Profile was not saved: {error}"),
                    };
                }
                ui.menu_button("Load custom profile", |ui| {
                    if self.saved_profiles.is_empty() {
                        ui.weak("No custom profiles saved.");
                    }
                    for saved in &self.saved_profiles {
                        let matching = saved.matches(table, slave);
                        if ui
                            .add_enabled(matching, egui::Button::new(&saved.name))
                            .on_hover_text(if matching {
                                "Apply this saved device type"
                            } else {
                                "This profile uses a different register set"
                            })
                            .clicked()
                        {
                            if let Some(index) =
                                profiles.iter().position(|p| p.info.id == saved.model_id)
                            {
                                self.network_types
                                    .insert((table.to_owned(), slave), TypeChoice::Profile(index));
                                self.custom_profile_message =
                                    format!("Using profile: {}", saved.name);
                            }
                            ui.close();
                        }
                    }
                });
            });
            if selected.is_none() {
                ui.weak("Select a device type before saving a custom profile.");
            }
            if !self.custom_profile_message.is_empty() {
                ui.label(&self.custom_profile_message);
            }
        });
    }
}
