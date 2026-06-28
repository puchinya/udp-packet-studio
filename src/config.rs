use crate::types::{Collection, PacketDefinition, PayloadType, LogExportFormat, SocketConfig};
use serde::{Serialize, Deserialize};
use crate::locales::LanguageSetting;

fn default_language_setting() -> LanguageSetting {
    LanguageSetting::System
}

fn default_auto_save_enabled() -> bool {
    false
}

fn default_auto_save_dir() -> String {
    if let Some(config) = dirs::config_dir() {
        config.join("udp-packet-studio").join("logs").to_string_lossy().into_owned()
    } else if let Some(home) = dirs::home_dir() {
        home.join("UdpPacketStudio").join("logs").to_string_lossy().into_owned()
    } else {
        "./logs".to_string()
    }
}

fn default_auto_save_format() -> LogExportFormat {
    LogExportFormat::Csv
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedCollections {
    pub collections: Vec<Collection>,
}

fn collections_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("udp-packet-studio").join("collections.json"))
}

impl Default for SavedCollections {
    fn default() -> Self {
        Self {
            collections: vec![
                Collection {
                    id: "default_col_1".to_string(),
                    name: "Sample".to_string(),
                    is_expanded: true,
                    requests: vec![
                        PacketDefinition {
                            id: "default_1".to_string(),
                            name: "Hello".to_string(),
                            target_ip: "127.0.0.1".to_string(),
                            target_port: "9000".to_string(),
                            payload_type: PayloadType::Text,
                            payload: "Hello!".to_string(),
                        },
                    ],
                },
            ],
        }
    }
}

impl SavedCollections {
    pub fn load() -> Self {
        let mut loaded = None;
        if let Some(path) = collections_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cols) = serde_json::from_str::<Self>(&content) {
                    loaded = Some(cols);
                }
            }
        }
        if loaded.is_none() {
            if let Ok(content) = std::fs::read_to_string("collections.json") {
                if let Ok(cols) = serde_json::from_str::<Self>(&content) {
                    loaded = Some(cols);
                }
            }
        }
        loaded.unwrap_or_default()
    }

    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let mut saved = false;
            if let Some(path) = collections_path() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::write(&path, &content).is_ok() {
                    saved = true;
                }
            }
            if !saved {
                let _ = std::fs::write("collections.json", content);
            }
        }
    }
}

fn default_max_display_data_bytes() -> usize {
    128
}

fn default_max_log_lines() -> usize {
    10000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConfig {
    #[serde(skip)]
    pub collections: Vec<Collection>,
    #[serde(default)]
    pub sockets: Vec<SocketConfig>,
    #[serde(default)]
    pub selected_socket_id: String,
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
    #[serde(default = "default_max_display_data_bytes")]
    pub max_display_data_bytes: usize,
    #[serde(default = "default_max_log_lines")]
    pub max_log_lines: usize,
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("udp-packet-studio").join("settings.json"))
}

impl Default for SavedConfig {
    fn default() -> Self {
        Self {
            collections: SavedCollections::default().collections,
            sockets: vec![SocketConfig {
                id: "main".to_string(),
                name: "Main Socket".to_string(),
                ip: "0.0.0.0".to_string(),
                port: "9000".to_string(),
            }],
            selected_socket_id: "main".to_string(),
            listener_ip: "0.0.0.0".to_string(),
            listener_port: "9000".to_string(),
            composer_ip: "127.0.0.1".to_string(),
            composer_port: "9000".to_string(),
            listener_ip_history: Vec::new(),
            listener_port_history: Vec::new(),
            composer_ip_history: Vec::new(),
            composer_port_history: Vec::new(),
            composer_payload_type: PayloadType::Text,
            composer_payload: "Hello!".to_string(),
            auto_save_enabled: false,
            auto_save_dir: default_auto_save_dir(),
            auto_save_format: LogExportFormat::Csv,
            language_setting: LanguageSetting::System,
            max_display_data_bytes: 128,
            max_log_lines: 10000,
        }
    }
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
            if let Ok(content) = std::fs::read_to_string("settings.json") {
                if let Ok(config) = serde_json::from_str::<Self>(&content) {
                    loaded_config = Some(config);
                }
            }
        }

        let mut migrated = false;
        let mut config = if let Some(mut cfg) = loaded_config {
            if cfg.sockets.is_empty() {
                cfg.sockets = vec![SocketConfig {
                    id: "main".to_string(),
                    name: "Main Socket".to_string(),
                    ip: cfg.listener_ip.clone(),
                    port: cfg.listener_port.clone(),
                }];
                cfg.selected_socket_id = "main".to_string();
                migrated = true;
            }

            let ifaces = crate::get_local_interfaces();
            for socket in &mut cfg.sockets {
                let mut found = false;
                if socket.ip == "0.0.0.0" || socket.ip == "127.0.0.1" {
                    found = true;
                } else {
                    for (_, ip) in &ifaces {
                        if ip == &socket.ip {
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    socket.ip = "0.0.0.0".to_string();
                    migrated = true;
                }
            }

            if !cfg.sockets.iter().any(|s| s.id == cfg.selected_socket_id) {
                if let Some(first) = cfg.sockets.first() {
                    cfg.selected_socket_id = first.id.clone();
                    migrated = true;
                }
            }

            if migrated {
                cfg.save();
            }

            cfg
        } else {
            Self::default()
        };

        // Load collections separately
        let saved_cols = SavedCollections::load();
        config.collections = saved_cols.collections;

        config
    }

    pub fn save(&self) {
        if cfg!(test) {
            return;
        }

        // Save collections separately to collections.json
        let cols = SavedCollections { collections: self.collections.clone() };
        cols.save();

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
                let _ = std::fs::write("settings.json", content);
            }
        }
    }
}
