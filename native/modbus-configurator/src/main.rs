#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Native Windows entry point and shared application state.
use eframe::egui;
mod activity;
mod app_commands;
mod app_configuration;
mod app_diagnostics;
mod app_events;
mod app_frame;
mod app_line_settings;
mod app_network;
mod app_settings;
mod diagnostic_checks;
mod diagnostic_log;
mod reference_files;
#[cfg(test)]
use reference_files::extract_reference_artifact;
use reference_files::open_reference_artifact;
mod brand;
mod capture_views;
mod catalog_view;
mod device_art;
mod history_view;
mod technician_view;
mod troubleshooting;
mod units;
use modbus_configurator::{bridge::BridgeResult, contract::*, service::Service};
use std::time::{Duration, Instant};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Preferences {
    communication: modbus_configurator::bridge::CommunicationSettings,
    dark_mode: Option<bool>,
    units: units::Preset,
    automatic_polling: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            communication: Default::default(),
            dark_mode: None,
            units: units::Preset::System,
            automatic_polling: true,
        }
    }
}
struct Configurator {
    communication_log: Vec<String>,
    diagnostic_report: Option<(String, String, Vec<diagnostic_checks::Finding>)>,
    multi_device: bool,
    network_devices: Vec<modbus_configurator::network::Device>,
    network_unrecognized: bool,
    single_selection: Option<(Option<String>, String, Option<usize>)>,
    line_settings: Option<modbus_configurator::bridge::LineSettings>,
    line_draft: Option<modbus_configurator::bridge::LineSettings>,
    line_applying: bool,
    preferences: Preferences,
    settings_message: String,
    system_region: Option<String>,
    service: Service,
    offline: bool,
    storage_root: Result<std::path::PathBuf, String>,
    history: modbus_configurator::history::History,
    history_view: history_view::HistoryView,
    capture: Option<capture_views::Capture>,
    capture_restore: Option<(Option<String>, String, Option<usize>, bool)>,
    ports: Vec<PortInfo>,
    next_id: u64,
    last_scan: Instant,
    scan_pending: Option<u64>,
    active: Option<u64>,
    selected: String,
    preferred_route: Option<PortInfo>,
    status: String,
    replay_log: Vec<String>,
    result: Option<BridgeResult>,
    loaded_config: Option<String>,
    backup_path: Option<String>,
    backup_name: String,
    file_message: String,
    backup_error: String,
    adapter: Option<modbus_configurator::adapter::AdapterResult>,
    programming: bool,
    programming_blocked: bool,
    config_source: String,
    profiles: Vec<modbus_configurator::catalog::Profile>,
    catalog_view: catalog_view::CatalogView,
    library_open: bool,
    reference: modbus_configurator::reference::Reference,
    technician: technician_view::TechnicianView,
    last_fetch: Instant,
    fetched_at: Option<String>,
    bridge_source: Option<PortInfo>,
    auto_paused: bool,
    auto_request: bool,
    queued: Option<(Operation, PortInfo)>,
}
impl Configurator {
    /// Create application state without opening a serial interface.
    fn new(
        service: Service,
        reference: modbus_configurator::reference::Reference,
        profiles: Vec<modbus_configurator::catalog::Profile>,
    ) -> Self {
        Configurator {
            communication_log: Vec::new(),
            diagnostic_report: None,
            multi_device: false,
            network_devices: Vec::new(),
            network_unrecognized: false,
            single_selection: None,
            line_settings: None,
            line_draft: None,
            line_applying: false,
            preferences: Preferences::default(),
            settings_message: String::new(),
            system_region: None,
            history: Default::default(),
            history_view: Default::default(),
            capture: None,
            capture_restore: None,
            service,
            offline: false,
            storage_root: modbus_configurator::config_file::directory().map_err(|e| e.to_string()),
            ports: vec![],
            next_id: 0,
            last_scan: Instant::now() - Duration::from_secs(2),
            scan_pending: None,
            active: None,
            selected: String::new(),
            preferred_route: None,
            status: String::new(),
            replay_log: vec![],
            result: None,
            loaded_config: None,
            backup_path: None,
            backup_name: String::new(),
            file_message: String::new(),
            backup_error: String::new(),
            adapter: None,
            programming: false,
            programming_blocked: false,
            config_source: "Remembered configuration selection".into(),
            profiles,
            catalog_view: catalog_view::CatalogView::default(),
            library_open: false,
            reference,
            technician: technician_view::TechnicianView::default(),
            last_fetch: Instant::now() - Duration::from_secs(5),
            fetched_at: None,
            bridge_source: None,
            auto_paused: false,
            auto_request: false,
            queued: None,
        }
    }

