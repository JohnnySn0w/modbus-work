use super::*;
#[test]
fn cached_photo_failure_uses_bounded_vectors_in_both_themes() {
    for dark in [false, true] {
        let ctx = egui::Context::default();
        ctx.set_visuals(if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        for key in [
            "bridge", "dpt146", "hmd65", "wattnode", "iaq_plus", "adapter", "unknown",
        ] {
            ctx.data_mut(|d| {
                d.insert_temp(
                    egui::Id::new(("device-product-photo", key)),
                    None::<egui::TextureHandle>,
                )
            });
            let output = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(!photo(ui, key, vec2(104.0, 84.0)));
                    let before = ui.cursor().min;
                    vector(ui, key);
                    assert!(ui.cursor().min.y > before.y);
                });
            });
            assert!(output.shapes.len() > 2);
            for shape in output.shapes {
                let rect = shape.shape.visual_bounding_rect();
                assert!(!rect.min.x.is_nan());
                assert!(!rect.max.y.is_nan());
            }
        }
    }
}
