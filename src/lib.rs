#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod udp_worker;
pub mod types;
pub mod config;
pub mod styling;
pub mod views;
pub mod locales;
pub mod mra;
pub mod mra_defs;
pub mod filter;

use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender, channel};
use chrono::Local;
use eframe::egui;
use egui_dock::{DockArea, DockState};
use egui_dock::tab_viewer::OnCloseResponse;

use udp_worker::{UdpWorker, UdpCommand, UdpEvent};
use types::{Tab, LogEntry, LogDirection, PayloadType, parse_hex_to_bytes, Collection, MulticastGroup, InspectorProtocol, LogExportFormat, LoggerCommand, AboutTab, ElBuilderProperty};
use config::SavedConfig;
use styling::setup_custom_styles;
use locales::LanguageSetting;

pub fn get_local_interfaces() -> Vec<(String, String)> {
    let mut list = Vec::new();
    if let Ok(interfaces) = get_if_addrs::get_if_addrs() {
        for iface in interfaces {
            if !iface.is_loopback() {
                if let get_if_addrs::IfAddr::V4(v4_addr) = iface.addr {
                    list.push((iface.name, v4_addr.ip.to_string()));
                }
            }
        }
    }
    list
}

pub struct UdpStudioState {
    pub collections: Vec<Collection>,
    pub selected_request_id: Option<String>,
    pub composer_selected_collection_idx: usize,
    
    // Composer tab inputs
    pub composer_ip: String,
    pub composer_port: String,
    pub composer_ip_history: Vec<String>,
    pub composer_port_history: Vec<String>,
    pub composer_payload_type: PayloadType,
    pub composer_payload: String,
    pub composer_name: String,
    
    // Logs tab inputs
    pub logs: Vec<LogEntry>,
    pub selected_log_idx: Option<usize>,
    pub filter_text: String,
    pub filter_input: String,
    pub filter_history: Vec<String>,
    pub auto_scroll: bool,
    pub log_export_format: LogExportFormat,
    pub filtered_indices: Vec<usize>,
    
    // Sockets settings
    pub sockets: Vec<crate::types::ActiveSocketState>,
    pub selected_socket_id: String,
    pub multicast_selected_socket_id: String,
    pub listener_ip_history: Vec<String>,
    pub listener_port_history: Vec<String>,
    
    // Channels & Worker
    pub udp_worker: UdpWorker,
    pub rx_event: Receiver<UdpEvent>,

    // ECHONET Lite Helper state
    pub el_tid: String,
    pub el_seoj: String,
    pub el_deoj_preset: usize,
    pub el_deoj_custom: String,
    pub el_deoj_eoj: String,          // resolved EOJ hex (4 chars, no 0x prefix)
    pub el_esv_preset: usize,
    pub el_properties: Vec<ElBuilderProperty>, // list of EPC+EDT rows
    pub el_show_helper: bool,

    // Multicast fields (UI inputs)
    pub multicast_input_addr: String,
    pub multicast_input_interface: String,

    // Inspector fields
    pub inspector_protocol: InspectorProtocol,

    pub auto_save_enabled: bool,
    pub auto_save_dir: String,
    pub auto_save_format: LogExportFormat,
    pub max_display_data_bytes: usize,
    pub max_log_lines: usize,
    pub settings_open: bool,
    pub settings_reset_confirm_open: bool,
    pub about_open: bool,
    pub about_tab: AboutTab,
    pub tx_logger: Sender<LoggerCommand>,
    pub language_setting: LanguageSetting,
    pub mra_db: mra::MraDatabase,
    pub dock_state_serialized: Option<String>,
    pub reset_layout_requested: bool,
}

impl UdpStudioState {
    pub fn get_selected_socket(&self) -> Option<&crate::types::ActiveSocketState> {
        self.sockets.iter().find(|s| s.id == self.selected_socket_id)
    }

    pub fn get_selected_socket_mut(&mut self) -> Option<&mut crate::types::ActiveSocketState> {
        self.sockets.iter_mut().find(|s| s.id == self.selected_socket_id)
    }

    pub fn reset_settings(&mut self) {
        let def = SavedConfig::default();
        self.sockets = def.sockets.iter().map(|s| crate::types::ActiveSocketState {
            id: s.id.clone(),
            name: s.name.clone(),
            ip: s.ip.clone(),
            port: s.port.clone(),
            is_listening: false,
            bound_addr: None,
            error: None,
            bind_time: None,
            multicast_groups: Vec::new(),
        }).collect();
        self.selected_socket_id = def.selected_socket_id.clone();
        self.multicast_selected_socket_id = def.selected_socket_id;
        self.composer_ip = def.composer_ip;
        self.composer_port = def.composer_port;
        self.listener_ip_history = def.listener_ip_history;
        self.listener_port_history = def.listener_port_history;
        self.composer_ip_history = def.composer_ip_history;
        self.composer_port_history = def.composer_port_history;
        self.composer_payload_type = def.composer_payload_type;
        self.composer_payload = def.composer_payload;
        self.auto_save_enabled = def.auto_save_enabled;
        self.auto_save_dir = def.auto_save_dir;
        self.auto_save_format = def.auto_save_format;
        self.language_setting = def.language_setting;
        self.max_display_data_bytes = def.max_display_data_bytes;
        self.max_log_lines = def.max_log_lines;
        self.enforce_log_limits();
        self.save_config();
        self.update_logger_config();
    }

    pub fn add_to_listener_history(&mut self, port: String) {
        let port = port.trim().to_string();
        if !port.is_empty() {
            self.listener_port_history.retain(|x| x != &port);
            self.listener_port_history.insert(0, port);
            self.listener_port_history.truncate(20);
        }
        self.save_config();
    }

    pub fn add_to_composer_history(&mut self, ip: String, port: String) {
        let ip = ip.trim().to_string();
        if !ip.is_empty() {
            self.composer_ip_history.retain(|x| x != &ip);
            self.composer_ip_history.insert(0, ip);
            self.composer_ip_history.truncate(20);
        }
        let port = port.trim().to_string();
        if !port.is_empty() {
            self.composer_port_history.retain(|x| x != &port);
            self.composer_port_history.insert(0, port);
            self.composer_port_history.truncate(20);
        }
        self.save_config();
    }

    pub fn language_id(&self) -> String {
        crate::locales::resolve_language(self.language_setting)
    }

    pub fn tr(&self, key: &str) -> String {
        crate::locales::init_translations();
        egui_i18n::set_language(&self.language_id());
        egui_i18n::tr!(key)
    }

