use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::UdpStudioState;
use crate::types::{LogDirection, LogEntry, LogExportFormat};

impl UdpStudioState {
    pub fn show_log_viewer(&mut self, ui: &mut egui::Ui) {
        let mut new_selection = self.selected_log_idx;
        
        ui.vertical(|ui| {
            // Header toolbar
            ui.horizontal(|ui| {
                if ui.button("🗑 Clear").clicked() {
                    self.logs.clear();
                    self.filtered_indices.clear();
                    new_selection = None;
                }

                ui.add_space(8.0);
                ui.label("Format:");
                egui::ComboBox::from_id_salt("log_export_format")
                    .selected_text(match self.log_export_format {
                        LogExportFormat::Csv => "CSV",
                        LogExportFormat::Json => "JSON",
                        LogExportFormat::Pcap => "PCAP",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.log_export_format, LogExportFormat::Csv, "CSV");
                        ui.selectable_value(&mut self.log_export_format, LogExportFormat::Json, "JSON");
                        ui.selectable_value(&mut self.log_export_format, LogExportFormat::Pcap, "PCAP");
                    });

                let mut save_logs_trigger = false;
                if ui.button("💾 Save Logs").on_hover_text("Export logs to selected format").clicked() {
                    save_logs_trigger = true;
                }

                if save_logs_trigger {
                    let mut dialog = rfd::FileDialog::new()
                        .set_file_name("communication_logs");
                    
                    dialog = match self.log_export_format {
                        LogExportFormat::Csv => dialog.add_filter("CSV File (*.csv)", &["csv"]),
                        LogExportFormat::Json => dialog.add_filter("JSON File (*.json)", &["json"]),
                        LogExportFormat::Pcap => dialog.add_filter("PCAP File (*.pcap)", &["pcap"]),
                    };

                    if let Some(path) = dialog.save_file() {
                        let extension = match self.log_export_format {
                            LogExportFormat::Csv => "csv",
                            LogExportFormat::Json => "json",
                            LogExportFormat::Pcap => "pcap",
                        };
                        let path = if path.extension().map(|e| e.to_ascii_lowercase()) != Some(std::ffi::OsString::from(extension)) {
                            path.with_extension(extension)
                        } else {
                            path
                        };

                        let result = match self.log_export_format {
                            LogExportFormat::Json => {
                                match serde_json::to_string_pretty(&self.logs) {
                                    Ok(json_str) => std::fs::write(&path, json_str),
                                    Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, format!("JSON Serialization Error: {}", e))),
                                }
                            }
                            LogExportFormat::Pcap => {
                                write_pcap_helper(&path, &self.logs, &self.listener_addr)
                            }
                            LogExportFormat::Csv => {
                                // Default to CSV
                                let mut csv_content = String::new();
                                csv_content.push_str("No,Timestamp,Direction,IP,Port,Length,DataHex,DataText\n");
                                for (idx, entry) in self.logs.iter().enumerate() {
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
                                    csv_content.push_str(&format!("{},\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"\n", 
                                        idx + 1, time_str, dir_str, entry.ip, entry.port, len_str, hex_str, plain_str));
                                }
                                std::fs::write(&path, csv_content)
                            }
                        };

                        match result {
                            Ok(_) => {
                                self.add_system_info(format!("Logs saved successfully to {}", path.display()));
                            }
                            Err(e) => {
                                self.add_system_error(format!("Failed to save logs: {}", e));
                            }
                        }
                    }
                }
                
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                
                ui.add_space(15.0);
                ui.label("IP Filter:");
                if ui.text_edit_singleline(&mut self.filter_text).changed() {
                    self.update_filtered_indices();
                }
            });
            
            ui.separator();

            let filtered_indices = &self.filtered_indices;

