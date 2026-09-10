//! Polygon palette transcribed from user-supplied guideline pages.
//! Official Polygon favicon artwork; fonts come from the user-supplied Fonts.zip.
use eframe::egui::{self, Color32, Stroke};
pub const CYAN: Color32 = Color32::from_rgb(0, 159, 227);
pub const DARK_GREY: Color32 = Color32::from_rgb(87, 87, 86);
pub const LOGO_GREY: Color32 = Color32::from_rgb(183, 189, 178);
pub const ORANGE: Color32 = Color32::from_rgb(221, 117, 0);
pub const BLACK: Color32 = Color32::from_rgb(29, 29, 27);
pub const DARK_BLUE: Color32 = Color32::from_rgb(31, 99, 129);
pub fn apply(ctx: &egui::Context) {
    apply_theme(ctx, false);
}
pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut v = egui::Visuals::light();
    v.panel_fill = Color32::WHITE;
    v.window_fill = Color32::WHITE;
    v.override_text_color = Some(DARK_GREY);
    v.weak_text_color = Some(DARK_GREY);
    v.hyperlink_color = DARK_BLUE;
    v.selection.bg_fill = CYAN;
    v.selection.stroke = Stroke::new(1.0, BLACK);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LOGO_GREY);
    v.widgets.inactive.bg_fill = Color32::WHITE;
    v.widgets.inactive.weak_bg_fill = Color32::WHITE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, LOGO_GREY);
    v.widgets.hovered.bg_fill = Color32::WHITE;
    v.widgets.hovered.weak_bg_fill = Color32::WHITE;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ORANGE);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, DARK_BLUE);
    v.widgets.active.bg_fill = CYAN;
    v.widgets.active.weak_bg_fill = CYAN;
    v.widgets.active.fg_stroke = Stroke::new(1.0, BLACK);
    if dark {
        v = egui::Visuals::dark();
        v.panel_fill = BLACK;
        v.window_fill = BLACK;
        v.extreme_bg_color = Color32::from_rgb(22, 22, 21);
        v.faint_bg_color = Color32::from_rgb(40, 40, 38);
        v.override_text_color = Some(Color32::from_rgb(235, 237, 232));
        v.weak_text_color = Some(LOGO_GREY);
        v.hyperlink_color = CYAN;
        v.selection.bg_fill = DARK_BLUE;
        v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, DARK_GREY);
        v.widgets.inactive.bg_fill = Color32::from_rgb(40, 40, 38);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(40, 40, 38);
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, DARK_GREY);
        v.widgets.hovered.bg_fill = DARK_BLUE;
        v.widgets.hovered.weak_bg_fill = DARK_BLUE;
        v.widgets.hovered.bg_stroke = Stroke::new(1.5, ORANGE);
        v.widgets.active.bg_fill = DARK_BLUE;
        v.widgets.active.weak_bg_fill = DARK_BLUE;
        v.widgets.active.bg_stroke = Stroke::new(1.5, CYAN);
        v.warn_fg_color = ORANGE;
    }
    ctx.set_visuals(v);
}

/// Bundled Brandon Text with egui's original fonts retained for missing glyphs.
pub fn typography(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    for (name, bytes) in [
        (
            "Brandon Light",
            include_bytes!("../assets/fonts/Brandon_txt_light.otf").as_slice(),
        ),
        (
            "Brandon Regular",
            include_bytes!("../assets/fonts/Brandon_txt_reg.otf").as_slice(),
        ),
        (
            "Brandon Medium",
            include_bytes!("../assets/fonts/Brandon_txt_med.otf").as_slice(),
        ),
        (
            "Brandon Bold",
            include_bytes!("../assets/fonts/Brandon_txt_bld.otf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), egui::FontData::from_static(bytes).into());
        let mut family = vec![name.into()];
        family.extend(fallback.clone());
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), family);
    }
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "Brandon Light".into());
    ctx.set_fonts(fonts);
    ctx.style_mut(|style| {
        for (text_style, size, name) in [
            (egui::TextStyle::Body, 16.0, "Brandon Light"),
            (egui::TextStyle::Small, 14.0, "Brandon Regular"),
            (egui::TextStyle::Button, 15.0, "Brandon Medium"),
            (egui::TextStyle::Heading, 26.0, "Brandon Bold"),
        ] {
            style.text_styles.insert(
                text_style,
                egui::FontId::new(size, egui::FontFamily::Name(name.into())),
            );
        }
    });
}

pub fn icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/branding/polygon-icon.png"))
        .expect("Bundled Polygon icon")
        .into_rgba8();
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}