    pub fn tr_with_args(&self, key: &str, args: &std::collections::HashMap<std::borrow::Cow<'static, str>, egui_i18n::fluent_bundle::FluentValue<'_>>) -> String {
        crate::locales::init_translations();
        egui_i18n::set_language(&self.language_id());
        let mut fluent_args = egui_i18n::fluent::FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(k.as_ref(), v.clone());
        }
        egui_i18n::translate_fluent(key, &fluent_args)
    }

    pub(crate) fn save_config(&self) {
        #[cfg(not(test))]
        {
            let config = SavedConfig {
                collections: self.collections.clone(),
                listener_ip: String::new(),
                listener_port: String::new(),
                sockets: self.sockets.iter().map(|s| crate::types::SocketConfig {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    ip: s.ip.clone(),
                    port: s.port.clone(),
                }).collect(),
                selected_socket_id: self.selected_socket_id.clone(),
                composer_ip: self.composer_ip.clone(),
                composer_port: self.composer_port.clone(),
                listener_ip_history: self.listener_ip_history.clone(),
                listener_port_history: self.listener_port_history.clone(),
                composer_ip_history: self.composer_ip_history.clone(),
                composer_port_history: self.composer_port_history.clone(),
                composer_payload_type: self.composer_payload_type,
                composer_payload: self.composer_payload.clone(),
                auto_save_enabled: self.auto_save_enabled,
                auto_save_dir: self.auto_save_dir.clone(),
                auto_save_format: self.auto_save_format,
                language_setting: self.language_setting,
                max_display_data_bytes: self.max_display_data_bytes,
                max_log_lines: self.max_log_lines,
                dock_state: self.dock_state_serialized.clone(),
            };
            config.save();
        }
    }

    pub(crate) fn update_logger_config(&self) {
        let listener_addr = self.get_selected_socket()
            .map(|s| format!("{}:{}", s.ip, s.port))
            .unwrap_or_else(|| "0.0.0.0:9000".to_string());
        let bind_time = self.get_selected_socket().and_then(|s| s.bind_time);
        let _ = self.tx_logger.send(LoggerCommand::Configure {
            enabled: self.auto_save_enabled,
            dir: self.auto_save_dir.clone(),
            format: self.auto_save_format,
            listener_addr,
            bind_time,
        });
    }

    pub fn enforce_log_limits(&mut self) {
        let max_lines = self.max_log_lines;
        if self.logs.len() > max_lines {
            let remove_count = self.logs.len() - max_lines;
            self.logs.drain(0..remove_count);
            if let Some(idx) = self.selected_log_idx {
                if idx < remove_count {
                    self.selected_log_idx = None;
                } else {
                    self.selected_log_idx = Some(idx - remove_count);
                }
            }
        }
    }

    pub fn push_log(&mut self, entry: LogEntry) {
        self.logs.push(entry.clone());
        self.enforce_log_limits();
        let _ = self.tx_logger.send(LoggerCommand::Log(entry));
        self.update_filtered_indices();
    }

    pub(crate) fn add_system_info(&mut self, msg: String) {
        let entry = LogEntry::new(
            Local::now(),
            LogDirection::SystemInfo,
            SocketAddr::from(([0, 0, 0, 0], 0)),
            msg.into_bytes(),
        );
        self.push_log(entry);
    }

    pub(crate) fn add_system_error(&mut self, msg: String) {
        let entry = LogEntry::new(
            Local::now(),
            LogDirection::SystemError,
            SocketAddr::from(([0, 0, 0, 0], 0)),
            msg.into_bytes(),
        );
        self.push_log(entry);
    }

    pub(crate) fn update_filtered_indices(&mut self) {
        let parsed_filter = if self.filter_text.trim().is_empty() {
            None
        } else {
            crate::filter::parse_filter(&self.filter_text).ok()
        };

        self.filtered_indices = self.logs
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if let Some(ref filter) = parsed_filter {
                    filter.eval(entry)
                } else {
                    self.filter_text.trim().is_empty()
                }
            })
            .map(|(idx, _)| idx)
            .collect();
    }

    pub(crate) fn apply_filter(&mut self) {
        let trimmed = self.filter_input.trim().to_string();
        if trimmed.is_empty() {
            self.filter_text = String::new();
            self.update_filtered_indices();
            return;
        }

        match crate::filter::parse_filter(&trimmed) {
            Ok(_) => {
                self.filter_text = trimmed.clone();
                self.update_filtered_indices();

                // 履歴に追加 (数字始まり＝IPアドレスのみは覚えない)
                let is_ip_only = trimmed.chars().next().map_or(false, |c| c.is_ascii_digit());
                if !is_ip_only {
                    if let Some(pos) = self.filter_history.iter().position(|x| x == &trimmed) {
                        self.filter_history.remove(pos);
                    }
                    self.filter_history.insert(0, trimmed);
                    if self.filter_history.len() > 20 {
                        self.filter_history.truncate(20);
                    }
                }
            }
            Err(err) => {
                self.add_system_error(format!("Filter Syntax Error: {}", err));
            }
        }
    }

    pub(crate) fn send_packet(&mut self, target: &str, payload_type: PayloadType, payload: &str) {
        let data_res = match payload_type {
            PayloadType::Text => Ok(payload.as_bytes().to_vec()),
            PayloadType::Hex => parse_hex_to_bytes(payload),
        };
        
        match data_res {
            Ok(data) => {
                if data.is_empty() {
                    self.add_system_error("Cannot send empty packet.".to_string());
                    return;
                }
                self.udp_worker.send(UdpCommand::Send {
                    id: self.selected_socket_id.clone(),
                    target: target.to_string(),
                    data,
                });
            }
            Err(e) => {
                self.add_system_error(format!("Hex parsing error: {}", e));
            }
        }
    }

    pub fn show_socket_bind_controls(&mut self, ui: &mut egui::Ui, socket_idx: usize, is_in_navbar: bool) {
        if socket_idx >= self.sockets.len() {
            return;
        }

        let is_listening = self.sockets[socket_idx].is_listening;
        let id = self.sockets[socket_idx].id.clone();

        ui.horizontal(|ui| {
            if is_in_navbar {
                ui.label(self.tr("titlebar-bind-addr"));
            }

            ui.add_enabled_ui(!is_listening, |ui| {
                let mut ip_chosen = None;
                let current_ip = self.sockets[socket_idx].ip.clone();
                let combo_id = format!("bind_ip_combo_{}", id);
                egui::ComboBox::from_id_salt(combo_id)
                    .selected_text(&current_ip)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(current_ip == "0.0.0.0", "0.0.0.0 (All interfaces)").clicked() {
                            ip_chosen = Some("0.0.0.0".to_string());
                        }
                        if ui.selectable_label(current_ip == "127.0.0.1", "127.0.0.1 (Loopback)").clicked() {
                            ip_chosen = Some("127.0.0.1".to_string());
                        }
                        let ifaces = crate::get_local_interfaces();
                        if !ifaces.is_empty() {
                            ui.separator();
                            for (name, ip) in &ifaces {
                                if ui.selectable_label(current_ip == *ip, format!("{} ({})", ip, name)).clicked() {
                                    ip_chosen = Some(ip.clone());
                                }
                            }
                        }
                    });
                if let Some(ip) = ip_chosen {
                    self.sockets[socket_idx].ip = ip;
                    self.save_config();
                }
            });

            ui.label(":");

            ui.add_enabled_ui(!is_listening, |ui| {
                let mut port_chosen = None;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                    let edit_res = ui.add(egui::TextEdit::singleline(&mut self.sockets[socket_idx].port).desired_width(55.0));
                    if edit_res.changed() {
                        self.save_config();
                    }
                    ui.menu_button("▾", |ui| {
                        ui.set_min_width(150.0);
                        ui.menu_button("Presets", |ui| {
                            if ui.button("ECHONET Lite : 3610").clicked() {
                                port_chosen = Some("3610".to_string());
                                ui.close();
                            }
                        });
                        if !self.listener_port_history.is_empty() {
                            ui.separator();
                            for h in &self.listener_port_history {
                                if ui.button(h).clicked() {
                                    port_chosen = Some(h.clone());
                                    ui.close();
                                }
                            }
                        }
                    });
                });
                if let Some(port) = port_chosen {
                    self.sockets[socket_idx].port = port;
                    self.save_config();
                }
            });

            ui.add_space(5.0);

            if is_listening {
                if ui.add(egui::Button::new(egui::RichText::new(self.tr("titlebar-btn-stop")).color(egui::Color32::from_rgb(255, 100, 100)))).clicked() {
                    self.udp_worker.send(UdpCommand::Unbind { id: id.clone() });
                }
            } else {
                let is_port_valid = {
                    let p = self.sockets[socket_idx].port.trim();
                    !p.is_empty() && p != "0" && p.parse::<u16>().is_ok()
                };
                let bind_btn = ui.add_enabled(
                    is_port_valid,
                    egui::Button::new(egui::RichText::new(self.tr("titlebar-btn-bind")).color(egui::Color32::from_rgb(100, 255, 100)))
                );
                if bind_btn.clicked() {
                    self.sockets[socket_idx].error = None;
                    let ip = self.sockets[socket_idx].ip.trim().to_string();
                    let port = self.sockets[socket_idx].port.trim().to_string();
                    self.add_to_listener_history(port.clone());
                    let bind_addr = format!("{}:{}", ip, port);
                    self.udp_worker.send(UdpCommand::Bind { id: id.clone(), addr: bind_addr });
                }
            }

            if let Some(ref err) = self.sockets[socket_idx].error {
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), format!("⚠ {}", err));
            }
        });
    }

    pub fn show_socket_list_window(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // List of sockets
            let mut socket_to_delete = None;
            egui::ScrollArea::vertical().id_salt("sockets_scroll").show(ui, |ui| {
                let len = self.sockets.len();
                for idx in 0..len {
                    let socket = &self.sockets[idx];
                    let id = socket.id.clone();
                    let is_main = id == "main";

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Editable socket name
                            ui.label(self.tr("sockets-lbl-name"));
                            let mut name_changed = false;
                            let mut name = self.sockets[idx].name.clone();
                            let edit_res = ui.text_edit_singleline(&mut name);
                            if edit_res.changed() {
                                self.sockets[idx].name = name;
                                name_changed = true;
                            }
                            if edit_res.lost_focus() && name_changed {
                                self.save_config();
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !is_main {
                                    if ui.button("🗑").on_hover_text(self.tr("sockets-tooltip-delete")).clicked() {
                                        socket_to_delete = Some(idx);
                                    }
                                }
                            });
                        });

                        ui.add_space(4.0);

                        // Bind controls
                        self.show_socket_bind_controls(ui, idx, false);
                    });
                    ui.add_space(8.0);
                }
            });

            // Perform deletion if selected
            if let Some(pos) = socket_to_delete {
                let socket = &self.sockets[pos];
                if socket.id != "main" {
                    if socket.is_listening {
                        self.udp_worker.send(UdpCommand::Unbind { id: socket.id.clone() });
                    }
                    if self.selected_socket_id == socket.id {
                        self.selected_socket_id = "main".to_string();
                    }
                    if self.multicast_selected_socket_id == socket.id {
                        self.multicast_selected_socket_id = "main".to_string();
                    }
                    self.sockets.remove(pos);
                    self.save_config();
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Button to add socket
            let add_btn = ui.add(egui::Button::new(egui::RichText::new(self.tr("sockets-btn-add")).strong()));
            if add_btn.clicked() {
                let new_id = crate::types::generate_id();
                let next_idx = self.sockets.len() + 1;
                let default_name = format!("Socket {}", next_idx);
                self.sockets.push(crate::types::ActiveSocketState {
                    id: new_id,
                    name: default_name,
                    ip: "0.0.0.0".to_string(),
                    port: "9000".to_string(),
                    is_listening: false,
                    bound_addr: None,
                    error: None,
                    bind_time: None,
                    multicast_groups: Vec::new(),
                });
                self.save_config();
            }
        });
    }
}