            let mut table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .sense(egui::Sense::click()) // Add click sense to enable selection!
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(45.0))  // No.
                .column(Column::exact(100.0)) // Time
                .column(Column::exact(80.0))  // Type
                .column(Column::exact(110.0)) // IP Address
                .column(Column::exact(55.0))  // Port
                .column(Column::exact(60.0))  // Length
                .column(Column::remainder());  // Info/Payload

            table = table.stick_to_bottom(self.auto_scroll);

            table
                .header(24.0, |mut header| {
                    header.col(|ui| { ui.strong("No."); });
                    header.col(|ui| { ui.strong("Time"); });
                    header.col(|ui| { ui.strong("Type"); });
                    header.col(|ui| { ui.strong("IP"); });
                    header.col(|ui| { ui.strong("Port"); });
                    header.col(|ui| { ui.strong("Length"); });
                    header.col(|ui| { ui.strong("Info (Preview)"); });
                })
                .body(|body| {
                    body.rows(20.0, filtered_indices.len(), |mut row| {
                        let row_index = row.index();
                        let orig_idx = filtered_indices[row_index];
                        let entry = &self.logs[orig_idx];
                        let is_selected = Some(orig_idx) == self.selected_log_idx;

                        let (direction_text, color) = match entry.direction {
                            LogDirection::Sent => ("SENT", egui::Color32::from_rgb(100, 220, 100)),
                            LogDirection::Received => ("RECV", egui::Color32::from_rgb(100, 180, 255)),
                            LogDirection::SystemInfo => ("INFO", egui::Color32::from_rgb(200, 200, 200)),
                            LogDirection::SystemError => ("ERROR", egui::Color32::from_rgb(255, 90, 90)),
                        };

                        let time_str = entry.timestamp.format("%H:%M:%S.%3f").to_string();
                        let preview_truncated = &entry.preview_str;

                        let ip_str = if entry.direction == LogDirection::SystemInfo || entry.direction == LogDirection::SystemError {
                            "-".to_string()
                        } else {
                            entry.address.ip().to_string()
                        };

                        let port_str = if entry.direction == LogDirection::SystemInfo || entry.direction == LogDirection::SystemError {
                            "-".to_string()
                        } else {
                            entry.address.port().to_string()
                        };

                        row.set_selected(is_selected);
                        
                        let mut clicked = false;

                        // Use borderless selectable buttons to ensure clicks on the text labels are captured
                        row.col(|ui| {
                            let text = egui::RichText::new(format!("#{}", orig_idx + 1)).monospace();
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });
                        row.col(|ui| {
                            let text = egui::RichText::new(&time_str).monospace();
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });
                        row.col(|ui| {
                            let text = egui::RichText::new(direction_text).color(color);
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });
                        row.col(|ui| {
                            let text = egui::RichText::new(&ip_str).monospace();
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });
                        row.col(|ui| {
                            let text = egui::RichText::new(&port_str).monospace();
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });
                        row.col(|ui| {
                            let text = egui::RichText::new(format!("{} B", entry.data.len())).monospace();
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });
                        row.col(|ui| {
                            let text = egui::RichText::new(preview_truncated).monospace();
                            let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                            if res.clicked() {
                                clicked = true;
                            }
                        });

                        // Select row if the row itself or any cell inside it is clicked
                        if clicked || row.response().clicked() {
                            new_selection = Some(orig_idx);
                        }
                    });
                });
        });

        self.selected_log_idx = new_selection;
    }
}

