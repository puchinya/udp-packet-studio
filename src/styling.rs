use eframe::egui;

// Styling system
pub fn setup_custom_styles(ctx: &egui::Context) {
    // Set up custom fonts for Japanese and Emoji/Icon support
    let mut fonts = egui::FontDefinitions::default();

    // Include font binaries statically from assets
    let noto_jp_data = include_bytes!("../assets/fonts/NotoSansJP-Regular.otf");
    let noto_symbols_data = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
    let font_awesome_data = include_bytes!("../assets/fonts/fa-solid-900.ttf");

    fonts.font_data.insert(
        "noto_sans_jp".to_owned(),
        egui::FontData::from_static(noto_jp_data).into(),
    );
    fonts.font_data.insert(
        "noto_sans_symbols".to_owned(),
        egui::FontData::from_static(noto_symbols_data).into(),
    );
    fonts.font_data.insert(
        "font_awesome".to_owned(),
        egui::FontData::from_static(font_awesome_data).into(),
    );

    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(0, "noto_sans_jp".to_owned());
        family.insert(1, "noto_sans_symbols".to_owned());
        family.insert(2, "font_awesome".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.insert(0, "noto_sans_jp".to_owned());
        family.insert(1, "noto_sans_symbols".to_owned());
        family.insert(2, "font_awesome".to_owned());
    }
    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    
    // Custom premium dark-theme color tokens (Linear/Slate style)
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(13, 16, 21); // deep black slate background
    visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(20, 24, 33); // secondary panel background
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(33, 41, 54)); // subtle border
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 200, 215)); // text color
    
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(26, 33, 45); // dark button/combobox fill
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(26, 33, 45);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 72)); // button border
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(218, 226, 238));
    
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(37, 47, 64); // hovered state
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(37, 47, 64);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 110, 242)); // indigo accent border glow
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(48, 62, 85); // clicked state
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(48, 62, 85);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 130, 255));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    
    visuals.selection.bg_fill = egui::Color32::from_rgb(79, 110, 242); // selection accent color
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    
    visuals.panel_fill = egui::Color32::from_rgb(13, 16, 21); // outer panel / background fill
    visuals.window_fill = egui::Color32::from_rgb(20, 24, 33); // preferences / dialog window fill
    visuals.extreme_bg_color = egui::Color32::from_rgb(8, 10, 14); // deeper black for inputs, logs and hex view
    visuals.faint_bg_color = egui::Color32::from_rgb(22, 28, 38); // alternating grid rows
    
    // Smooth macOS-style shadows for windows and popups
    visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: egui::Color32::from_black_alpha(80),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: egui::Color32::from_black_alpha(60),
    };
    
    // Rounded layouts
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    
    ctx.set_visuals(visuals);
    
    // Tweak spacing and rounding for standard layouts
    let mut style = (*ctx.global_style()).clone();
    
    // Set custom text styles/font sizes for clear proportions
    style.text_styles = [
        (egui::TextStyle::Heading, egui::FontId::new(15.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, egui::FontId::new(13.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Button, egui::FontId::new(13.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, egui::FontId::new(12.0, egui::FontFamily::Monospace)),
        (egui::TextStyle::Small, egui::FontId::new(11.0, egui::FontFamily::Proportional)),
    ].into();

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(12);
    ctx.set_global_style(style);
}
