use eframe::egui;
use modbus_configurator::history::History;
#[derive(Default)]
pub struct HistoryView {
    selected: Option<(String, String)>,
    message: String,
}
impl HistoryView {
    pub fn show(&mut self, ui: &mut egui::Ui, history: &History) {
        ui.heading("Reading history");
        ui.label(
            "This application session · up to 50,000 point samples · local receipt timestamps",
        );
        ui.weak(
            "Failed reads leave gaps. Export before closing the application to keep this history.",
        );
        if history.discarded > 0 {
            ui.label(format!(
                "{} oldest samples removed to limit memory use.",
                history.discarded
            ));
        }
        if ui
            .add_enabled(
                !history.samples.is_empty(),
                egui::Button::new("Export history CSV…"),
            )
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Reading history", &["csv"])
                .set_file_name("reading-history.csv")
                .save_file()
        {
            self.message = match history.export(&path) {
                Ok(()) => format!(
                    "Saved {} samples to {}",
                    history.samples.len(),
                    path.display()
                ),
                Err(e) => format!("History was not saved: {e}"),
            };
        }
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        let series: std::collections::BTreeSet<_> = history
            .samples
            .iter()
            .map(|s| (s.source.clone(), s.definition.clone()))
            .collect();
        if self
            .selected
            .as_ref()
            .is_none_or(|key| !series.contains(key))
        {
            self.selected = series.first().cloned();
        }
        egui::ComboBox::from_id_salt("history-series")
            .width(ui.available_width().min(850.0))
            .selected_text(self.selected.as_ref().map_or(
                "Waiting for live readings".into(),
                |(source, definition)| series_label(source, definition),
            ))
            .show_ui(ui, |ui| {
                for key in series {
                    let text = series_label(&key.0, &key.1);
                    ui.selectable_value(&mut self.selected, Some(key), text);
                }
            });
        let Some((source, definition)) = &self.selected else {
            return;
        };
        crate::brand::collapsing(ui, "Point definition", |ui| {
            ui.label(source);
            ui.label(definition);
        });
        let samples: Vec<_> = history
            .samples
            .iter()
            .filter(|s| &s.source == source && &s.definition == definition)
            .collect();
        let values: Vec<_> = samples.iter().filter_map(|s| s.value).collect();
        ui.label(format!(
            "{} successful samples · {} gaps",
            values.len(),
            samples.len() - values.len()
        ));
        if values.is_empty() {
            ui.label("No successful samples for this point yet.");
        } else {
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let (measurement, unit) = measurement(definition);
            ui.label(format!("{measurement} · Native units: {unit}"));
            ui.label(format!("Observed range: {min:.3} to {max:.3} {unit}"));
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 230.0),
                egui::Sense::hover(),
            );
            let plot = egui::Rect::from_min_max(
                rect.min + egui::vec2(100.0, 12.0),
                rect.max - egui::vec2(20.0, 35.0),
            );
            ui.painter()
                .rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
            let first = samples.first().unwrap().unix_ms;
            let last = samples.last().unwrap().unix_ms;
            let span = last.saturating_sub(first).max(1) as f64;
            let padding = if max == min {
                (min.abs() * 0.05).max(1.0)
            } else {
                (max - min) * 0.05
            };
            let min = min - padding;
            let max = max + padding;
            let spread = max - min;
            let painter = ui.painter();
            let color = ui.visuals().text_color();
            for step in 0..=4 {
                let fraction = step as f32 / 4.0;
                let y = plot.bottom() - fraction * plot.height();
                painter.line_segment(
                    [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                painter.text(
                    egui::pos2(plot.left() - 8.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.3}", min + f64::from(fraction) * spread),
                    egui::FontId::proportional(12.0),
                    color,
                );
                let x = plot.left() + fraction * plot.width();
                painter.text(
                    egui::pos2(x, plot.bottom() + 8.0),
                    egui::Align2::CENTER_TOP,
                    format!("{:.1}s", f64::from(fraction) * span / 1000.0),
                    egui::FontId::proportional(12.0),
                    color,
                );
            }
            ui.painter().text(
                egui::pos2(plot.center().x, rect.bottom() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                "Elapsed time from first sample",
                egui::FontId::proportional(11.0),
                color,
            );
            let mut previous: Option<(u64, egui::Pos2)> = None;
            for sample in &samples {
                if let Some(v) = sample.value {
                    let position = egui::pos2(
                        plot.left()
                            + (sample.unix_ms.saturating_sub(first) as f64 / span) as f32
                                * plot.width(),
                        plot.bottom() - ((v - min) / spread) as f32 * plot.height(),
                    );
                    if let Some((ms, old)) = previous
                        && sample.unix_ms >= ms
                        && sample.unix_ms - ms <= 30_000
                    {
                        ui.painter().line_segment(
                            [old, position],
                            egui::Stroke::new(1.5, crate::brand::CYAN),
                        );
                    }
                    ui.painter()
                        .circle_filled(position, 2.0, crate::brand::CYAN);
                    previous = Some((sample.unix_ms, position));
                } else {
                    previous = None;
                }
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(&samples.first().unwrap().timestamp);
                ui.label("to");
                ui.label(&samples.last().unwrap().timestamp);
            });
            ui.weak("Lines also break after more than 30 seconds without a sample. Point definitions identify native units and register settings; raw E5 bridge tables may not name a unit.");
        }
        ui.add_space(12.0);
        ui.strong("Latest samples");
        for sample in samples.iter().rev().take(8) {
            ui.label(format!(
                "{} · {} · {}",
                sample.timestamp,
                sample.value.map_or("—".into(), |v| v.to_string()),
                sample.status
            ));
        }
    }
}

#[cfg(test)]
#[path = "tests/history_view.rs"]
mod tests;

fn measurement(definition: &str) -> (String, String) {
    let first = definition.split(" · ").next().unwrap_or(definition);
    if let Some(address) = definition
        .rsplit("PDU ")
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        && let Ok(reference) = modbus_configurator::reference::Reference::bundled()
        && let Some(register) = reference
            .registers
            .get(first)
            .and_then(|rs| rs.iter().find(|r| r.first_pdu() == Some(address)))
    {
        return (
            format!("{} — {}", reference.devices[first].name, register.name),
            if register.unit.is_empty() {
                "unitless".into()
            } else {
                register.unit.clone()
            },
        );
    }
    let unit = first
        .rsplit_once('(')
        .and_then(|(_, s)| s.strip_suffix(')'))
        .filter(|s| !s.is_empty())
        .unwrap_or("unit unspecified");
    (first.into(), unit.into())
}
fn series_label(source: &str, definition: &str) -> String {
    let (name, unit) = measurement(definition);
    let details = definition
        .split_once(" · ")
        .map_or("", |(_, detail)| detail);
    let identity = source.split_once('/').map_or(source, |(_, serial)| serial);
    format!("{name} [{unit}] · {details} · USB {identity}")
}
