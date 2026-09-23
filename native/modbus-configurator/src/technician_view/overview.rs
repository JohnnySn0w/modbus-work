//! Device overview: derive connection entries and render their status cards.
use super::*;

impl TechnicianView {
    /// Render connection and sensor groups from the current snapshots.
    pub(super) fn overview(
        &mut self,
        ui: &mut egui::Ui,
        context: DevicePageContext<'_>,
        result_port: &str,
    ) -> Vec<Action> {
        let DevicePageContext {
            reference,
            ports,
            profiles,
            result,
            direct,
            ..
        } = context;
        let mut actions = Vec::new();
        let network = result.is_some();
        ui.horizontal(|ui| {
            ui.heading("Devices");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear errors").on_hover_text("Acknowledge displayed errors. Logs and safety blocks remain; new errors appear again.").clicked() {
                    actions.push(Action::ClearErrors);
                }
                if ui.button("Refresh").clicked() {
                    actions.push(Action::Refresh);
                }
            });
        });
        ui.add_space(25.0);
        let nodes = self.overview_nodes(&context, result_port, network);
        if nodes.is_empty() {
            ui.group(|ui| {
                ui.heading("No devices connected");
                ui.label("Connect a Modbus Bridge or USB-COMi-TB to get started.");
            });
        }
        // Keep transports beside their sensors; narrow windows can scroll horizontally.
        let map_width = ui.available_width().max(650.0);
        egui::ScrollArea::horizontal()
            .id_salt("device-map")
            .show(ui, |ui| {
                ui.set_min_width(map_width);
                let mut divider_x = None;
                let map = ui.horizontal_top(|ui| {
                    for (interfaces, heading) in [(true, "Connections"), (false, "Sensors")] {
                        let in_group = |key: &str| {
                            matches!(key, "bridge" | "adapter" | "synetica_usb") == interfaces
                        };
                        let column_width = if interfaces { 305.0 } else { map_width - 330.0 };
                        ui.allocate_ui_with_layout(
                            egui::vec2(column_width, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(column_width);
                                ui.strong(heading);
                                ui.add_space(8.0);
                                ui.horizontal_wrapped(|ui| {
                                    for (key, connection) in
                                        nodes.iter().filter(|(key, _)| in_group(key))
                                    {
                                        if let Some(device) = reference.devices.get(*key) {
                                            let response = ui.group(|ui| {
                                                ui.with_layout(
                                                    egui::Layout::top_down(egui::Align::Min),
                                                    |ui| {
                                                        ui.set_width(285.0);
                                                        ui.set_min_height(210.0);
                                                        let view_device = ui
                                                            .horizontal(|ui| {
                                                                if *key == "bridge" {
                                                                    let (label, warning) = if self
                                                                        .bridge_busy
                                                                    {
                                                                        (
                                                                            if self.bridge_polling {
                                                                                "Polling"
                                                                            } else {
                                                                                "Communicating"
                                                                            },
                                                                            false,
                                                                        )
                                                                    } else if self.bridge_fault {
                                                                        ("Not responding", true)
                                                                    } else if self.bridge_connected
                                                                    {
                                                                        ("Ready", false)
                                                                    } else {
                                                                        ("Disconnected", true)
                                                                    };
                                                                    crate::brand::badge(
                                                                        ui, label, warning,
                                                                    );
                                                                } else if !interfaces {
                                                                    let slave = direct
                                                                        .filter(|d| d.key == *key)
                                                                        .map(|d| d.settings.slave)
                                                                        .or_else(|| {
                                                                            result?
                                                                                .native_tsv
                                                                                .lines()
                                                                                .nth(1)?
                                                                                .split('\t')
                                                                                .nth(1)?
                                                                                .parse::<u8>()
                                                                                .ok()
                                                                        });
                                                                    if let Some(slave) = slave {
                                                                        self.slave_card_badge(
                                                                            ui,
                                                                            result.filter(|_| {
                                                                                direct.is_none_or(
                                                                                    |d| {
                                                                                        d.key
                                                                                            != *key
                                                                                    },
                                                                                )
                                                                            }),
                                                                            slave,
                                                                        );
                                                                    }
                                                                }
                                                                ui.with_layout(
                                                                    egui::Layout::right_to_left(
                                                                        egui::Align::Center,
                                                                    ),
                                                                    |ui| ui.button("View device"),
                                                                )
                                                                .inner
                                                            })
                                                            .inner;
                                                        if *key == "bridge" {
                                                            crate::brand::bridge_eui(
                                                                ui,
                                                                result
                                                                    .filter(|_| {
                                                                        self.bridge_connected
                                                                    })
                                                                    .and_then(|r| {
                                                                        r.dev_eui.as_deref()
                                                                    }),
                                                            );
                                                        }
                                                        Self::icon(ui, device);
                                                        ui.label(
                                                            RichText::new(&device.name)
                                                                .strong()
                                                                .size(17.0),
                                                        );
                                                        if matches!(
                                                            *key,
                                                            "dpt146"
                                                                | "hmd65"
                                                                | "wattnode"
                                                                | "ati-f12"
                                                        ) && direct.is_none_or(|d| d.key != *key)
                                                        {
                                                            ui.add(
                                        egui::Label::new(
                                            "Configured model · sensor identity unverified",
                                        )
                                        .wrap(),
                                    );
                                                        }
                                                        if matches!(
                                                            *key,
                                                            "dpt146"
                                                                | "hmd65"
                                                                | "wattnode"
                                                                | "ati-f12"
                                                        ) && direct.is_none_or(|d| d.key != *key)
                                                        {
                                                            let profile =
                                                                profiles.iter().find(|p| {
                                                                    reference::profile_matches(
                                                                        key, &p.info.id,
                                                                    )
                                                                });
                                                            if profile.is_none_or(|p| {
                                                                result.is_some_and(|r| {
                                                                    p.contains_points(r)
                                                                })
                                                            }) && let Some(notice) =
                                                                &self.configuration_change
                                                            {
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        RichText::new(notice)
                                                                            .color(
                                                                            crate::brand::ORANGE,
                                                                        ),
                                                                    )
                                                                    .wrap(),
                                                                );
                                                            }
                                                            if let Some(warning) =
                                                                configuration_warning(
                                                                    profile,
                                                                    result,
                                                                    self.bridge_stale,
                                                                )
                                                                .filter(|_| {
                                                                    !self.errors_acknowledged
                                                                })
                                                            {
                                                                ui.add(
                                                                    egui::Label::new(
                                                                        RichText::new(warning)
                                                                            .color(
                                                                            crate::brand::ORANGE,
                                                                        ),
                                                                    )
                                                                    .wrap(),
                                                                );
                                                            }
                                                        }
                                                        if *key == "synetica_usb" {
                                                            ui.weak("Not identified");
                                                        }
                                                        ui.add_space(12.0);
                                                        if *key == "adapter"
                                                            && ports.iter().any(is_synetica)
                                                        {
                                                            ui.colored_label(
                                        crate::brand::ORANGE,
                                        "Blocked · another Modbus master may be active",
                                    );
                                                            ui.weak(connection);
                                                        } else {
                                                            ui.label(connection);
                                                        }
                                                        if matches!(
                                                            *key,
                                                            "dpt146"
                                                                | "hmd65"
                                                                | "wattnode"
                                                                | "ati-f12"
                                                                | "bridge"
                                                        ) && self.bridge_stale
                                                        {
                                                            ui.weak("Last good readings - stale");
                                                        }
                                                        let profile = profiles.iter().find(|p| {
                                                            reference::profile_matches(
                                                                key, &p.info.id,
                                                            )
                                                        });
                                                        if let Some(registers) =
                                                            reference.registers.get(*key)
                                                        {
                                                            for register in registers
                                                                .iter()
                                                                .filter(|r| {
                                                                    r.readout_label.is_some()
                                                                })
                                                                .take(3)
                                                            {
                                                                let live = direct
                                                                    .filter(|d| d.key == *key)
                                                                    .and_then(|d| {
                                                                        d.values
                                                                            .get(
                                                                                &register
                                                                                    .first_pdu()?,
                                                                            )
                                                                            .copied()
                                                                    })
                                                                    .or_else(|| {
                                                                        value(
                                                                            profile, result,
                                                                            register,
                                                                        )
                                                                    });
                                                                if let Some(v) = live {
                                                                    ui.label(format!(
                                                                        "{}: {v:.2} {}",
                                                                        register
                                                                            .readout_label
                                                                            .as_deref()
                                                                            .unwrap_or(
                                                                                &register.name
                                                                            ),
                                                                        register.unit
                                                                    ));
                                                                }
                                                            }
                                                        }
                                                        view_device
                                                    },
                                                )
                                                .inner
                                            });
                                            if response.inner.clicked() {
                                                self.reference_context = false;
                                                self.page = Page::Detail((*key).into());
                                            }
                                        }
                                    }
                                });
                                if !interfaces
                                    && network
                                    && let Some(result) = result
                                {
                                    self.network_readings(ui, result, profiles);
                                }
                            },
                        );
                        if interfaces {
                            divider_x = Some(ui.cursor().left() + 4.0);
                            ui.add_space(17.0);
                        }
                    }
                });
                if let Some(x) = divider_x {
                    ui.painter().line_segment(
                        [
                            egui::pos2(x, map.response.rect.top()),
                            egui::pos2(x, map.response.rect.bottom()),
                        ],
                        ui.visuals().widgets.noninteractive.bg_stroke,
                    );
                }
            });
        actions
    }

    /// Build map entries while preserving disconnected devices with retained readings.
    fn overview_nodes<'a>(
        &self,
        context: &DevicePageContext<'a>,
        result_port: &str,
        network: bool,
    ) -> Vec<(&'a str, String)> {
        let DevicePageContext {
            ports,
            profiles,
            result,
            direct,
            ..
        } = *context;
        let mut nodes: Vec<(&str, String)> = vec![];
        let bridge = ports
            .iter()
            .find(|p| is_synetica(p) && p.port == result_port)
            .or_else(|| ports.iter().find(|p| is_synetica(p)));
        if let Some(port) = bridge {
            let known = self.bridge_connected
                && port.port == result_port
                && result.is_some_and(|r| {
                    r.identity.model == "ENL-MOD-32" && r.identity.firmware == "3.6"
                });
            nodes.push((
                "bridge",
                if known {
                    port.port.clone()
                } else {
                    format!("{} · awaiting identification", port.port)
                },
            ));
            if known && !network {
                for key in ["dpt146", "hmd65", "wattnode", "ati-f12"] {
                    if profiles.iter().any(|p| {
                        reference::profile_matches(key, &p.info.id)
                            && result.is_some_and(|r| p.contains_points(r))
                    }) {
                        nodes.push((key, "Configured on Modbus Bridge".into()));
                    }
                }
            }
        }
        if !self.bridge_connected && result.is_some() {
            nodes.push(("bridge", format!("{result_port} - disconnected")));
            for key in ["dpt146", "hmd65", "wattnode", "ati-f12"]
                .into_iter()
                .filter(|_| !network)
            {
                if profiles.iter().any(|p| {
                    reference::profile_matches(key, &p.info.id)
                        && result.is_some_and(|r| p.contains_points(r))
                }) {
                    nodes.push((key, "Disconnected - last good readings".into()));
                }
            }
        }
        for port in ports.iter().filter(|p| is_adapter(p)) {
            nodes.push((
                "adapter",
                if bridge.is_some() {
                    format!("{} · paused while Modbus Bridge is connected", port.port)
                } else {
                    format!("{} · USB adapter available", port.port)
                },
            ));
        }
        if let Some(direct) = direct {
            nodes.push((
                &direct.key,
                format!(
                    "{} - {}",
                    direct.port.port,
                    if ports
                        .iter()
                        .any(|p| modbus_configurator::adapter::same_route(p, &direct.port))
                    {
                        direct.settings.label()
                    } else {
                        "Disconnected - last good readings".into()
                    }
                ),
            ));
        }
        nodes
    }
}
