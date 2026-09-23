//! Shared offline GUI fixtures and interaction helpers.
use super::*;
#[path = "device_feedback.rs"]
mod device_feedback;
#[path = "line_ui.rs"]
mod line_ui;
#[path = "network_ui.rs"]
mod network_ui;
use modbus_configurator::{
    bridge::{PointException, Reading},
    service::Backend,
    transport::Transport,
};
struct Offline;
impl Backend for Offline {
    fn inventory(&self) -> Result<Vec<PortInfo>, String> {
        Ok(vec![])
    }
    fn open(&self, _: &str) -> std::io::Result<Box<dyn Transport>> {
        panic!("GUI event tests must not open hardware")
    }
}
pub(crate) fn app() -> Configurator {
    Configurator::new(
        Service::with_backend(std::sync::Arc::new(Offline), Default::default()).unwrap(),
        modbus_configurator::reference::Reference::bundled().unwrap(),
        modbus_configurator::catalog::bundled().unwrap(),
    )
}
fn port() -> PortInfo {
    PortInfo {
        port: "COM41".into(),
        usb_vid: Some(0x0483),
        usb_pid: Some(0x5740),
        serial_number: Some("test-unit".into()),
        description: "USB console".into(),
        identity: None,
        busy: false,
    }
}
pub(crate) fn read(app: &Configurator, value: f64) -> BridgeResult {
    BridgeResult {
        dev_eui: None,
        identity: Identity {
            model: "ENL-MOD-32".into(),
            firmware: "3.6".into(),
        },
        native_tsv: app.profiles[0].native_tsv.clone(),
        readings: vec![Reading { item: 1, value }],
        successful_reads: Some(1),
        exceptions: vec![],
    }
}
fn start(a: &mut Configurator) {
    a.active = Some(42);
    a.selected = port().port;
    a.ports = vec![port()];
}
fn send(a: &mut Configurator, kind: EventKind) {
    a.handle_event(Event {
        request_id: 42,
        kind,
    });
}

fn adapter_result(slave: u8) -> modbus_configurator::adapter::AdapterResult {
    let mut p = port();
    p.usb_vid = Some(0x0403);
    p.usb_pid = Some(0x6001);
    modbus_configurator::adapter::AdapterResult {
        port: p,
        key: "dpt146".into(),
        settings: modbus_configurator::adapter::Settings {
            slave,
            even: false,
            two_stops: true,
        },
        family_only: false,
        values: std::collections::BTreeMap::from([(4, 21.5)]),
        errors: Default::default(),
    }
}

fn painted_text(shape: &egui::epaint::Shape, text: &mut String) {
    match shape {
        egui::epaint::Shape::Text(t) => {
            text.push_str(&t.galley.job.text);
            text.push('\n');
        }
        egui::epaint::Shape::Vec(v) => {
            for s in v {
                painted_text(s, text)
            }
        }
        _ => {}
    }
}
pub(crate) fn draw(a: &mut Configurator, ctx: &egui::Context) -> String {
    a.auto_paused = true;
    a.last_scan = Instant::now();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 2400.0),
            )),
            ..Default::default()
        },
        |ctx| a.frame(ctx),
    );
    let mut text = String::new();
    for shape in output.shapes {
        painted_text(&shape.shape, &mut text);
    }
    text
}

pub(crate) fn click(a: &mut Configurator, ctx: &egui::Context, label: &str) -> egui::FullOutput {
    fn locate(shapes: &[egui::epaint::ClippedShape], label: &str) -> Option<egui::Pos2> {
        shapes.iter().find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                Some(t.pos + t.galley.size() / 2.0)
            }
            _ => None,
        })
    }
    a.auto_paused = true;
    a.last_scan = Instant::now();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 2400.0),
            )),
            ..Default::default()
        },
        |ctx| a.frame(ctx),
    );
    let pos = locate(&output.shapes, label).unwrap_or_else(|| panic!("Missing {label}"));
    let mut output = output;
    for pressed in [true, false] {
        a.last_scan = Instant::now();
        output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 2400.0),
                )),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
            |ctx| a.frame(ctx),
        );
    }
    output
}

#[path = "gui_state.rs"]
mod state_tests;

#[path = "gui_configuration.rs"]
mod configuration_tests;

#[path = "gui_presentation.rs"]
mod presentation_tests;

#[path = "gui_diagnostics.rs"]
mod diagnostics_tests;

#[cfg(windows)]
#[path = "device_map_visual.rs"]
mod device_map_visual;
