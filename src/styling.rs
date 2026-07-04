use eframe::egui;
use crate::types::AppTheme;

// Setup custom fonts statically (called once at startup)
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Include font binaries statically from assets
    let noto_jp_data = include_bytes!("../assets/fonts/NotoSansJP-Regular.otf");
    let noto_symbols_data = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
    let font_awesome_data = include_bytes!("../assets/fonts/fa-solid-900.ttf");

    fonts.font_data.insert(
        "noto_sans_jp".to_owned(),
        egui::FontData::from_static(noto_jp_data).into(),
    );
    let mut symbols_font = egui::FontData::from_static(noto_symbols_data);
    symbols_font.tweak = egui::FontTweak {
        y_offset_factor: 0.08,
        ..Default::default()
    };
    fonts.font_data.insert(
        "noto_sans_symbols".to_owned(),
        symbols_font.into(),
    );

    let mut fa_font = egui::FontData::from_static(font_awesome_data);
    fa_font.tweak = egui::FontTweak {
        y_offset_factor: 0.08,
        ..Default::default()
    };
    fonts.font_data.insert(
        "font_awesome".to_owned(),
        fa_font.into(),
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

    // Tweak spacing and rounding for standard layouts (fonts-dependent metrics)
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
    style.spacing.interact_size = egui::vec2(40.0, 30.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.interaction.tooltip_delay = 0.2;
    ctx.set_global_style(style);
}

// Apply the selected theme visuals
pub fn apply_theme(ctx: &egui::Context, theme: AppTheme) {
    let resolved_theme = match theme {
        AppTheme::System => {
            match ctx.theme() {
                egui::Theme::Dark => AppTheme::Dark,
                egui::Theme::Light => AppTheme::Light,
            }
        }
        AppTheme::Dark => AppTheme::Dark,
        AppTheme::Light => AppTheme::Light,
    };

    let mut visuals = match resolved_theme {
        AppTheme::Dark => {
            let mut vis = egui::Visuals::dark();
            // Custom premium dark-theme color tokens (Linear/Slate style)
            vis.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(13, 16, 21); // deep black slate background
            vis.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(20, 24, 33); // secondary panel background
            vis.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(33, 41, 54)); // subtle border
            vis.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 200, 215)); // text color
            
            vis.widgets.inactive.bg_fill = egui::Color32::from_rgb(26, 33, 45); // dark button/combobox fill
            vis.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(26, 33, 45);
            vis.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 72)); // button border
            vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(218, 226, 238));
            
            vis.widgets.hovered.bg_fill = egui::Color32::from_rgb(37, 47, 64); // hovered state
            vis.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(37, 47, 64);
            vis.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 110, 242)); // indigo accent border glow
            vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            
            vis.widgets.active.bg_fill = egui::Color32::from_rgb(48, 62, 85); // clicked state
            vis.widgets.active.weak_bg_fill = egui::Color32::from_rgb(48, 62, 85);
            vis.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 130, 255));
            vis.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            
            vis.selection.bg_fill = egui::Color32::from_rgb(79, 110, 242); // selection accent color
            vis.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

            vis.panel_fill = egui::Color32::from_rgb(13, 16, 21); // outer panel / background fill
            vis.window_fill = egui::Color32::from_rgb(20, 24, 33); // preferences / dialog window fill
            vis.extreme_bg_color = egui::Color32::from_rgb(8, 10, 14); // deeper black for inputs, logs and hex view
            vis.faint_bg_color = egui::Color32::from_rgb(22, 28, 38); // alternating grid rows
            
            // Smooth shadows for windows and popups
            vis.window_shadow = egui::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 0,
                color: egui::Color32::from_black_alpha(80),
            };
            vis.popup_shadow = egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(60),
            };
            vis
        }
        AppTheme::Light => {
            let mut vis = egui::Visuals::light();
            // Custom premium light-theme color tokens (Slate light style)
            vis.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(245, 247, 250); // slate white background
            vis.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(238, 241, 246); // secondary panel background
            vis.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(209, 217, 227)); // subtle border
            vis.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 72)); // text color (dark gray)
            
            vis.widgets.inactive.bg_fill = egui::Color32::from_rgb(226, 232, 240); // light button/combobox fill
            vis.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(226, 232, 240);
            vis.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(203, 213, 225)); // button border
            vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 41, 59));
            
            vis.widgets.hovered.bg_fill = egui::Color32::from_rgb(203, 213, 225); // hovered state
            vis.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(203, 213, 225);
            vis.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(79, 110, 242)); // indigo accent border glow
            vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(15, 23, 42));
            
            vis.widgets.active.bg_fill = egui::Color32::from_rgb(148, 163, 184); // clicked state
            vis.widgets.active.weak_bg_fill = egui::Color32::from_rgb(148, 163, 184);
            vis.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(67, 94, 206));
            vis.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(15, 23, 42));
            
            vis.selection.bg_fill = egui::Color32::from_rgb(79, 110, 242); // selection accent color (indigo)
            vis.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

            vis.panel_fill = egui::Color32::from_rgb(245, 247, 250); // outer panel / background fill
            vis.window_fill = egui::Color32::from_rgb(255, 255, 255); // preferences / dialog window fill
            vis.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255); // white for inputs, logs and hex view
            vis.faint_bg_color = egui::Color32::from_rgb(238, 241, 246); // alternating grid rows
            
            // Smooth shadows for windows and popups
            vis.window_shadow = egui::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 0,
                color: egui::Color32::from_black_alpha(20),
            };
            vis.popup_shadow = egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(15),
            };
            vis
        }
        _ => unreachable!(),
    };

    // Rounded layouts (common)
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
    visuals.window_corner_radius = egui::CornerRadius::same(12);

    ctx.set_visuals(visuals);
}

// Main entry point for styling initialization
pub fn setup_custom_styles(ctx: &egui::Context, theme: AppTheme) {
    setup_fonts(ctx);
    apply_theme(ctx, theme);
}
