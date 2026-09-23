//! Optional native rendering check with labelled synthetic devices and no hardware access.
use super::*;
use winit::platform::windows::EventLoopBuilderExtWindows;

struct Preview {
    app: Configurator,
    frames: usize,
    output: std::path::PathBuf,
}
impl eframe::App for Preview {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.app.auto_paused = true;
        self.app.last_scan = Instant::now();
        self.app.frame(ctx);
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let rgba: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|p| p.to_srgba_unmultiplied())
                    .collect();
                image::save_buffer(
                    &self.output,
                    &rgba,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.frames += 1;
        if self.frames == 8 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        ctx.request_repaint();
    }
}

/// Run explicitly on a Windows desktop; fixtures never enter production snapshots.
#[test]
#[ignore = "Native screenshot review; requires an interactive Windows desktop"]
fn capture_device_map() {
    let output = std::path::PathBuf::from(
        std::env::var("POLYGON_VISUAL_OUTPUT").expect("Set screenshot output path"),
    );
    let mut a = app();
    a.offline = true;
    a.auto_paused = true;
    a.preferences.dark_mode = Some(true);
    a.ports = vec![port()];
    a.selected = port().port;
    a.bridge_source = Some(port());
    let mut result = read(&a, 21.5);
    let devices: Vec<_> = (1..=2)
        .map(|slave| {
            let mut device = modbus_configurator::network::Device::new(0, slave, &a.profiles);
            device.points = device.points.into_iter().take(3).collect();
            device
        })
        .collect();
    result.native_tsv = modbus_configurator::network::compose(&devices, &a.profiles).unwrap();
    result.dev_eui = Some("000000000000CAFE".into());
    result.readings = (1..=3).map(|item| Reading { item, value: 21.5 }).collect();
    result.exceptions = (4..=6)
        .map(|item| PointException {
            item,
            code: 11,
            message: "No response".into(),
        })
        .collect();
    result.successful_reads = Some(3);
    a.result = Some(result);
    a.technician.slave_failures.insert(2, 3);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Visual test — synthetic readings, no hardware")
            .with_inner_size([1180.0, 760.0])
            .with_visible(false),
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_any_thread(true);
        })),
        ..Default::default()
    };
    eframe::run_native(
        "Device map visual test",
        options,
        Box::new(move |cc| {
            crate::brand::apply_theme(&cc.egui_ctx, true);
            crate::brand::typography(&cc.egui_ctx);
            cc.egui_ctx.style_mut(|s| {
                s.spacing.item_spacing = egui::vec2(10.0, 10.0);
                s.spacing.button_padding = egui::vec2(12.0, 7.0);
            });
            Ok(Box::new(Preview {
                app: a,
                frames: 0,
                output,
            }))
        }),
    )
    .unwrap();
}
