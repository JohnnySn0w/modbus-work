//! Explicit screenshot export of real rendered views. Never creates device data.
use crate::technician_view::Page;
use eframe::egui;
use std::{collections::VecDeque, path::PathBuf};
pub struct Capture {
    directory: PathBuf,
    queue: VecDeque<(Page, Option<usize>, String)>,
    pending: Option<String>,
    frames: usize,
    pub started: bool,
    pub done: bool,
    pub reference_context: bool,
    files: Vec<String>,
}
impl Capture {
    pub fn from_args(
        reference: &modbus_configurator::reference::Reference,
        profiles: &[modbus_configurator::catalog::Profile],
    ) -> Option<Self> {
        let args: Vec<_> = std::env::args_os().collect();
        let index = args.iter().position(|s| s == "--capture-views")?;
        let directory = PathBuf::from(args.get(index + 1)?);
        Some(Self::new(directory, reference, profiles))
    }
    pub fn new(
        directory: PathBuf,
        reference: &modbus_configurator::reference::Reference,
        profiles: &[modbus_configurator::catalog::Profile],
    ) -> Self {
        std::fs::create_dir(&directory).expect("Screenshot output must be a new directory");
        let mut queue = VecDeque::from([(Page::Overview, None, "01-devices.png".into())]);
        queue.push_back((
            Page::Troubleshooting(None),
            None,
            "troubleshooting.png".into(),
        ));
        queue.push_back((Page::Settings, None, "settings.png".into()));
        queue.push_back((Page::History, None, "history.png".into()));
        queue.push_back((Page::References, None, "02-references.png".into()));
        for key in reference
            .devices
            .keys()
            .filter(|key| key.as_str() != "synetica_usb")
        {
            queue.push_back((Page::Detail(key.clone()), None, format!("device-{key}.png")));
            queue.push_back((Page::Detail(key.clone()), None, format!("model-{key}.png")));
        }
        for key in reference.registers.keys() {
            queue.push_back((
                Page::Registers(key.clone()),
                None,
                format!("model-registers-{key}.png"),
            ));
            queue.push_back((
                Page::Registers(key.clone()),
                None,
                format!("registers-{key}.png"),
            ));
        }
        queue.push_back((Page::Radio, None, "radio.png".into()));
        queue.push_back((Page::Console, None, "diagnostics.png".into()));
        for (index, p) in profiles.iter().enumerate() {
            queue.push_back((
                Page::Configurations,
                Some(index),
                format!("configuration-{}.png", p.info.id),
            ));
        }
        Self {
            directory,
            queue,
            pending: None,
            frames: 0,
            started: false,
            done: false,
            reference_context: false,
            files: vec![],
        }
    }
    pub fn update(&mut self, ctx: &egui::Context, ready: bool) -> Option<(Page, Option<usize>)> {
        if self.done || (!self.started && !ready) {
            return None;
        }
        self.started = true;
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event
                && let Some(name) = self.pending.take()
            {
                let rgba: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|p| p.to_srgba_unmultiplied())
                    .collect();
                image::save_buffer(
                    self.directory.join(&name),
                    &rgba,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .expect("Save screenshot");
                self.files.push(name);
                self.frames = 0;
            }
        }
        if self.frames == 0 {
            if let Some((page, profile, name)) = self.queue.pop_front() {
                self.reference_context = name.starts_with("model-") || page == Page::References;
                self.pending = Some(name);
                self.frames = 1;
                return Some((page, profile));
            }
            self.done = true;
            std::fs::write(
                self.directory.join("complete.json"),
                serde_json::to_vec_pretty(&self.files).unwrap(),
            )
            .unwrap();
            return Some((Page::Overview, None));
        }
        if self.frames == 4 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        self.frames += 1;
        None
    }
}

#[cfg(test)]
#[path = "tests/capture.rs"]
mod tests;
