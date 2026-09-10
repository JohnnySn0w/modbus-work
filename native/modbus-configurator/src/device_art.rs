//! Offline product photos with vector fallback for unidentified devices.
use eframe::egui::{self, Color32, Stroke, pos2, vec2};

fn vector(ui: &mut egui::Ui, key: &str) {
    let (rect, response) = ui.allocate_exact_size(vec2(104.0, 84.0), egui::Sense::hover());
    response.on_hover_text("Device illustration");
    let p = ui.painter_at(rect);
    let origin = rect.min + vec2(10.0, 3.0);
    let at = |x, y| origin + vec2(x, y);
    let outline = if ui.visuals().dark_mode {
        Color32::from_rgb(176, 194, 207)
    } else {
        Color32::from_rgb(60, 80, 96)
    };
    let fill = if ui.visuals().dark_mode {
        Color32::from_rgb(40, 53, 64)
    } else {
        Color32::from_rgb(237, 242, 245)
    };
    let accent = if ui.visuals().dark_mode {
        Color32::from_rgb(97, 194, 173)
    } else {
        Color32::from_rgb(37, 113, 102)
    };
    let stroke = Stroke::new(1.7, outline);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], stroke);
    };
    let box_at = |x, y, w, h, radius: f32| {
        let r = egui::Rect::from_min_size(at(x, y), vec2(w, h));
        p.rect_filled(r, radius, fill);
        p.rect_stroke(r, radius, stroke, egui::StrokeKind::Inside);
    };
    // Use a consistent stroke and material palette across all device types.
    match key {
        "dpt146" => {
            box_at(12.0, 26.0, 39.0, 25.0, 3.0);
            box_at(4.0, 31.0, 8.0, 15.0, 2.0);
            line((7.0, 34.0), (7.0, 43.0));
            p.rect_filled(
                egui::Rect::from_min_size(at(18.0, 31.0), vec2(25.0, 15.0)),
                2.0,
                accent,
            );
            for x in [22.0, 27.0, 32.0] {
                p.line_segment([at(x, 35.0), at(x, 42.0)], Stroke::new(1.0, fill));
            }
            p.add(egui::Shape::convex_polygon(
                vec![
                    at(52.0, 25.0),
                    at(59.0, 29.0),
                    at(59.0, 48.0),
                    at(52.0, 52.0),
                    at(48.0, 48.0),
                    at(48.0, 29.0),
                ],
                fill,
                stroke,
            ));
            box_at(59.0, 32.0, 19.0, 13.0, 2.0);
            for x in [63.0, 67.0, 71.0] {
                line((x, 34.0), (x, 43.0));
            }
        }
        "bridge" => {
            box_at(20.0, 7.0, 43.0, 64.0, 4.0);
            box_at(26.0, 15.0, 31.0, 40.0, 2.0);
            for y in [24.0, 32.0, 40.0] {
                p.circle_stroke(at(32.0, y), 2.0, stroke);
                line((39.0, y), (50.0, y));
            }
            p.rect_filled(
                egui::Rect::from_min_size(at(25.0, 59.0), vec2(33.0, 9.0)),
                1.0,
                accent,
            );
            for x in [29.0, 36.0, 43.0, 50.0, 56.0] {
                p.circle_filled(at(x, 63.0), 1.2, fill);
            }
            line((28.0, 72.0), (54.0, 72.0));
        }
        "hmd65" => {
            box_at(17.0, 9.0, 47.0, 37.0, 4.0);
            box_at(25.0, 16.0, 31.0, 15.0, 2.0);
            line((26.0, 37.0), (54.0, 37.0));
            box_at(35.0, 46.0, 10.0, 27.0, 2.0);
            for y in [59.0, 63.0, 67.0] {
                line((38.0, y), (42.0, y));
            }
        }
        "wattnode" => {
            box_at(8.0, 19.0, 66.0, 42.0, 3.0);
            p.rect_filled(
                egui::Rect::from_min_size(at(14.0, 31.0), vec2(28.0, 19.0)),
                2.0,
                accent,
            );
            for x in [17.0, 27.0, 37.0, 47.0, 57.0, 67.0] {
                p.circle_stroke(at(x, 24.0), 1.8, stroke);
                p.circle_stroke(at(x, 56.0), 1.8, stroke);
            }
            line((51.0, 35.0), (66.0, 35.0));
            line((51.0, 42.0), (62.0, 42.0));
        }
        "iaq_plus" => {
            box_at(15.0, 11.0, 53.0, 57.0, 6.0);
            for y in [25.0, 31.0, 37.0, 43.0] {
                line((26.0, y), (57.0, y));
            }
            p.circle_stroke(at(41.0, 56.0), 3.0, Stroke::new(1.7, accent));
        }
        "adapter" => {
            box_at(24.0, 19.0, 34.0, 41.0, 4.0);
            box_at(31.0, 8.0, 20.0, 11.0, 1.0);
            line((36.0, 12.0), (36.0, 16.0));
            line((45.0, 12.0), (45.0, 16.0));
            box_at(29.0, 60.0, 24.0, 10.0, 1.0);
            for x in [34.0, 41.0, 48.0] {
                p.circle_filled(at(x, 65.0), 1.4, accent);
            }
            line((34.0, 34.0), (48.0, 34.0));
            line((34.0, 42.0), (48.0, 42.0));
        }
        _ => {
            // A USB candidate is not yet identified as a specific product.
            box_at(27.0, 20.0, 29.0, 40.0, 4.0);
            box_at(33.0, 10.0, 17.0, 10.0, 1.0);
            p.circle_stroke(at(41.5, 39.0), 7.0, stroke);
            line((37.0, 39.0), (46.0, 39.0));
        }
    }
}