// PCAP Helper: prepends raw ethernet, IPv4 and UDP headers to the UDP payloads
fn write_pcap_helper(path: &std::path::Path, logs: &[LogEntry], listener_addr_str: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    use std::net::SocketAddr;

    let mut file = File::create(path)?;

    // Global Header (24 bytes)
    file.write_all(&0xa1b2c3d4u32.to_ne_bytes())?; // magic number
    file.write_all(&2u16.to_ne_bytes())?;          // major version
    file.write_all(&4u16.to_ne_bytes())?;          // minor version
    file.write_all(&0i32.to_ne_bytes())?;          // gmt to local correction
    file.write_all(&0u32.to_ne_bytes())?;          // accuracy of timestamps
    file.write_all(&65535u32.to_ne_bytes())?;      // max length of captured packets
    file.write_all(&1u32.to_ne_bytes())?;          // data link type (1 = Ethernet)

    // Parse local bind address to use for dummy IP headers
    let local_ip = listener_addr_str.split(':').next().unwrap_or("127.0.0.1");
    let local_ip_parsed = local_ip.parse::<std::net::IpAddr>().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let local_port = listener_addr_str.split(':').nth(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(9000);

    for entry in logs {
        if entry.direction == LogDirection::SystemInfo || entry.direction == LogDirection::SystemError {
            continue;
        }

        let src_addr = match entry.direction {
            LogDirection::Received => entry.address,
            LogDirection::Sent => SocketAddr::new(local_ip_parsed, local_port),
            _ => continue,
        };
        let dest_addr = match entry.direction {
            LogDirection::Received => SocketAddr::new(local_ip_parsed, local_port),
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

        // 1. Ethernet Header (14 bytes)
        packet_data.extend_from_slice(&[0u8; 6]); // Dest MAC
        packet_data.extend_from_slice(&[0u8; 6]); // Src MAC
        packet_data.extend_from_slice(&0x0800u16.to_be_bytes()); // Type: IPv4

        // 2. IPv4 Header (20 bytes)
        packet_data.push(0x45);
        packet_data.push(0x00);
        let ip_total_len = (20 + 8 + payload_len) as u16;
        packet_data.extend_from_slice(&ip_total_len.to_be_bytes());
        packet_data.extend_from_slice(&0x0000u16.to_be_bytes());
        packet_data.extend_from_slice(&0x4000u16.to_be_bytes());
        packet_data.push(64);
        packet_data.push(17); // UDP
        
        let checksum_offset = packet_data.len();
        packet_data.extend_from_slice(&[0u8; 2]);

        packet_data.extend_from_slice(&src_ip.octets());
        packet_data.extend_from_slice(&dest_ip.octets());

        // Checksum
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

        // 3. UDP Header (8 bytes)
        packet_data.extend_from_slice(&src_addr.port().to_be_bytes());
        packet_data.extend_from_slice(&dest_addr.port().to_be_bytes());
        let udp_len = (8 + payload_len) as u16;
        packet_data.extend_from_slice(&udp_len.to_be_bytes());
        packet_data.extend_from_slice(&0x0000u16.to_be_bytes());

        // 4. Payload
        packet_data.extend_from_slice(payload);

        // PCAP Packet Record Header (16 bytes)
        let ts_sec = entry.timestamp.timestamp() as u32;
        let ts_usec = entry.timestamp.timestamp_subsec_micros() as u32;
        let cap_len = packet_data.len() as u32;
        let orig_len = packet_data.len() as u32;

        file.write_all(&ts_sec.to_ne_bytes())?;
        file.write_all(&ts_usec.to_ne_bytes())?;
        file.write_all(&cap_len.to_ne_bytes())?;
        file.write_all(&orig_len.to_ne_bytes())?;
        file.write_all(&packet_data)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LogDirection;
    use chrono::TimeZone;

    #[test]
    fn test_csv_formatting() {
        let timestamp = chrono::Local.with_ymd_and_hms(2026, 6, 13, 12, 0, 0).unwrap();
        let logs = vec![
            LogEntry::new(
                timestamp,
                LogDirection::Sent,
                "127.0.0.1:9000".parse().unwrap(),
                b"Hello".to_vec(),
            ),
            LogEntry::new(
                timestamp,
                LogDirection::Received,
                "192.168.1.50:5000".parse().unwrap(),
                vec![0x10, 0x81, 0x00, 0x01],
            ),
        ];

        let mut csv_content = String::new();
        csv_content.push_str("No,Timestamp,Direction,IP,Port,Length,DataHex,DataText\n");
        for (idx, entry) in logs.iter().enumerate() {
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
            csv_content.push_str(&format!("{},\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"\n", 
                idx + 1, time_str, dir_str, entry.ip, entry.port, len_str, hex_str, plain_str));
        }

        assert!(csv_content.contains("1,\"2026-06-13 12:00:00.000\",\"SENT\",\"127.0.0.1\",\"9000\",5,\"48 65 6C 6C 6F\",\"Hello\""));
        assert!(csv_content.contains("2,\"2026-06-13 12:00:00.000\",\"RECV\",\"192.168.1.50\",\"5000\",4,\"10 81 00 01\""));
    }

    #[test]
    fn test_write_pcap_helper() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_output.pcap");

        let timestamp = chrono::Local::now();
        let logs = vec![
            LogEntry::new(
                timestamp,
                LogDirection::Sent,
                "127.0.0.1:9000".parse().unwrap(),
                b"Hello".to_vec(),
            ),
            LogEntry::new(
                timestamp,
                LogDirection::Received,
                "192.168.1.50:5000".parse().unwrap(),
                vec![0x10, 0x81, 0x00, 0x01],
            ),
            LogEntry::new(
                timestamp,
                LogDirection::SystemInfo,
                "0.0.0.0:0".parse().unwrap(),
                b"System started".to_vec(),
            ),
        ];

        let result = write_pcap_helper(&path, &logs, "127.0.0.1:9000");
        assert!(result.is_ok());

        // Verify file size and header presence
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 24);
        
        // Check magic number
        let magic = &bytes[0..4];
        assert!(magic == &[0xa1, 0xb2, 0xc3, 0xd4] || magic == &[0xd4, 0xc3, 0xb2, 0xa1]);

        // Clean up
        let _ = std::fs::remove_file(path);
    }
}
