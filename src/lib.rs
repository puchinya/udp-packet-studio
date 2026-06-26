#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod udp_worker;
pub mod types;
pub mod config;
pub mod styling;
pub mod views;

use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender, channel};
use chrono::Local;
use eframe::egui;
use egui_dock::{DockArea, DockState};

use udp_worker::{UdpWorker, UdpCommand, UdpEvent};
use types::{Tab, LogEntry, LogDirection, PayloadType, parse_hex_to_bytes, Collection, MulticastGroup, InspectorProtocol, LogExportFormat, LoggerCommand};
use config::SavedConfig;
use styling::setup_custom_styles;

pub struct UdpStudioState {
    pub collections: Vec<Collection>,
    pub selected_request_id: Option<String>,
    pub composer_selected_collection_idx: usize,
    
    // Composer tab inputs
    pub composer_target: String,
    pub composer_payload_type: PayloadType,
    pub composer_payload: String,
    pub composer_name: String,
    
    // Logs tab inputs
    pub logs: Vec<LogEntry>,
    pub selected_log_idx: Option<usize>,
    pub filter_text: String,
    pub auto_scroll: bool,
    pub log_export_format: LogExportFormat,
    pub filtered_indices: Vec<usize>,
    
    // Listener settings
    pub listener_addr: String,
    pub is_listening: bool,
    pub bound_addr: Option<String>,
    pub listener_error: Option<String>,
    
    // Channels & Worker
    pub udp_worker: UdpWorker,
    pub rx_event: Receiver<UdpEvent>,

    // ECHONET Lite Helper state
    pub el_tid: String,
    pub el_seoj: String,
    pub el_deoj_preset: usize,
    pub el_deoj_custom: String,
    pub el_esv_preset: usize,
    pub el_epc_preset: usize,
    pub el_epc_custom: String,
    pub el_edt: String,
    pub el_show_helper: bool,

    // Multicast fields
    pub multicast_groups: Vec<MulticastGroup>,
    pub multicast_input_addr: String,
    pub multicast_input_interface: String,

    // Inspector fields
    pub inspector_protocol: InspectorProtocol,

    // Auto-save logger fields
    pub auto_save_enabled: bool,
    pub auto_save_dir: String,
    pub auto_save_format: LogExportFormat,
    pub settings_open: bool,
    pub tx_logger: Sender<LoggerCommand>,
}

impl UdpStudioState {
    pub(crate) fn save_config(&self) {
        let config = SavedConfig {
            collections: self.collections.clone(),
            listener_addr: self.listener_addr.clone(),
            composer_target: self.composer_target.clone(),
            composer_payload_type: self.composer_payload_type,
            composer_payload: self.composer_payload.clone(),
            auto_save_enabled: self.auto_save_enabled,
            auto_save_dir: self.auto_save_dir.clone(),
            auto_save_format: self.auto_save_format,
        };
        config.save();
    }

    pub(crate) fn update_logger_config(&self) {
        let _ = self.tx_logger.send(LoggerCommand::Configure {
            enabled: self.auto_save_enabled,
            dir: self.auto_save_dir.clone(),
            format: self.auto_save_format,
            listener_addr: self.listener_addr.clone(),
        });
    }

    pub(crate) fn push_log(&mut self, entry: LogEntry) {
        self.logs.push(entry.clone());
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
        self.filtered_indices = self.logs
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if self.filter_text.is_empty() {
                    return true;
                }
                entry.address_str.contains(&self.filter_text)
            })
            .map(|(idx, _)| idx)
            .collect();
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
                    target: target.to_string(),
                    data,
                });
            }
            Err(e) => {
                self.add_system_error(format!("Hex parsing error: {}", e));
            }
        }
    }
}

struct MyTabViewer<'a> {
    state: &'a mut UdpStudioState,
}

impl<'a> egui_dock::TabViewer for MyTabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::Collections => "📁 Collections".into(),
            Tab::Sender => "🚀 Composer".into(),
            Tab::LogViewer => "📊 Logs".into(),
            Tab::Inspector => "🔍 Inspector".into(),
            Tab::Multicast => "🌐 Multicast".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Collections => self.state.show_collections(ui),
            Tab::Sender => self.state.show_sender(ui),
            Tab::LogViewer => self.state.show_log_viewer(ui),
            Tab::Inspector => self.state.show_inspector(ui),
            Tab::Multicast => self.state.show_multicast(ui),
        }
    }
}

struct MainApp {
    dock_state: DockState<Tab>,
    state: UdpStudioState,
}

