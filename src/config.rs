use crate::types::{Collection, PacketDefinition, PayloadType, LogExportFormat};
use serde::{Serialize, Deserialize};
use crate::locales::LanguageSetting;

fn default_language_setting() -> LanguageSetting {
    LanguageSetting::System
}

fn default_auto_save_enabled() -> bool {
    false
}

fn default_auto_save_dir() -> String {
    if let Some(home) = dirs::home_dir() {
        home.join("UdpPacketStudio").join("Logs").to_string_lossy().into_owned()
    } else {
        "./Logs".to_string()
    }
}

fn default_auto_save_format() -> LogExportFormat {
    LogExportFormat::Csv
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConfig {
    pub collections: Vec<Collection>,
    pub listener_ip: String,
    pub listener_port: String,
    pub composer_ip: String,
    pub composer_port: String,
    #[serde(default)]
    pub listener_ip_history: Vec<String>,
    #[serde(default)]
    pub listener_port_history: Vec<String>,
    #[serde(default)]
    pub composer_ip_history: Vec<String>,
    #[serde(default)]
    pub composer_port_history: Vec<String>,
    pub composer_payload_type: PayloadType,
    pub composer_payload: String,
    #[serde(default = "default_auto_save_enabled")]
    pub auto_save_enabled: bool,
    #[serde(default = "default_auto_save_dir")]
    pub auto_save_dir: String,
    #[serde(default = "default_auto_save_format")]
    pub auto_save_format: LogExportFormat,
    #[serde(default = "default_language_setting")]
    pub language_setting: LanguageSetting,
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("udp-packet-studio").join("updexp_config.json"))
}

impl SavedConfig {
    pub fn load() -> Self {
        let mut loaded_config = None;

        // Try reading from app-specific config directory first
        if let Some(path) = config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<Self>(&content) {
                    loaded_config = Some(config);
                }
            }
        }

        // Fallback to local file in current directory if not found in config directory
        if loaded_config.is_none() {
            if let Ok(content) = std::fs::read_to_string("updexp_config.json") {
                if let Ok(config) = serde_json::from_str::<Self>(&content) {
                    loaded_config = Some(config);
                }
            }
        }

        if let Some(config) = loaded_config {
            return config;
        }
        
        Self {
            collections: vec![
                Collection {
                    id: "default_col_1".to_string(),
                    name: "ECHONET Lite Queries".to_string(),
                    is_expanded: true,
                    requests: vec![
                        PacketDefinition {
                            id: "default_1".to_string(),
                            name: "Aircon Get Operation".to_string(),
                            target_ip: "127.0.0.1".to_string(),
                            target_port: "3610".to_string(),
                            payload_type: PayloadType::Hex,
                            payload: "10 81 00 01 05 FF 01 01 30 01 62 01 80 00".to_string(),
                        },
                        PacketDefinition {
                            id: "default_2".to_string(),
                            name: "Node Profile Get".to_string(),
                            target_ip: "127.0.0.1".to_string(),
                            target_port: "3610".to_string(),
                            payload_type: PayloadType::Hex,
                            payload: "10 81 00 02 05 FF 01 0E F0 01 62 01 D6 00".to_string(),
                        },
                    ],
                },
                Collection {
                    id: "default_col_2".to_string(),
                    name: "General UDP Tests".to_string(),
                    is_expanded: true,
                    requests: vec![
                        PacketDefinition {
                            id: "default_3".to_string(),
                            name: "Local Loopback Ping".to_string(),
                            target_ip: "127.0.0.1".to_string(),
                            target_port: "9000".to_string(),
                            payload_type: PayloadType::Text,
                            payload: "Ping!".to_string(),
                        },
                    ],
                },
            ],
            listener_ip: "0.0.0.0".to_string(),
            listener_port: "9000".to_string(),
            composer_ip: "127.0.0.1".to_string(),
            composer_port: "9000".to_string(),
            listener_ip_history: Vec::new(),
            listener_port_history: Vec::new(),
            composer_ip_history: Vec::new(),
            composer_port_history: Vec::new(),
            composer_payload_type: PayloadType::Text,
            composer_payload: "Hello from Composer!".to_string(),
            auto_save_enabled: false,
            auto_save_dir: default_auto_save_dir(),
            auto_save_format: LogExportFormat::Csv,
            language_setting: LanguageSetting::System,
        }
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let mut saved = false;
            if let Some(path) = config_path() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::write(&path, &content).is_ok() {
                    saved = true;
                }
            }
            if !saved {
                let _ = std::fs::write("updexp_config.json", content);
            }
        }
    }
}