    // Inventory can invalidate an operation before its terminal event arrives.
    // Conservatively block interrupted programming here; a late event may be ignored.
    /// Keep a bounded history of manual activity for diagnostics.
    fn record_activity(&mut self, message: String) {
        diagnostic_log::write(&message.replace(['\r', '\n'], " "));
        if self.replay_log.len() == 64 {
            self.replay_log.remove(0);
        }
        self.replay_log.push(format!(
            "{} | {}",
            modbus_configurator::last_good::timestamp(),
            message.replace(['\r', '\n'], " ")
        ));
    }
}

/// Parse a trailing completed/total count, rejecting invalid progress.
fn transfer_progress(stage: &str) -> Option<f32> {
    let counts = stage.rsplit_once('(')?.1.strip_suffix(')')?;
    let (done, total) = counts.split_once('/')?;
    let (done, total) = (done.parse::<u32>().ok()?, total.parse::<u32>().ok()?);
    (total > 0 && done <= total).then(|| done as f32 / total as f32)
}
/// Initialize reference data and the selected backend, then run the native window.
fn main() -> eframe::Result {
    diagnostic_log::initialize();
    let reference = modbus_configurator::reference::Reference::bundled()
        .map_err(|e| eframe::Error::AppCreation(Box::new(std::io::Error::other(e))))?;
    let profiles = modbus_configurator::catalog::bundled()
        .map_err(|e| eframe::Error::AppCreation(Box::new(std::io::Error::other(e))))?;
    let offline = std::env::args_os().any(|arg| arg == "--offline");
    let service = if offline {
        Service::offline()
    } else {
        Service::start()
    }
    .map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "Polygon Device Configurator — {}",
                option_env!("POLYGON_BUILD_ID")
                    .and_then(|build| build.split('|').next())
                    .map(str::trim)
                    .unwrap_or(concat!("v", env!("CARGO_PKG_VERSION"), " (development)"))
            ))
            .with_icon(brand::icon())
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Polygon Device Configurator",
        options,
        Box::new(move |cc| {
            brand::apply(&cc.egui_ctx);
            cc.egui_ctx.style_mut(|style| {
                style.spacing.item_spacing = egui::vec2(10.0, 10.0);
                style.spacing.button_padding = egui::vec2(12.0, 7.0);
                style
                    .text_styles
                    .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
                style
                    .text_styles
                    .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
                style
                    .text_styles
                    .insert(egui::TextStyle::Heading, egui::FontId::proportional(24.0));
            });
            brand::typography(&cc.egui_ctx);
            let mut app = Configurator::new(service, reference, profiles);
            app.offline = offline;
            if !offline
                && let Err(error) =
                    modbus_configurator::config_file::migrate_previous_installation()
            {
                app.file_message =
                    format!("Could not import settings from the previous installation: {error}");
            }
            if !offline {
                app.load_preferences();
                if let Ok(root) = &app.storage_root {
                    app.technician.load_custom_profiles(root, &app.profiles);
                }
            }
            app.system_region = units::system_region();
            app.technician.unit_preset =
                app.preferences.units.resolve(app.system_region.as_deref());
            if std::env::args().any(|arg| arg == "--dark") {
                app.preferences.dark_mode = Some(true);
            } else if std::env::args().any(|arg| arg == "--light") {
                app.preferences.dark_mode = Some(false);
            }
            app.apply_theme(&cc.egui_ctx);
            app.capture = capture_views::Capture::from_args(&app.reference, &app.profiles);
            app.loaded_config = modbus_configurator::config_file::directory()
                .ok()
                .and_then(|p| modbus_configurator::config_file::load(&p.join("Selected.tsv")).ok());
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
#[path = "tests/gui_events.rs"]
mod gui_event_tests;

impl eframe::App for Configurator {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.frame(ctx);
    }
}
