//! Dedicated help view; guidance does not initiate hardware operations.
use eframe::egui;
use modbus_configurator::reference::Reference;

pub fn show(ui: &mut egui::Ui, reference: &Reference, selected: &mut Option<String>) {
    ui.heading("Troubleshooting");
    ui.label("Choose the application or a device for setup and troubleshooting guidance.");
    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(selected, None, "Application");
        for (key, device) in reference
            .devices
            .iter()
            .filter(|(key, _)| key.as_str() != "synetica_usb")
        {
            ui.selectable_value(selected, Some(key.clone()), &device.name);
        }
    });
    ui.add_space(20.0);
    if let Some(device) = selected.as_ref().and_then(|key| reference.devices.get(key)) {
        ui.heading(&device.name);
        if device.help_setup == ["TBD"] && device.help_troubleshooting == ["TBD"] {
            ui.label("• TBD");
            return;
        }
        ui.add_space(12.0);
        ui.strong("Setup");
        for (index, line) in device.help_setup.iter().enumerate() {
            ui.add(egui::Label::new(format!("{}. {}", index + 1, guidance(line))).wrap());
        }
        ui.add_space(20.0);
        ui.strong("Troubleshooting steps");
        for line in &device.help_troubleshooting {
            ui.add(egui::Label::new(format!("• {}", guidance(line))).wrap());
        }
        if matches!(device.key.as_str(), "adapter" | "bridge" | "synetica_usb") {
            ui.add_space(16.0);
            ui.strong("E5 bridge hardware switch and adapter access");
            ui.add(egui::Label::new("The E5 bridge can be switched off with its hardware switch while external power remains connected. Turning automatic polling off in Settings does not establish that the E5 bridge has stopped driving RS-485. Use only one active master on the shared bus.").wrap());
        }
    } else {
        for (title, text) in [
            (
                "USB device is missing or unavailable",
                "Open Diagnostics to check the current USB interfaces. Check the USB cable and Windows Device Manager. Serial port numbers may change between computers or reconnects. Close other applications that may own the serial port, then refresh discovery from Devices.",
            ),
            (
                "Readings are not updating",
                "Check Automatic polling in Settings. When it is off, use Read now for a manual read. Disconnected or failed reads retain the last good values and their timestamp; those values are not current measurements. Check the device status and wiring before retrying.",
            ),
            (
                "Adapter is detected but blocked",
                "The adapter is visible even when another Modbus master may be active. Switch the E5 bridge off using its hardware switch before direct adapter reads. External power can remain connected; its USB interface goes away when switched off.",
            ),
            (
                "A command is queued or taking longer than expected",
                "Manual actions wait for the current automatic read to finish. The bottom status bar shows queued work and progress. Cancel queued removes only the waiting action. Avoid unplugging equipment during programming; a failed or interrupted write requires reviewing the recovered E5 bridge configuration before another write.",
            ),
            (
                "Configuration does not match the attached sensor",
                "The E5 bridge point table describes the requested registers. A selected device type does not verify the physical sensor model. Check the sensor model, slave address, baud rate, parity and wiring. In Configuration, select the matching profile or configuration file and review the differences before using Program E5 bridge.",
            ),
            (
                "Backup, load or programming failed",
                "Use Configuration to load or save a configuration file, retrieve a backup, or program the E5 bridge. Programming requires a successful backup and verifies the resulting point table. Check the reported error and folder permissions. After an uncertain write, inspect the recovered console and export before retrying; do not assume the change completed.",
            ),
            (
                "Theme, units or settings look wrong",
                "Settings offers System, Light and Dark themes and System, United States, United Kingdom and Europe unit presets. System units use the Windows region. Individual register unit choices take priority; Reset individual unit choices restores the preset. A settings-save error means changes apply only to this session.",
            ),
            (
                "Collect information for troubleshooting",
                "In Diagnostics, expand Activity log and use Copy log. Include the affected device, USB identity, action attempted and last good reading timestamp. Automatic polling is omitted from this log. Offline prompt replay checks bundled protocol scenarios; it does not test connected hardware.",
            ),
        ] {
            ui.strong(title);
            ui.add(egui::Label::new(text).wrap());
            ui.add_space(16.0);
        }
    }
}
fn guidance(line: &str) -> String {
    if line.contains("Power down or isolate the E5 bridge master") {
        return "Ensure the E5 bridge is not an active master before direct polling. See the hardware-switch guidance below.".into();
    }
    line.replace("COM3", "the USB adapter")
}
