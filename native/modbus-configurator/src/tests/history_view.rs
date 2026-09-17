use super::*;
use modbus_configurator::history::Sample;
fn render(view: &mut HistoryView, history: &History) -> String {
    let ctx = egui::Context::default();
    let output = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| view.show(ui, history));
    });
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn sample(definition: &str, value: Option<f64>) -> Sample {
    Sample {
        timestamp: "2026-09-10 15:30:00".into(),
        unix_ms: 1000,
        source: "fixture-usb".into(),
        definition: definition.into(),
        value,
        status: if value.is_some() {
            "Good"
        } else {
            "Device disconnected"
        }
        .into(),
    }
}
#[test]
fn graphs_follow_recorded_series_and_empty_history() {
    let mut view = HistoryView::default();
    let mut history = History {
        samples: [sample("new point", Some(23.5))].into(),
        discarded: 1,
    };
    let text = render(&mut view, &history);
    assert!(text.contains("23.5"));
    history.samples.clear();
    let text = render(&mut view, &history);
    assert!(text.contains("Waiting for live readings"));
}
#[test]
fn failed_only_series_keeps_timestamp_status_and_real_zero_stays_successful() {
    let mut view = HistoryView::default();
    let mut history = History {
        samples: [sample("pressure", None)].into(),
        discarded: 0,
    };
    let text = render(&mut view, &history);
    assert!(text.contains("2026-09-10 15:30:00"));
    assert!(text.contains("Device disconnected"));
    assert!(text.contains("No successful samples"));
    history.samples.push_back(sample("pressure", Some(0.0)));
    let text = render(&mut view, &history);
    assert!(text.contains("Observed range: 0.000 to 0.000"));
}

#[test]
fn adapter_series_names_distinguish_points_and_chart_has_scales() {
    let temperature = "dpt146 · Slave 1 · PDU 4";
    let dewpoint = "dpt146 · Slave 1 · PDU 6";
    assert!(series_label("0403:6001/ABC", temperature).contains("Temperature"));
    assert_ne!(
        series_label("0403:6001/ABC", temperature),
        series_label("0403:6001/ABC", dewpoint)
    );
    let history = History {
        samples: [sample(temperature, Some(23.5))].into(),
        ..Default::default()
    };
    let text = render(&mut HistoryView::default(), &history);
    assert!(text.contains("Native units: °C"), "{text}");
    assert!(text.contains("Elapsed time from first sample"));
}

#[test]
fn every_slave_register_and_source_has_a_separate_graph() {
    let mut history = History::default();
    for slave in 1..=32 {
        history.samples.push_back(sample(
            &format!("Temperature (°C) · 1 / {slave} / Hold / 4 / F32 / HL / 1 / Int"),
            Some(f64::from(slave)),
        ));
    }
    history.samples.push_back(sample(
        "Temperature (°C) · 1 / 1 / Hold / 6 / F32 / HL / 1 / Int",
        None,
    ));
    let mut other_source = history.samples[0].clone();
    other_source.source = "second-usb".into();
    history.samples.push_back(other_source);
    let groups = group_samples(&history);
    assert_eq!(groups.len(), 34);
    assert!(groups.values().all(|samples| samples.len() == 1));
    for ((source, definition), _) in groups {
        let label = series_label(source, definition);
        assert!(
            label.contains("Slave ") && label.contains("Register ") && label.contains("°C"),
            "{label}"
        );
    }
}