impl MainApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_styles(&cc.egui_ctx);
        
        let config = SavedConfig::load();
        
        // Setup initial docking layout tree (3-column split where center is stacked)
        let mut dock_state = DockState::new(vec![Tab::Collections]);
        let surface = dock_state.main_surface_mut();
        
        // Split right to place LogViewer in the middle
        let [_left, middle] = surface.split_right(egui_dock::NodeIndex::root(), 0.25, vec![Tab::LogViewer]);
        // Split middle to place Sender on the right. 'center' now points to the leaf node with Tab::LogViewer.
        let [center, right] = surface.split_right(middle, 0.60, vec![Tab::Sender]);
        // Split center below to place Inspector at the bottom of the center column
        let [_, _] = surface.split_below(center, 0.55, vec![Tab::Inspector]);
        // Split right below to place Multicast at the bottom of the right column
        let [_, _] = surface.split_below(right, 0.55, vec![Tab::Multicast]);
        
        let (tx_event, rx_event) = channel();
        let udp_worker = UdpWorker::spawn(tx_event, cc.egui_ctx.clone());
        
        let (tx_logger, rx_logger) = channel();
        let init_enabled = config.auto_save_enabled;
        let init_dir = config.auto_save_dir.clone();
        let init_format = config.auto_save_format;
        let init_addr = config.listener_addr.clone();
        
        std::thread::spawn(move || {
            let mut enabled = init_enabled;
            let mut dir = init_dir;
            let mut format = init_format;
            let mut listener_addr = init_addr;
            
            while let Ok(cmd) = rx_logger.recv() {
                match cmd {
                    LoggerCommand::Configure { enabled: e, dir: d, format: f, listener_addr: addr } => {
                        enabled = e;
                        dir = d;
                        format = f;
                        listener_addr = addr;
                    }
                    LoggerCommand::Log(entry) => {
                        if enabled && !dir.is_empty() {
                            let date_str = entry.timestamp.format("%Y-%m-%d").to_string();
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
                                            let _ = writeln!(file, "Timestamp,Direction,IP,Port,Length,DataHex,DataText");
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
                                        let _ = writeln!(file, "\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"",
                                            time_str, dir_str, entry.ip, entry.port, len_str, hex_str, plain_str);
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
                                        
                                        let local_ip = listener_addr.split(':').next().unwrap_or("127.0.0.1");
                                        let local_ip_parsed = local_ip.parse::<std::net::IpAddr>().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
                                        let local_port = listener_addr.split(':').nth(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(9000);
                                        
                                        let src_addr = match entry.direction {
                                            LogDirection::Received => entry.address,
                                            LogDirection::Sent => std::net::SocketAddr::new(local_ip_parsed, local_port),
                                            _ => continue,
                                        };
                                        let dest_addr = match entry.direction {
                                            LogDirection::Received => std::net::SocketAddr::new(local_ip_parsed, local_port),
                                            LogDirection::Sent => entry.address,
                                            _ => continue,
                                        };

                                        let src_ip = match src_addr.ip() {
                                            std::net::IpAddr::V4(ip) => ip,
                                            _ => std::net::Ipv4Addr::new(127, 0, 0, 1),
                                        };
                                        let dest_ip = match dest_addr.ip() {
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

                                        packet_data.extend_from_slice(&src_ip.octets());
                                        packet_data.extend_from_slice(&dest_ip.octets());

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

                                        packet_data.extend_from_slice(&src_addr.port().to_be_bytes());
                                        packet_data.extend_from_slice(&dest_addr.port().to_be_bytes());
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

        let state = UdpStudioState {
            collections: config.collections,
            selected_request_id: None,
            composer_selected_collection_idx: 0,
            composer_target: config.composer_target,
            composer_payload_type: config.composer_payload_type,
            composer_payload: config.composer_payload,
            composer_name: String::new(),
            logs: Vec::new(),
            selected_log_idx: None,
            filter_text: String::new(),
            auto_scroll: true,
            log_export_format: LogExportFormat::Csv,
            filtered_indices: Vec::new(),
            listener_addr: config.listener_addr,
            is_listening: false,
            bound_addr: None,
            listener_error: None,
            udp_worker,
            rx_event,
            el_tid: "0001".to_string(),
            el_seoj: "05FF01".to_string(),
            el_deoj_preset: 0,
            el_deoj_custom: "013001".to_string(),
            el_esv_preset: 0,
            el_epc_preset: 0,
            el_epc_custom: "80".to_string(),
            el_edt: "30".to_string(),
            el_show_helper: false,
            multicast_groups: Vec::new(),
            multicast_input_addr: "224.0.23.0".to_string(),
            multicast_input_interface: "0.0.0.0".to_string(),
            inspector_protocol: InspectorProtocol::Raw,
            auto_save_enabled: config.auto_save_enabled,
            auto_save_dir: config.auto_save_dir,
            auto_save_format: config.auto_save_format,
            settings_open: false,
            tx_logger,
        };

        Self {
            dock_state,
            state,
        }
    }
}

// macOS style window control circles
fn circle_button(ui: &mut egui::Ui, color: egui::Color32) -> egui::Response {
    let size = egui::vec2(12.0, 12.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let is_hovered = response.hovered();
        let is_pressed = is_hovered && ui.input(|i| i.pointer.primary_down());
        
        let fill_color = if is_pressed {
            color.linear_multiply(0.7)
        } else if is_hovered {
            color.linear_multiply(0.9)
        } else {
            color
        };
        
        painter.circle_filled(rect.center(), 6.0, fill_color);
    }
    response
}

impl eframe::App for MainApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        
        // Handle all incoming events from the background UDP worker thread
        while let Ok(event) = self.state.rx_event.try_recv() {
            match event {
                UdpEvent::Bound(addr) => {
                    self.state.is_listening = true;
                    self.state.bound_addr = Some(addr.to_string());
                    self.state.listener_addr = addr.to_string();
                    self.state.listener_error = None;
                    self.state.add_system_info(format!("Listening socket bound to {}", addr));
                    self.state.save_config();
                    self.state.update_logger_config();
                }
                UdpEvent::Unbound => {
                    self.state.is_listening = false;
                    self.state.bound_addr = None;
                    self.state.multicast_groups.clear();
                    self.state.add_system_info("Listening socket unbound".to_string());
                }
                UdpEvent::Sent { to, data, timestamp } => {
                    let entry = LogEntry::new(timestamp, LogDirection::Sent, to, data);
                    self.state.push_log(entry);
                }
                UdpEvent::Received { from, data, timestamp } => {
                    let entry = LogEntry::new(timestamp, LogDirection::Received, from, data);
                    self.state.push_log(entry);
                    ctx.request_repaint();
                }
                UdpEvent::Error(err_msg) => {
                    if !self.state.is_listening {
                        self.state.listener_error = Some(err_msg.clone());
                        self.state.multicast_groups.clear();
                    }
                    self.state.add_system_error(err_msg);
                }
                UdpEvent::MulticastJoined { multi_addr, interface_addr } => {
                    self.state.multicast_groups.push(MulticastGroup {
                        multi_addr: multi_addr.clone(),
                        interface_addr: interface_addr.clone(),
                    });
                    self.state.add_system_info(format!("Joined multicast group {} on interface {}", multi_addr, interface_addr));
                }
                UdpEvent::MulticastLeft { multi_addr, interface_addr } => {
                    self.state.multicast_groups.retain(|g| !(g.multi_addr == *multi_addr && g.interface_addr == *interface_addr));
                    self.state.add_system_info(format!("Left multicast group {} on interface {}", multi_addr, interface_addr));
                }
            }
        }

        // Outer window container with rounded corners (enabled by transparent viewport option)
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(20, 24, 30))
            .corner_radius(egui::CornerRadius::same(12))
            .show(ui, |ui| {
                // Force the frame to expand to fill the entire window area
                ui.expand_to_include_rect(ui.max_rect());

                // Set spacing between panel elements to zero so they align perfectly
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

                // Custom Title Bar Panel (Mac Style header with integrated socket listener setup)
                egui::Panel::top("custom_title_bar")
                    .frame(egui::Frame::default()
                        .fill(egui::Color32::from_rgb(15, 18, 22))
                        .corner_radius(egui::CornerRadius {
                            nw: 12,
                            ne: 12,
                            sw: 0,
                            se: 0,
                        })
                        .inner_margin(egui::Margin::symmetric(12, 8)))
                    .show_inside(ui, |ui| {
                        // Title bar background drag/double-click action covering the entire bar area
                        let title_bar_rect = ui.max_rect();
                        let drag_resp = ui.interact(title_bar_rect, ui.id().with("title_bar_drag"), egui::Sense::drag());
                        if drag_resp.dragged_by(egui::PointerButton::Primary) {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        if drag_resp.double_clicked() {
                            let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }

                        ui.horizontal(|ui| {
                            // Traffic lights window controls
                            ui.horizontal(|ui| {
                                if circle_button(ui, egui::Color32::from_rgb(255, 95, 86)).on_hover_text("Close").clicked() {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                ui.add_space(2.0);
                                if circle_button(ui, egui::Color32::from_rgb(255, 189, 46)).on_hover_text("Minimize").clicked() {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                                }
                                ui.add_space(2.0);
                                if circle_button(ui, egui::Color32::from_rgb(39, 201, 63)).on_hover_text("Maximize").clicked() {
                                    let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                                }
                            });
                            
                            ui.add_space(15.0);
                            
                            // Application title
                            ui.strong(concat!("UDP Packet Studio v", env!("CARGO_PKG_VERSION")));
                            
                            ui.add_space(20.0);
                            ui.separator();
                            ui.add_space(15.0);
                            
                            // Integrated Socket Bind Controls
                            ui.label("Bind Address:");
                            let text_res = ui.add(egui::TextEdit::singleline(&mut self.state.listener_addr).desired_width(130.0));
                            if text_res.changed() {
                                self.state.save_config();
                            }
                            
                            ui.add_space(5.0);
                            
                            if self.state.is_listening {
                                if ui.button("⏹ Stop").clicked() {
                                    self.state.udp_worker.send(UdpCommand::Unbind);
                                }
                                ui.add_space(5.0);
                                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "🟢 Active");
                                if let Some(ref addr) = self.state.bound_addr {
                                    ui.label(format!("({})", addr));
                                }
                            } else {
                                if ui.button("▶ Bind").clicked() {
                                    self.state.listener_error = None;
                                    self.state.udp_worker.send(UdpCommand::Bind(self.state.listener_addr.clone()));
                                }
                                ui.add_space(5.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), "🔴 Offline");
                            }
                            
                            if let Some(ref err) = self.state.listener_error {
                                ui.add_space(10.0);
                                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), format!("⚠️ {}", err));
                            }
                            
                            // Align settings button to the right end of title bar
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("⚙ Settings").clicked() {
                                    self.state.settings_open = true;
                                }
                            });
                        });
                    });

                // Bottom status bar panel
                egui::Panel::bottom("bottom_status_bar")
                    .frame(egui::Frame::default()
                        .fill(egui::Color32::from_rgb(15, 18, 22))
                        .corner_radius(egui::CornerRadius {
                            nw: 0,
                            ne: 0,
                            sw: 12,
                            se: 12,
                        })
                        .inner_margin(egui::Margin::symmetric(12, 6)))
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            if self.state.is_listening {
                                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "🟢 Active");
                                if let Some(ref addr) = self.state.bound_addr {
                                    ui.label(format!("Bound: {}", addr));
                                }
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);
                                ui.colored_label(egui::Color32::from_rgb(140, 200, 255), "📣 Broadcast Enabled");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(255, 90, 90), "🔴 Offline");
                                ui.label("Socket not bound");
                            }
                            
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);
                            if self.state.auto_save_enabled {
                                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), format!("💾 Auto-Save: Enabled ({:?})", self.state.auto_save_format));
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(150, 150, 150), "💾 Auto-Save: Disabled");
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("Logged packets: {}", self.state.logs.len()));
                            });
                        });
                    });

                // Main docking control area inside central panel
                let mut viewer = MyTabViewer { state: &mut self.state };
                DockArea::new(&mut self.dock_state)
                    .show_inside(ui, &mut viewer);
            });

        // Draw the settings dialog if open
        if self.state.settings_open {
            let mut open = self.state.settings_open;
            let mut close_clicked = false;
            egui::Window::new("⚙ Settings")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Log Auto-Save Settings");
                        ui.add_space(4.0);
                        
                        let checkbox_res = ui.checkbox(&mut self.state.auto_save_enabled, "Enable log auto-save");
                        if checkbox_res.changed() {
                            self.state.save_config();
                            self.state.update_logger_config();
                        }
                        
                        ui.add_space(8.0);
                        
                        ui.label("Log Format:");
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
                        
                        ui.label("Save Directory:");
                        ui.horizontal(|ui| {
                            let dir_res = ui.add(egui::TextEdit::singleline(&mut self.state.auto_save_dir).desired_width(300.0));
                            if dir_res.changed() {
                                self.state.save_config();
                                self.state.update_logger_config();
                            }
                            
                            if ui.button("📁 Browse...").clicked() {
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
                        
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                close_clicked = true;
                            }
                        });
                    });
                });
            self.state.settings_open = open && !close_clicked;
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
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(concat!("UDP Packet Studio v", env!("CARGO_PKG_VERSION")))
            .with_inner_size([1100.0, 700.0])
            .with_resizable(true)
            .with_decorations(false) // borderless window
            .with_transparent(true),
        ..Default::default()
    };
    
    eframe::run_native(
        concat!("UDP Packet Studio v", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|cc| Ok(Box::new(MainApp::new(cc)))),
    )
}