struct MyTabViewer<'a> {
    state: &'a mut UdpStudioState,
}

impl<'a> egui_dock::TabViewer for MyTabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::Collections => self.state.tr("tabs-collections").into(),
            Tab::Sender => self.state.tr("tabs-composer").into(),
            Tab::LogViewer => self.state.tr("tabs-logs").into(),
            Tab::Inspector => self.state.tr("tabs-inspector").into(),
            Tab::Multicast => self.state.tr("tabs-multicast").into(),
            Tab::Sockets => self.state.tr("sockets-window-title").into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Collections => self.state.show_collections(ui),
            Tab::Sender => self.state.show_sender(ui),
            Tab::LogViewer => self.state.show_log_viewer(ui),
            Tab::Inspector => self.state.show_inspector(ui),
            Tab::Multicast => self.state.show_multicast(ui),
            Tab::Sockets => self.state.show_socket_list_window(ui),
        }
    }

    fn is_closeable(&self, _tab: &Self::Tab) -> bool {
        false
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> OnCloseResponse {
        OnCloseResponse::Ignore
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

struct MainApp {
    dock_state: DockState<Tab>,
    state: UdpStudioState,
    was_focused: bool,
}

impl MainApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_styles(&cc.egui_ctx);
        
        let config = SavedConfig::load();
        
        // Try to restore docking layout from config
        let mut dock_state = None;
        if let Some(ref json_str) = config.dock_state {
            if let Ok(state) = serde_json::from_str::<DockState<Tab>>(json_str) {
                dock_state = Some(state);
            }
        }

        let dock_state = dock_state.unwrap_or_else(|| {
            // Setup initial docking layout tree (3-column split where center is stacked)
            let mut ds = DockState::new(vec![Tab::Collections]);
            let surface = ds.main_surface_mut();
            
            // Split right to place LogViewer in the middle
            let [_left, middle] = surface.split_right(egui_dock::NodeIndex::root(), 0.25, vec![Tab::LogViewer]);
            // Split middle to place Sender and Sockets on the right. 'center' now points to the leaf node with Tab::LogViewer.
            let [center, right] = surface.split_right(middle, 0.60, vec![Tab::Sender, Tab::Sockets]);
            // Split center below to place Inspector at the bottom of the center column
            let [_, _] = surface.split_below(center, 0.55, vec![Tab::Inspector]);
            // Split right below to place Multicast at the bottom of the right column
            let [_, _] = surface.split_below(right, 0.55, vec![Tab::Multicast]);
            ds
        });
        
        let (tx_event, rx_event) = channel();
        let udp_worker = UdpWorker::spawn(tx_event, cc.egui_ctx.clone());
        
        let (tx_logger, rx_logger) = channel();
        let init_enabled = config.auto_save_enabled;
        let init_dir = config.auto_save_dir.clone();
        let init_format = config.auto_save_format;
        let selected_socket = config.sockets.iter().find(|s| s.id == config.selected_socket_id)
            .or_else(|| config.sockets.first())
            .unwrap();
        let init_addr = format!("{}:{}", selected_socket.ip, selected_socket.port);
        
        std::thread::spawn(move || {
            let mut enabled = init_enabled;
            let mut dir = init_dir;
            let mut format = init_format;
            let mut listener_addr = init_addr;
            let mut bind_time: Option<chrono::DateTime<chrono::Local>> = None;
            
            while let Ok(cmd) = rx_logger.recv() {
                match cmd {
                    LoggerCommand::Configure { enabled: e, dir: d, format: f, listener_addr: addr, bind_time: bt } => {
                        enabled = e;
                        dir = d;
                        format = f;
                        listener_addr = addr;
                        bind_time = bt;
                    }
                    LoggerCommand::Log(entry) => {
                        if enabled && !dir.is_empty() {
                            let ref_time = bind_time.unwrap_or(entry.timestamp);
                            let date_str = ref_time.format("%Y-%m-%d_%H-%M-%S").to_string();
                            let extension = match format {
                                LogExportFormat::Csv => "csv",
                                LogExportFormat::Json => "json",
                                LogExportFormat::Pcap => "pcap",
                            };
                            let file_name = format!("udp_log_{}.{}", date_str, extension);
                            let path = std::path::Path::new(&dir).join(file_name);
                            
                            if let Some(parent) = path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            
                            let file_exists = path.exists();
                            
                            match format {
                                LogExportFormat::Csv => {
                                    if let Ok(mut file) = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(&path)
                                    {
                                        use std::io::Write;
                                        if !file_exists {
                                            let _ = writeln!(file, "Timestamp,Direction,Src IP,Src Port,Dest IP,Dest Port,Length,DataHex,DataText");
                                        }
                                        let time_str = entry.timestamp.format("%Y-%m-%d %H:%M:%S.%3f").to_string();
                                        let dir_str = match entry.direction {
                                            LogDirection::Sent => "SENT",
                                            LogDirection::Received => "RECV",
                                            LogDirection::SystemInfo => "INFO",
                                            LogDirection::SystemError => "ERROR",
                                        };

                                        let len_str = entry.data.len().to_string();
                                        let hex_str = entry.data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
                                        let plain_str = String::from_utf8_lossy(&entry.data).replace('\n', " ").replace('"', "\"\"");
                                        let _ = writeln!(file, "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"",
                                            time_str, dir_str, entry.src_ip, entry.src_port, entry.dest_ip, entry.dest_port, len_str, hex_str, plain_str);
                                    }
                                }
                                LogExportFormat::Json => {
                                    if let Ok(mut file) = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(&path)
                                    {
                                        use std::io::Write;
                                        if let Ok(json_str) = serde_json::to_string(&entry) {
                                            let _ = writeln!(file, "{}", json_str);
                                        }
                                    }
                                }
                                LogExportFormat::Pcap => {
                                    if entry.direction == LogDirection::SystemInfo || entry.direction == LogDirection::SystemError {
                                        continue;
                                    }
                                    if let Ok(mut file) = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(&path)
                                    {
                                        use std::io::Write;
                                        if !file_exists {
                                            let _ = file.write_all(&0xa1b2c3d4u32.to_ne_bytes());
                                            let _ = file.write_all(&2u16.to_ne_bytes());
                                            let _ = file.write_all(&4u16.to_ne_bytes());
                                            let _ = file.write_all(&0i32.to_ne_bytes());
                                            let _ = file.write_all(&0u32.to_ne_bytes());
                                            let _ = file.write_all(&65535u32.to_ne_bytes());
                                            let _ = file.write_all(&1u32.to_ne_bytes());
                                        }
                                        
                                        let local_ip_parsed = listener_addr.split(':').next().unwrap_or("127.0.0.1")
                                            .parse::<std::net::IpAddr>().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
                                        let local_port = listener_addr.split(':').nth(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(9000);

                                        let src_ip = entry.src_ip.parse::<std::net::IpAddr>().unwrap_or(local_ip_parsed);
                                        let dest_ip = entry.dest_ip.parse::<std::net::IpAddr>().unwrap_or(local_ip_parsed);
                                        let src_port = entry.src_port.parse::<u16>().unwrap_or(local_port);
                                        let dest_port = entry.dest_port.parse::<u16>().unwrap_or(local_port);

                                        let src_ip_v4 = match src_ip {
                                            std::net::IpAddr::V4(ip) => ip,
                                            _ => std::net::Ipv4Addr::new(127, 0, 0, 1),
                                        };
                                        let dest_ip_v4 = match dest_ip {
                                            std::net::IpAddr::V4(ip) => ip,
                                            _ => std::net::Ipv4Addr::new(127, 0, 0, 1),
                                        };

                                        let payload = &entry.data;
                                        let payload_len = payload.len();

                                        let mut packet_data = Vec::with_capacity(42 + payload_len);

                                        packet_data.extend_from_slice(&[0u8; 6]);
                                        packet_data.extend_from_slice(&[0u8; 6]);
                                        packet_data.extend_from_slice(&0x0800u16.to_be_bytes());

                                        packet_data.push(0x45);
                                        packet_data.push(0x00);
                                        let ip_total_len = (20 + 8 + payload_len) as u16;
                                        packet_data.extend_from_slice(&ip_total_len.to_be_bytes());
                                        packet_data.extend_from_slice(&0x0000u16.to_be_bytes());
                                        packet_data.extend_from_slice(&0x4000u16.to_be_bytes());
                                        packet_data.push(64);
                                        packet_data.push(17);
                                        
                                        let checksum_offset = packet_data.len();
                                        packet_data.extend_from_slice(&[0u8; 2]);

                                        packet_data.extend_from_slice(&src_ip_v4.octets());
                                        packet_data.extend_from_slice(&dest_ip_v4.octets());

                                        let mut sum = 0u32;
                                        for i in (14..34).step_by(2) {
                                            let word = ((packet_data[i] as u16) << 8) | (packet_data[i+1] as u16);
                                            sum += word as u32;
                                        }
                                        while sum >> 16 != 0 {
                                            sum = (sum & 0xffff) + (sum >> 16);
                                        }
                                        let checksum = !(sum as u16);
                                        packet_data[checksum_offset] = (checksum >> 8) as u8;
                                        packet_data[checksum_offset + 1] = (checksum & 0xff) as u8;

                                        packet_data.extend_from_slice(&src_port.to_be_bytes());
                                        packet_data.extend_from_slice(&dest_port.to_be_bytes());
                                        let udp_len = (8 + payload_len) as u16;
                                        packet_data.extend_from_slice(&udp_len.to_be_bytes());
                                        packet_data.extend_from_slice(&0x0000u16.to_be_bytes());

                                        packet_data.extend_from_slice(payload);

                                        let ts_sec = entry.timestamp.timestamp() as u32;
                                        let ts_usec = entry.timestamp.timestamp_subsec_micros() as u32;
                                        let cap_len = packet_data.len() as u32;
                                        let orig_len = packet_data.len() as u32;

                                        let _ = file.write_all(&ts_sec.to_ne_bytes());
                                        let _ = file.write_all(&ts_usec.to_ne_bytes());
                                        let _ = file.write_all(&cap_len.to_ne_bytes());
                                        let _ = file.write_all(&orig_len.to_ne_bytes());
                                        let _ = file.write_all(&packet_data);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let mra_db = mra::MraDatabase::load();

        let state = UdpStudioState {
            collections: config.collections,
            selected_request_id: None,
            composer_selected_collection_idx: 0,
            composer_ip: config.composer_ip,
            composer_port: config.composer_port,
            composer_ip_history: config.composer_ip_history,
            composer_port_history: config.composer_port_history,
            composer_payload_type: config.composer_payload_type,
            composer_payload: config.composer_payload,
            composer_name: String::new(),
            logs: Vec::new(),
            selected_log_idx: None,
            filter_text: String::new(),
            filter_input: String::new(),
            filter_history: Vec::new(),
            auto_scroll: true,
            log_export_format: LogExportFormat::Csv,
            filtered_indices: Vec::new(),
            sockets: config.sockets.iter().map(|s| crate::types::ActiveSocketState {
                id: s.id.clone(),
                name: s.name.clone(),
                ip: s.ip.clone(),
                port: s.port.clone(),
                is_listening: false,
                bound_addr: None,
                error: None,
                bind_time: None,
                multicast_groups: Vec::new(),
            }).collect(),
            selected_socket_id: config.selected_socket_id.clone(),
            multicast_selected_socket_id: config.selected_socket_id.clone(),
            listener_ip_history: config.listener_ip_history,
            listener_port_history: config.listener_port_history,
            udp_worker,
            rx_event,
            el_tid: "0001".to_string(),
            el_seoj: "05FF01".to_string(),
            el_deoj_preset: 0,
            el_deoj_custom: "013001".to_string(),
            el_deoj_eoj: "0130".to_string(),
            el_esv_preset: 0,
            el_properties: vec![ElBuilderProperty { epc: "80".to_string(), edt: String::new() }],
            el_show_helper: false,
            multicast_input_addr: "224.0.23.0".to_string(),
            multicast_input_interface: "0.0.0.0".to_string(),
            inspector_protocol: InspectorProtocol::Raw,
            auto_save_enabled: config.auto_save_enabled,
            auto_save_dir: config.auto_save_dir,
            auto_save_format: config.auto_save_format,
            max_display_data_bytes: config.max_display_data_bytes,
            max_log_lines: config.max_log_lines,
            settings_open: false,
            settings_reset_confirm_open: false,
            about_open: false,
            about_tab: AboutTab::Info,
            tx_logger,
            language_setting: config.language_setting,
            mra_db,
            dock_state_serialized: config.dock_state.clone(),
            reset_layout_requested: false,
        };

        Self {
            dock_state,
            state,
            was_focused: true,
        }
    }
}

fn minimize_window(ctx: &egui::Context) {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::{msg_send, sel, sel_impl};
        let ns_app: *mut objc::runtime::Object = msg_send![objc::class!(NSApplication), sharedApplication];
        if !ns_app.is_null() {
            let key_window: *mut objc::runtime::Object = msg_send![ns_app, keyWindow];
            if !key_window.is_null() {
                let _: () = msg_send![key_window, miniaturize: std::ptr::null_mut::<objc::runtime::Object>()];
                return;
            }
        }
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
}

// macOS style window control circles
#[derive(Debug, Clone, Copy)]
enum TrafficLightType {
    Close,
    Minimize,
    Maximize,
}

fn circle_button(
    ui: &mut egui::Ui,
    light_type: TrafficLightType,
    is_any_hovered: bool,
    is_focused: bool,
) -> egui::Response {
    let size = egui::vec2(12.0, 12.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response.hovered();
        let is_pressed = is_hovered && ui.input(|i| i.pointer.primary_down());
        
        let base_color = if !is_focused && !is_any_hovered {
            egui::Color32::from_rgb(60, 60, 60)
        } else {
            match light_type {
                TrafficLightType::Close => egui::Color32::from_rgb(255, 95, 86),
                TrafficLightType::Minimize => egui::Color32::from_rgb(255, 189, 46),
                TrafficLightType::Maximize => egui::Color32::from_rgb(39, 201, 63),
            }
        };
        
        let fill_color = if is_pressed {
            base_color.linear_multiply(0.7)
        } else if is_hovered {
            base_color.linear_multiply(0.85)
        } else {
            base_color
        };
        
        let border_color = if !is_focused && !is_any_hovered {
            egui::Color32::from_rgb(45, 45, 45)
        } else {
            match light_type {
                TrafficLightType::Close => egui::Color32::from_rgb(224, 76, 68),
                TrafficLightType::Minimize => egui::Color32::from_rgb(223, 159, 36),
                TrafficLightType::Maximize => egui::Color32::from_rgb(30, 163, 50),
            }
        };
        
        painter.circle_filled(rect.center(), 6.0, border_color);
        painter.circle_filled(rect.center(), 5.2, fill_color);
        
        if is_any_hovered {
            let glyph_color = egui::Color32::from_rgba_premultiplied(0, 0, 0, 180);
            let stroke = egui::Stroke::new(1.2, glyph_color);
            let center = rect.center();
            
            match light_type {
                TrafficLightType::Close => {
                    let d = 2.2;
                    painter.line_segment([center + egui::vec2(-d, -d), center + egui::vec2(d, d)], stroke);
                    painter.line_segment([center + egui::vec2(d, -d), center + egui::vec2(-d, d)], stroke);
                }
                TrafficLightType::Minimize => {
                    let w = 3.0;
                    painter.line_segment([center + egui::vec2(-w, 0.0), center + egui::vec2(w, 0.0)], stroke);
                }
                TrafficLightType::Maximize => {
                    let d = 2.2;
                    painter.line_segment([center + egui::vec2(-d, d), center + egui::vec2(d, -d)], stroke);
                    painter.line_segment([center + egui::vec2(d, -d), center + egui::vec2(d - 1.8, -d)], stroke);
                    painter.line_segment([center + egui::vec2(d, -d), center + egui::vec2(d, -d + 1.8)], stroke);
                    painter.line_segment([center + egui::vec2(-d, d), center + egui::vec2(-d + 1.8, d)], stroke);
                    painter.line_segment([center + egui::vec2(-d, d), center + egui::vec2(-d, d - 1.8)], stroke);
                }
            }
        }
    }
    response
}

impl eframe::App for MainApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.state.reset_layout_requested {
            self.state.reset_layout_requested = false;

            // Re-create default docking layout tree
            let mut ds = DockState::new(vec![Tab::Collections]);
            let surface = ds.main_surface_mut();
            let [_left, middle] = surface.split_right(egui_dock::NodeIndex::root(), 0.25, vec![Tab::LogViewer]);
            let [center, right] = surface.split_right(middle, 0.60, vec![Tab::Sender, Tab::Sockets]);
            let [_, _] = surface.split_below(center, 0.55, vec![Tab::Inspector]);
            let [_, _] = surface.split_below(right, 0.55, vec![Tab::Multicast]);
            self.dock_state = ds;

            // Serialize and save
            if let Ok(current_json) = serde_json::to_string(&self.dock_state) {
                self.state.dock_state_serialized = Some(current_json);
            }
            self.state.save_config();

            // Send ViewportCommands to reset size and exit maximization
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1100.0, 700.0)));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        }

        let is_focused = ui.ctx().input(|i| i.focused);
        if is_focused && !self.was_focused {
            ui.ctx().input_mut(|i| {
                i.events.push(egui::Event::PointerButton {
                    pos: egui::pos2(-1.0, -1.0),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                });
            });
            ui.ctx().request_repaint();
        }
        self.was_focused = is_focused;
        
        let ctx = ui.ctx().clone();
        
        // Handle all incoming events from the background UDP worker thread
        while let Ok(event) = self.state.rx_event.try_recv() {
            match event {
                 UdpEvent::Bound { id, addr } => {
                     if let Some(socket) = self.state.sockets.iter_mut().find(|s| s.id == id) {
                         socket.is_listening = true;
                         socket.bound_addr = Some(addr.to_string());
                         socket.bind_time = Some(chrono::Local::now());
                         socket.ip = addr.ip().to_string();
                         socket.port = addr.port().to_string();
                         socket.error = None;
                         let msg = format!("Listening socket '{}' bound to {}", socket.name, addr);
                         self.state.add_system_info(msg);
                     }
                     self.state.save_config();
                     self.state.update_logger_config();
                 }
                 UdpEvent::Unbound { id } => {
                     if let Some(socket) = self.state.sockets.iter_mut().find(|s| s.id == id) {
                         socket.is_listening = false;
                         socket.bound_addr = None;
                         socket.bind_time = None;
                         socket.multicast_groups.clear();
                         let msg = format!("Listening socket '{}' unbound", socket.name);
                         self.state.add_system_info(msg);
                     }
                     self.state.update_logger_config();
                 }
                UdpEvent::Sent { id: _, to, data, timestamp, local_addr } => {
                    let entry = LogEntry::new_with_local(timestamp, LogDirection::Sent, to, Some(local_addr), data);
                    self.state.push_log(entry);
                }
                UdpEvent::Received { id: _, from, data, timestamp, local_addr } => {
                    let entry = LogEntry::new_with_local(timestamp, LogDirection::Received, from, Some(local_addr), data);
                    self.state.push_log(entry);
                    ctx.request_repaint();
                }
                UdpEvent::Error { id, err } => {
                    if let Some(socket) = self.state.sockets.iter_mut().find(|s| s.id == id) {
                        socket.error = Some(err.clone());
                    }
                    self.state.add_system_error(err);
                }
                UdpEvent::MulticastJoined { id, multi_addr, interface_addr } => {
                    if let Some(socket) = self.state.sockets.iter_mut().find(|s| s.id == id) {
                        socket.multicast_groups.push(MulticastGroup {
                            multi_addr: multi_addr.clone(),
                            interface_addr: interface_addr.clone(),
                        });
                        let msg = format!("Socket '{}' joined multicast group {} on interface {}", socket.name, multi_addr, interface_addr);
                        self.state.add_system_info(msg);
                    }
                }
                UdpEvent::MulticastLeft { id, multi_addr, interface_addr } => {
                    if let Some(socket) = self.state.sockets.iter_mut().find(|s| s.id == id) {
                        socket.multicast_groups.retain(|g| !(g.multi_addr == *multi_addr && g.interface_addr == *interface_addr));
                        let msg = format!("Socket '{}' left multicast group {} on interface {}", socket.name, multi_addr, interface_addr);
                        self.state.add_system_info(msg);
                    }
                }
            }
        }

        // Outer window container with rounded corners (enabled by transparent viewport option)
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(13, 16, 21))
            .corner_radius(egui::CornerRadius::same(12))
            .show(ui, |ui| {
                // Force the frame to expand to fill the entire window area
                ui.expand_to_include_rect(ui.max_rect());

                // Set spacing between panel elements to zero so they align perfectly
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

                // Custom Title Bar Panel (Mac Style header with integrated socket listener setup)
                egui::Panel::top("custom_title_bar")
                    .frame(egui::Frame::default()
                        .fill(egui::Color32::from_rgb(9, 11, 14))
                        .corner_radius(egui::CornerRadius {
                            nw: 12,
                            ne: 12,
                            sw: 0,
                            se: 0,
                        })
                        .inner_margin(egui::Margin {
                            left: 16,
                            right: 16,
                            top: 14,
                            bottom: 14,
                        }))
                    .show(ui, |ui| {
                        // Title bar background drag/double-click action covering the entire bar area
                        let title_bar_rect = ui.max_rect();
                        let drag_resp = ui.interact(title_bar_rect, ui.id().with("title_bar_drag"), egui::Sense::click_and_drag());
                        if drag_resp.dragged_by(egui::PointerButton::Primary) {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        if drag_resp.double_clicked() {
                            let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }

                        ui.horizontal(|ui| {
                            // Traffic lights window controls
                            let is_focused = ui.ctx().input(|i| i.focused);
                            
                            // Detect if pointer is hovering over the traffic lights group.
                            // Use pointer_hover_pos() directly so we can build the rect *after*
                            // the inner horizontal() has allocated space, ensuring the Y range
                            // matches the actual rendered button positions (egui centers widgets
                            // vertically in a horizontal layout, so next_widget_position() alone
                            // gives the wrong Y origin).
                            let row_top = ui.next_widget_position().y;
                            let row_height = ui.available_height();
                            let row_x_start = ui.next_widget_position().x;
                            // Width: 12 * 3 buttons + 8 * 2 gaps = 52 px
                            let traffic_lights_rect = egui::Rect::from_min_max(
                                egui::pos2(row_x_start, row_top),
                                egui::pos2(row_x_start + 52.0, row_top + row_height),
                            );
                            let is_any_hovered = ui.ctx().pointer_hover_pos()
                                .map(|p| traffic_lights_rect.contains(p))
                                .unwrap_or(false);

                            // Allocate the interact region so egui still processes it correctly
                            let _area_response = ui.interact(
                                traffic_lights_rect,
                                ui.id().with("traffic_lights_area"),
                                egui::Sense::hover()
                            );

                            ui.horizontal(|ui| {
                                if circle_button(ui, TrafficLightType::Close, is_any_hovered, is_focused)
                                    .on_hover_text("Close")
                                    .clicked()
                                {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                ui.add_space(8.0);
                                if circle_button(ui, TrafficLightType::Minimize, is_any_hovered, is_focused)
                                    .on_hover_text("Minimize")
                                    .clicked()
                                {
                                    ui.ctx().input_mut(|i| {
                                        i.events.push(egui::Event::PointerButton {
                                            pos: egui::pos2(-1.0, -1.0),
                                            button: egui::PointerButton::Primary,
                                            pressed: false,
                                            modifiers: egui::Modifiers::default(),
                                        });
                                    });
                                    minimize_window(ui.ctx());
                                }
                                ui.add_space(8.0);
                                if circle_button(ui, TrafficLightType::Maximize, is_any_hovered, is_focused)
                                    .on_hover_text("Maximize")
                                    .clicked()
                                {
                                    let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                                }
                            });
                            
                            ui.add_space(15.0);
                            
                            // Application title
                            ui.strong(concat!("UDP Packet Studio v", env!("CARGO_PKG_VERSION")));
                            
                            ui.add_space(15.0);
                            ui.separator();
                            ui.add_space(15.0);
                            
                            // Dropdown for socket selection
                            let current_socket_name = self.state.get_selected_socket()
                                .map(|s| s.name.clone())
                                .unwrap_or_else(|| "Main Socket".to_string());
                            
                            let mut socket_changed = false;
                            egui::ComboBox::from_id_salt("navbar_socket_select")
                                .selected_text(&current_socket_name)
                                .width(130.0)
                                .show_ui(ui, |ui| {
                                    for socket in &self.state.sockets {
                                        if ui.selectable_value(&mut self.state.selected_socket_id, socket.id.clone(), &socket.name).clicked() {
                                            socket_changed = true;
                                        }
                                    }
                                });

                            if socket_changed {
                                self.state.save_config();
                                self.state.update_logger_config();
                            }



                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            // Bind controls for selected socket
                            if let Some(selected_idx) = self.state.sockets.iter().position(|s| s.id == self.state.selected_socket_id) {
                                self.state.show_socket_bind_controls(ui, selected_idx, true);
                            }
                            
                            // Align settings button to the right end of title bar
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.menu_button("⚙", |ui| {
                                    if ui.button(self.state.tr("titlebar-preferences")).clicked() {
                                        self.state.settings_open = true;
                                        ui.close();
                                    }
                                    if ui.button(self.state.tr("titlebar-about")).clicked() {
                                        self.state.about_open = true;
                                        self.state.about_tab = AboutTab::Info;
                                        ui.close();
                                    }
                                });
                            });
                        });
                    });

                // Bottom status bar panel
                egui::Panel::bottom("bottom_status_bar")
                    .frame(egui::Frame::default()
                        .fill(egui::Color32::from_rgb(9, 11, 14))
                        .corner_radius(egui::CornerRadius {
                            nw: 0,
                            ne: 0,
                            sw: 12,
                            se: 12,
                        })
                        .inner_margin(egui::Margin::symmetric(12, 6)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (is_listening, bound_addr) = if let Some(s) = self.state.get_selected_socket() {
                                (s.is_listening, s.bound_addr.clone())
                            } else {
                                (false, None)
                            };

                            if is_listening {
                                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), self.state.tr("titlebar-status-active"));
                                if let Some(ref addr) = bound_addr {
                                    let mut args = std::collections::HashMap::new();
                                    args.insert(std::borrow::Cow::Borrowed("addr"), addr.clone().into());
                                    ui.label(self.state.tr_with_args("statusbar-bound", &args));
                                }
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);
                                ui.colored_label(egui::Color32::from_rgb(140, 200, 255), self.state.tr("statusbar-broadcast"));
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), self.state.tr("titlebar-status-offline"));
                                ui.label(self.state.tr("statusbar-not-bound"));
                            }
                            
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);
                            
                            let auto_save_text = if self.state.auto_save_enabled {
                                let mut args = std::collections::HashMap::new();
                                args.insert(std::borrow::Cow::Borrowed("format"), format!("{:?}", self.state.auto_save_format).into());
                                self.state.tr_with_args("statusbar-auto-save-enabled", &args)
                            } else {
                                self.state.tr("statusbar-auto-save-disabled")
                            };
                            let auto_save_color = if self.state.auto_save_enabled {
                                egui::Color32::from_rgb(100, 255, 100)
                            } else {
                                egui::Color32::from_rgb(150, 150, 150)
                            };
                            
                            let label_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(auto_save_text)
                                        .color(auto_save_color)
                                )
                                .sense(egui::Sense::click())
                            );
                            
                            let label_resp = label_resp.on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text(self.state.tr("statusbar-auto-save-tip"));

                            if label_resp.clicked() {
                                self.state.auto_save_enabled = !self.state.auto_save_enabled;
                                self.state.save_config();
                                self.state.update_logger_config();
                            }
                            
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);
                            
                            let folder_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(self.state.tr("statusbar-open-log-dir"))
                                        .color(egui::Color32::from_rgb(140, 200, 255))
                                )
                                .sense(egui::Sense::click())
                            );
                            
                            let folder_resp = folder_resp.on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text(self.state.tr("statusbar-open-log-dir-tip"));

                            if folder_resp.clicked() {
                                let dir = &self.state.auto_save_dir;
                                if !dir.is_empty() {
                                    let path = std::path::Path::new(dir);
                                    let _ = std::fs::create_dir_all(path);
                                    
                                    #[cfg(target_os = "macos")]
                                    {
                                        let _ = std::process::Command::new("open")
                                            .arg(path)
                                            .spawn();
                                    }
                                    #[cfg(target_os = "windows")]
                                    {
                                        let _ = std::process::Command::new("explorer")
                                            .arg(path)
                                            .spawn();
                                    }
                                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                                    {
                                        let _ = std::process::Command::new("xdg-open")
                                            .arg(path)
                                            .spawn();
                                    }
                                }
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut args = std::collections::HashMap::new();
                                args.insert(std::borrow::Cow::Borrowed("count"), self.state.logs.len().into());
                                ui.label(self.state.tr_with_args("statusbar-logged-packets", &args));
                            });
                        });
                    });

                // Main docking control area inside central panel
                let mut viewer = MyTabViewer { state: &mut self.state };
                
                let mut dock_style = egui_dock::Style::from_egui(ui.style());
                
                // Customize tab bar background and padding to fit title bar style
                dock_style.tab_bar.bg_fill = egui::Color32::from_rgb(9, 11, 14);
                dock_style.tab_bar.height = 30.0;
                
                // Customize active tab style (matches panel background)
                dock_style.tab.active.bg_fill = egui::Color32::from_rgb(13, 16, 21);
                dock_style.tab.active.text_color = egui::Color32::from_rgb(255, 255, 255);
                dock_style.tab.active.outline_color = egui::Color32::from_rgb(33, 41, 54);
                
                // Customize inactive tab style
                dock_style.tab.inactive.bg_fill = egui::Color32::from_rgb(9, 11, 14);
                dock_style.tab.inactive.text_color = egui::Color32::from_rgb(130, 140, 155);
                dock_style.tab.inactive.outline_color = egui::Color32::from_rgb(9, 11, 14);
                
                // Customize hovered tab style
                dock_style.tab.hovered.bg_fill = egui::Color32::from_rgb(26, 33, 45);
                dock_style.tab.hovered.text_color = egui::Color32::from_rgb(255, 255, 255);
                dock_style.tab.hovered.outline_color = egui::Color32::from_rgb(33, 41, 54);
                
                // Customize focused tab style
                dock_style.tab.focused.bg_fill = egui::Color32::from_rgb(13, 16, 21);
                dock_style.tab.focused.text_color = egui::Color32::from_rgb(255, 255, 255);
                dock_style.tab.focused.outline_color = egui::Color32::from_rgb(79, 110, 242);
                
                dock_style.tab.active.corner_radius = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
                dock_style.tab.inactive.corner_radius = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
                dock_style.tab.hovered.corner_radius = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
                dock_style.tab.focused.corner_radius = egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };


                // Resizing separator styling
                dock_style.separator.width = 1.0;
                dock_style.separator.extra_interact_width = 4.0;
                
                DockArea::new(&mut self.dock_state)
                    .style(dock_style)
                    .show_close_buttons(false)
                    .draggable_tabs(false)
                    .tab_context_menus(false)
                    .show_inside(ui, &mut viewer);

                // Auto-save layout if changed
                if let Ok(current_json) = serde_json::to_string(&self.dock_state) {
                    let mut needs_save = false;
                    if let Some(ref last_json) = self.state.dock_state_serialized {
                        if *last_json != current_json {
                            needs_save = true;
                        }
                    } else {
                        needs_save = true;
                    }
                    if needs_save {
                        self.state.dock_state_serialized = Some(current_json);
                        self.state.save_config();
                    }
                }
            });

        // Draw the settings dialog if open
        if self.state.settings_open {
            let mut open = self.state.settings_open;
            let mut close_clicked = false;
            let mut reset_clicked = false;
            
            // Retrieve all translations upfront to satisfy borrow checker
            let settings_title = self.state.tr("settings-title");
            let lang_section = self.state.tr("settings-lang-section");
            let lang_label = self.state.tr("settings-lang-label");
            let selected_lang_text = self.state.language_setting.to_display_name();
            let system_display = LanguageSetting::System.to_display_name();
            let ja_display = LanguageSetting::Japanese.to_display_name();
            let en_display = LanguageSetting::English.to_display_name();
            
            let auto_save_section = self.state.tr("settings-auto-save-section");
            let auto_save_enable = self.state.tr("settings-auto-save-enable");
            let auto_save_format_label = self.state.tr("settings-auto-save-format");
            let auto_save_dir_label = self.state.tr("settings-auto-save-dir");
            let browse_btn_label = self.state.tr("settings-browse");
            let close_btn_label = self.state.tr("settings-close");
            let reset_btn_label = self.state.tr("settings-reset");
            
            let log_limit_section = self.state.tr("settings-log-limit-section");
            let max_display_bytes_label = self.state.tr("settings-max-display-bytes");
            let max_log_lines_label = self.state.tr("settings-max-log-lines");

            egui::Window::new(settings_title)
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.vertical(|ui| {
                        // Language Settings
                        ui.heading(lang_section);
                        ui.add_space(4.0);
                        
                        ui.horizontal(|ui| {
                            ui.label(lang_label);
                            
                            let combo_lang_res = egui::ComboBox::from_id_salt("settings_language")
                                .selected_text(selected_lang_text)
                                .show_ui(ui, |ui| {
                                    let mut changed = false;
                                    changed |= ui.selectable_value(&mut self.state.language_setting, LanguageSetting::System, system_display).changed();
                                    changed |= ui.selectable_value(&mut self.state.language_setting, LanguageSetting::Japanese, ja_display).changed();
                                    changed |= ui.selectable_value(&mut self.state.language_setting, LanguageSetting::English, en_display).changed();
                                    changed
                                });
                            if combo_lang_res.inner.unwrap_or(false) {
                                self.state.save_config();
                            }
                        });
                        
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // Log Auto-Save Settings
                        ui.heading(auto_save_section);
                        ui.add_space(4.0);
                        
                        let checkbox_res = ui.checkbox(&mut self.state.auto_save_enabled, auto_save_enable);
                        if checkbox_res.changed() {
                            self.state.save_config();
                            self.state.update_logger_config();
                        }
                        
                        ui.add_space(8.0);
                        
                        ui.label(auto_save_format_label);
                        let combo_res = egui::ComboBox::from_id_salt("settings_auto_save_format")
                            .selected_text(match self.state.auto_save_format {
                                LogExportFormat::Csv => "CSV",
                                LogExportFormat::Json => "JSON",
                                LogExportFormat::Pcap => "PCAP",
                            })
                            .show_ui(ui, |ui| {
                                let mut changed = false;
                                changed |= ui.selectable_value(&mut self.state.auto_save_format, LogExportFormat::Csv, "CSV").changed();
                                changed |= ui.selectable_value(&mut self.state.auto_save_format, LogExportFormat::Json, "JSON").changed();
                                changed |= ui.selectable_value(&mut self.state.auto_save_format, LogExportFormat::Pcap, "PCAP").changed();
                                changed
                            });
                        if combo_res.inner.unwrap_or(false) {
                            self.state.save_config();
                            self.state.update_logger_config();
                        }
                        
                        ui.add_space(8.0);
                        
                        ui.label(auto_save_dir_label);
                        ui.horizontal(|ui| {
                            let dir_res = ui.add(egui::TextEdit::singleline(&mut self.state.auto_save_dir).desired_width(300.0));
                            if dir_res.changed() {
                                self.state.save_config();
                                self.state.update_logger_config();
                            }
                            
                            if ui.button(browse_btn_label).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_directory(&self.state.auto_save_dir)
                                    .pick_folder()
                                {
                                    self.state.auto_save_dir = path.to_string_lossy().into_owned();
                                    self.state.save_config();
                                    self.state.update_logger_config();
                                }
                            }
                        });
                        
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // Log Limits Settings
                        ui.heading(log_limit_section);
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label(max_display_bytes_label);
                            let drag_res = ui.add(egui::DragValue::new(&mut self.state.max_display_data_bytes).range(1..=65536));
                            if drag_res.changed() {
                                self.state.save_config();
                            }
                        });

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.label(max_log_lines_label);
                            let drag_res = ui.add(egui::DragValue::new(&mut self.state.max_log_lines).range(1..=1000000));
                            if drag_res.changed() {
                                self.state.enforce_log_limits();
                                self.state.save_config();
                            }
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // Layout Settings
                        ui.heading(self.state.tr("settings-layout-section"));
                        ui.add_space(4.0);
                        if ui.button(self.state.tr("settings-reset-layout-btn")).clicked() {
                            self.state.reset_layout_requested = true;
                        }
                        
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(close_btn_label).clicked() {
                                close_clicked = true;
                            }
                            
                            let reset_btn = ui.add(egui::Button::new(egui::RichText::new(reset_btn_label).color(egui::Color32::from_rgb(255, 100, 100))));
                            if reset_btn.clicked() {
                                reset_clicked = true;
                            }
                        });
                    });
                });
            if reset_clicked {
                self.state.settings_reset_confirm_open = true;
            }
            self.state.settings_open = open && !close_clicked;
        }

        // Draw the settings reset confirmation dialog if open
        if self.state.settings_reset_confirm_open {
            let mut open = self.state.settings_reset_confirm_open;
            let mut ok_clicked = false;
            let mut cancel_clicked = false;

            let title = self.state.tr("settings-reset-confirm-title");
            let msg = self.state.tr("settings-reset-confirm-msg");
            let ok_label = self.state.tr("settings-ok");
            let cancel_label = self.state.tr("settings-cancel");

            egui::Window::new(title)
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.label(msg);
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui.button(ok_label).clicked() {
                                ok_clicked = true;
                            }
                            if ui.button(cancel_label).clicked() {
                                cancel_clicked = true;
                            }
                        });
                    });
                });

            if ok_clicked {
                self.state.reset_settings();
                self.state.settings_reset_confirm_open = false;
                self.state.settings_open = false;
                
                if let Ok(exe_path) = std::env::current_exe() {
                    let mut cmd = std::process::Command::new(exe_path);
                    let args: Vec<String> = std::env::args().collect();
                    if args.len() > 1 {
                        cmd.args(&args[1..]);
                    }
                    let _ = cmd.spawn();
                }
                std::process::exit(0);
            }

            if cancel_clicked || !open {
                self.state.settings_reset_confirm_open = false;
            }
        }

        // Draw the About dialog if open
        if self.state.about_open {
            let mut open = self.state.about_open;
            let mut close_clicked = false;

            let about_title = self.state.tr("about-title");
            let about_desc = self.state.tr("about-desc");
            let about_license_lbl = self.state.tr("about-license-label");
            let about_show_oss = self.state.tr("about-show-oss");
            let about_oss_title = self.state.tr("about-oss-title");
            let about_oss_desc = self.state.tr("about-oss-description");
            let about_back = self.state.tr("about-back");
            let close_btn_label = self.state.tr("settings-close");

            egui::Window::new(about_title)
                .open(&mut open)
                .resizable(true)
                .collapsible(false)
                .default_size(egui::vec2(550.0, 450.0))
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.vertical(|ui| {
                        match self.state.about_tab {
                            AboutTab::Info => {
                                ui.vertical_centered(|ui| {
                                    ui.heading("UDP Packet Studio");
                                    ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
                                    ui.add_space(8.0);
                                    ui.label(about_desc);
                                    ui.add_space(12.0);
                                });

                                ui.label(about_license_lbl);
                                ui.add_space(4.0);
                                
                                let mut license_text = include_str!("../LICENSE.md").to_string();
                                egui::ScrollArea::vertical()
                                    .max_height(180.0)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut license_text)
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .interactive(false)
                                        );
                                    });

                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    if ui.button(about_show_oss).clicked() {
                                        self.state.about_tab = AboutTab::ThirdParty;
                                    }
                                });
                            }
                            AboutTab::ThirdParty => {
                                ui.heading(about_oss_title);
                                ui.add_space(4.0);
                                ui.label(about_oss_desc);
                                ui.add_space(8.0);

                                egui::ScrollArea::vertical()
                                    .max_height(250.0)
                                    .show(ui, |ui| {
                                        ui.collapsing("eframe / egui (MIT or Apache-2.0)", |ui| {
                                            ui.label("Licensed under the MIT License or Apache License, Version 2.0.");
                                            ui.add_space(4.0);
                                            ui.small(
"MIT License\n\
\n\
Copyright (c) 2018-2024 Emil Ernerfeldt\n\
\n\
Permission is hereby granted, free of charge, to any person obtaining a copy\n\
of this software and associated documentation files (the \"Software\"), to deal\n\
in the Software without restriction, including without limitation the rights\n\
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n\
copies of the Software, and to permit persons to whom the Software is\n\
furnished to do so, subject to the following conditions:\n\
\n\
The above copyright notice and this permission notice shall be included in all\n\
copies or substantial portions of the Software."
                                            );
                                        });

                                        ui.collapsing("egui_dock (MIT or Apache-2.0)", |ui| {
                                            ui.label("Licensed under the MIT License or Apache License, Version 2.0.");
                                            ui.add_space(4.0);
                                            ui.small(
"MIT License\n\
\n\
Copyright (c) 2022-2024 anhosh\n\
\n\
Permission is hereby granted, free of charge, to any person obtaining a copy\n\
of this software and associated documentation files (the \"Software\"), to deal\n\
in the Software without restriction, including without limitation the rights\n\
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n\
copies of the Software, and to permit persons to whom the Software is\n\
furnished to do so, subject to the following conditions:\n\
\n\
The above copyright notice and this permission notice shall be included in all\n\
copies or substantial portions of the Software."
                                            );
                                        });

                                        ui.collapsing("serde / serde_json / serde_yaml (MIT or Apache-2.0)", |ui| {
                                            ui.label("Licensed under the MIT License or Apache License, Version 2.0.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright (c) 2017 Erick Tryzelaar and David Tolnay");
                                        });

                                        ui.collapsing("rfd (MIT)", |ui| {
                                            ui.label("Licensed under the MIT License.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright (c) 2020 Szymon Lipiński");
                                        });

                                        ui.collapsing("chrono (MIT or Apache-2.0)", |ui| {
                                            ui.label("Licensed under the MIT License or Apache License, Version 2.0.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright (c) 2014, Kang Seonghoon");
                                        });

                                        ui.collapsing("dirs (MIT or Apache-2.0)", |ui| {
                                            ui.label("Licensed under the MIT License or Apache License, Version 2.0.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright (c) 2018 dirs-rs contributors");
                                        });

                                        ui.collapsing("Noto Sans JP (OFL-1.1)", |ui| {
                                            ui.label("Licensed under the SIL Open Font License, Version 1.1.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'. Noto is a trademark of Google Inc.");
                                        });

                                        ui.collapsing("Font Awesome 6 Free (Solid) (OFL-1.1)", |ui| {
                                            ui.label("Licensed under the SIL Open Font License, Version 1.1.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright Fonticons, Inc.");
                                        });

                                        ui.collapsing("Noto Sans Symbols 2 (OFL-1.1)", |ui| {
                                            ui.label("Licensed under the SIL Open Font License, Version 1.1.");
                                            ui.add_space(4.0);
                                            ui.small("Copyright 2022 The Noto Project Authors (https://github.com/notofonts/symbols)");
                                        });
                                    });

                                ui.add_space(12.0);
                                if ui.button(about_back).clicked() {
                                    self.state.about_tab = AboutTab::Info;
                                }
                            }
                        }

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(close_btn_label).clicked() {
                                close_clicked = true;
                            }
                        });
                    });
                });
            self.state.about_open = open && !close_clicked;
        }



        show_resize_handles(ui);
    }
}