pub fn connection(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(116.0, 32.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let color = ui.visuals().weak_text_color();
    let y = rect.center().y;
    for (a, b) in [
        (rect.left() + 3.0, rect.center().x - 29.0),
        (rect.center().x + 29.0, rect.right() - 3.0),
    ] {
        p.line_segment([pos2(a, y), pos2(b, y)], Stroke::new(1.4, color));
    }
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "RS-485",
        egui::FontId::proportional(12.0),
        color,
    );
}

fn photo_bytes(key: &str) -> Option<&'static [u8]> {
    Some(match key {
        "bridge" => include_bytes!("../assets/devices/bridge.png"),
        "dpt146" => include_bytes!("../assets/devices/dpt146.png"),
        "hmd65" => include_bytes!("../assets/devices/hmd65.png"),
        "wattnode" => include_bytes!("../assets/devices/wattnode.png"),
        "iaq_plus" => include_bytes!("../assets/devices/iaq_plus.png"),
        "adapter" => include_bytes!("../assets/devices/adapter.png"),
        "ati-f12" => include_bytes!("../assets/devices/ati-f12.png"),
        _ => return None,
    })
}

pub fn photo(ui: &mut egui::Ui, key: &str, size: egui::Vec2) -> bool {
    let Some(bytes) = photo_bytes(key) else {
        return false;
    };
    let cache_key = egui::Id::new(("device-product-photo", key));
    let cached = ui
        .ctx()
        .data(|data| data.get_temp::<Option<egui::TextureHandle>>(cache_key));
    let texture = cached.unwrap_or_else(|| {
        let texture = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .ok()
            .map(|image| {
                let rgba = image.to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                ui.ctx().load_texture(
                    format!("product-{key}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
        ui.ctx()
            .data_mut(|data| data.insert_temp(cache_key, texture.clone()));
        texture
    });
    let Some(texture) = texture else {
        return false;
    };
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    response.on_hover_text("Product reference photo");
    let inner = rect.shrink(8.0);
    let original = texture.size_vec2();
    let scale = (inner.width() / original.x).min(inner.height() / original.y);
    let image_rect = egui::Rect::from_center_size(inner.center(), original * scale);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::WHITE);
    painter.image(
        texture.id(),
        image_rect,
        egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    true
}

pub fn device(ui: &mut egui::Ui, key: &str) {
    if !photo(ui, key, vec2(225.0, 156.0)) {
        vector(ui, key);
    }
}

#[cfg(test)]
mod photo_tests {
    use super::*;
    #[test]
    fn all_supplied_photos_decode_and_unknown_devices_stay_generic() {
        for key in [
            "bridge", "dpt146", "hmd65", "wattnode", "iaq_plus", "adapter", "ati-f12",
        ] {
            let image = image::load_from_memory_with_format(
                photo_bytes(key).unwrap(),
                image::ImageFormat::Png,
            )
            .unwrap();
            assert!(image.width() >= 400 && image.height() >= 400, "{key}");
        }
        assert!(photo_bytes("synetica_usb").is_none());
        assert!(photo_bytes("unknown").is_none());
    }
}

#[cfg(test)]
#[path = "tests/art_fallback.rs"]
mod fallback_tests;
