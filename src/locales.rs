use serde::{Deserialize, Serialize};
use std::sync::Once;

static INIT: Once = Once::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageSetting {
    System,
    Japanese,
    English,
}

impl Default for LanguageSetting {
    fn default() -> Self {
        LanguageSetting::System
    }
}

impl LanguageSetting {
    pub fn to_display_name(self) -> String {
        match self {
            LanguageSetting::System => egui_i18n::tr!("settings-lang-system"),
            LanguageSetting::Japanese => egui_i18n::tr!("settings-lang-ja"),
            LanguageSetting::English => egui_i18n::tr!("settings-lang-en"),
        }
    }
}

pub fn resolve_language(setting: LanguageSetting) -> String {
    match setting {
        LanguageSetting::System => {
            if let Some(locale) = sys_locale::get_locale() {
                if locale.starts_with("ja") {
                    "ja".to_string()
                } else {
                    "en".to_string()
                }
            } else {
                "en".to_string()
            }
        }
        LanguageSetting::Japanese => "ja".to_string(),
        LanguageSetting::English => "en".to_string(),
    }
}

pub fn init_translations() {
    INIT.call_once(|| {
        egui_i18n::set_use_isolating(false);
        let en_ftl = include_str!("../locales/en/main.ftl");
        let ja_ftl = include_str!("../locales/ja/main.ftl");

        egui_i18n::load_translations_from_text("en", en_ftl).unwrap();
        egui_i18n::load_translations_from_text("ja", ja_ftl).unwrap();

        egui_i18n::set_fallback("en");
    });
}