pub fn show_resize_handles(ui: &mut egui::Ui) {
    use egui::viewport::ResizeDirection;
    use egui::{Sense, Rect, pos2, CursorIcon, ViewportCommand};

    let rect = ui.ctx().viewport_rect();
    let border = 6.0;
    let corner = 12.0;

    struct ResizeZone {
        rect: Rect,
        direction: ResizeDirection,
        cursor: CursorIcon,
    }

    let zones = [
        // Corners
        ResizeZone {
            rect: Rect::from_min_max(rect.left_top(), pos2(rect.left() + corner, rect.top() + corner)),
            direction: ResizeDirection::NorthWest,
            cursor: CursorIcon::ResizeNwSe,
        },
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.right() - corner, rect.top()), pos2(rect.right(), rect.top() + corner)),
            direction: ResizeDirection::NorthEast,
            cursor: CursorIcon::ResizeNeSw,
        },
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.left(), rect.bottom() - corner), pos2(rect.left() + corner, rect.bottom())),
            direction: ResizeDirection::SouthWest,
            cursor: CursorIcon::ResizeNeSw,
        },
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.right() - corner, rect.bottom() - corner), rect.right_bottom()),
            direction: ResizeDirection::SouthEast,
            cursor: CursorIcon::ResizeNwSe,
        },
        // Edges
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.left() + corner, rect.top()), pos2(rect.right() - corner, rect.top() + border)),
            direction: ResizeDirection::North,
            cursor: CursorIcon::ResizeVertical,
        },
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.left() + corner, rect.bottom() - border), pos2(rect.right() - corner, rect.bottom())),
            direction: ResizeDirection::South,
            cursor: CursorIcon::ResizeVertical,
        },
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.left(), rect.top() + corner), pos2(rect.left() + border, rect.bottom() - corner)),
            direction: ResizeDirection::West,
            cursor: CursorIcon::ResizeHorizontal,
        },
        ResizeZone {
            rect: Rect::from_min_max(pos2(rect.right() - border, rect.top() + corner), pos2(rect.right(), rect.bottom() - corner)),
            direction: ResizeDirection::East,
            cursor: CursorIcon::ResizeHorizontal,
        },
    ];

    for zone in zones.iter() {
        let response = ui.allocate_rect(zone.rect, Sense::drag());

        if response.hovered() {
            ui.ctx().set_cursor_icon(zone.cursor);
        }
        if response.dragged() {
            ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(zone.direction));
        }
    }
}

pub fn run() -> eframe::Result<()> {
    let icon_bytes = include_bytes!("icon.rgba");
    let icon = egui::IconData {
        rgba: icon_bytes.to_vec(),
        width: 64,
        height: 64,
    };

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(concat!("UDP Packet Studio v", env!("CARGO_PKG_VERSION")))
            .with_inner_size([1100.0, 700.0])
            .with_resizable(true)
            .with_decorations(false) // borderless window
            .with_transparent(true)
            .with_icon(icon),
        ..Default::default()
    };
    
    eframe::run_native(
        concat!("UDP Packet Studio v", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|cc| Ok(Box::new(MainApp::new(cc)))),
    )
}

