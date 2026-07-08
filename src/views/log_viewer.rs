use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::UdpStudioState;
use crate::types::{LogDirection, LogEntry, LogExportFormat};
use std::sync::atomic::{AtomicUsize, Ordering};
pub static LOG_ROW_RENDER_COUNT: AtomicUsize = AtomicUsize::new(0);

impl UdpStudioState {
    pub fn show_log_viewer(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().visuals.error_fg_color = egui::Color32::TRANSPARENT;
        ui.style_mut().visuals.warn_fg_color = egui::Color32::TRANSPARENT;
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };
        let tr_args = |key: &str, args: &std::collections::HashMap<std::borrow::Cow<'static, str>, egui_i18n::fluent_bundle::FluentValue<'_>>| {
            egui_i18n::set_language(&lang_id);
            let mut fluent_args = egui_i18n::fluent::FluentArgs::new();
            for (k, v) in args {
                fluent_args.set(k.as_ref(), v.clone());
            }
            egui_i18n::translate_fluent(key, &fluent_args)
        };

        let old_selection = self.selected_log_idx;
        let mut scroll_to_row_idx = None;

        let filtered_indices = &self.filtered_indices;

        // Copy shortcut (Ctrl+C / Cmd+C)
        if !ui.ctx().egui_wants_keyboard_input() {
            let copy_shortcut = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C));
            if copy_shortcut {
                self.copy_selected_logs_to_clipboard(ui, LogExportFormat::Csv);
            }
        }

        // Handle keyboard navigation (ArrowUp / ArrowDown)
        if !filtered_indices.is_empty() && !ui.ctx().egui_wants_keyboard_input() {
            let mut key_up = false;
            let mut key_down = false;
            let mut shift = false;
            ui.input(|i| {
                if i.key_pressed(egui::Key::ArrowUp) {
                    key_up = true;
                    shift = i.modifiers.shift;
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    key_down = true;
                    shift = i.modifiers.shift;
                }
            });

            if key_up || key_down {
                let current_filtered_pos = self.last_clicked_log_idx.and_then(|idx| {
                    filtered_indices.iter().position(|&x| x == idx)
                });

                let next_filtered_pos = match current_filtered_pos {
                    Some(pos) => {
                        if key_up {
                            if pos > 0 {
                                Some(pos - 1)
                            } else {
                                Some(0)
                            }
                        } else {
                            if pos + 1 < filtered_indices.len() {
                                Some(pos + 1)
                            } else {
                                Some(filtered_indices.len() - 1)
                            }
                        }
                    }
                    None => {
                        if key_up {
                            Some(filtered_indices.len() - 1)
                        } else {
                            Some(0)
                        }
                    }
                };

                if let Some(pos) = next_filtered_pos {
                    let next_idx = filtered_indices[pos];
                    if shift {
                        if let Some(last_clicked) = self.last_clicked_log_idx {
                            let last_clicked_row = filtered_indices.iter().position(|&x| x == last_clicked);
                            if let Some(start_row) = last_clicked_row {
                                let end_row = pos;
                                let r_start = start_row.min(end_row);
                                let r_end = start_row.max(end_row);
                                self.selected_log_indices.clear();
                                for r in r_start..=r_end {
                                    if r < filtered_indices.len() {
                                        self.selected_log_indices.insert(filtered_indices[r]);
                                    }
                                }
                            }
                        } else {
                            self.selected_log_indices.insert(next_idx);
                            self.last_clicked_log_idx = Some(next_idx);
                        }
                    } else {
                        self.selected_log_indices.clear();
                        self.selected_log_indices.insert(next_idx);
                        self.last_clicked_log_idx = Some(next_idx);
                    }
                    self.sync_selected_log_idx();
                    scroll_to_row_idx = Some(pos);
                }
            }
        }
        ui.horizontal(|ui| {
                        if ui.button(tr("log-btn-clear")).clicked() {
                            self.logs.clear();
                            self.filtered_indices.clear();
                            self.selected_log_indices.clear();
                            self.last_clicked_log_idx = None;
                            self.selected_log_idx = None;
                        }

                        ui.add_space(8.0);
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
                        if ui.button(tr("log-btn-save")).on_hover_text(tr("log-btn-save-tip")).clicked() {
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
                                        let listener_addr = self.get_selected_socket()
                                            .map(|s| format!("{}:{}", s.ip, s.port))
                                            .unwrap_or_else(|| "0.0.0.0:9000".to_string());
                                        write_pcap_helper(&path, &self.logs, &listener_addr)
                                    }
                                    LogExportFormat::Csv => {
                                        // Default to CSV
                                        let mut csv_content = String::new();
                                        csv_content.push_str("No,Timestamp,Direction,Src IP,Src Port,Dest IP,Dest Port,Length,DataHex,DataText\n");
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
                                            csv_content.push_str(&format!("{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"\n", 
                                                idx + 1, time_str, dir_str, entry.src_ip, entry.src_port, entry.dest_ip, entry.dest_port, len_str, hex_str, plain_str));
                                        }
                                        std::fs::write(&path, csv_content)
                                    }
                                };

                                match result {
                                    Ok(_) => {
                                        let mut args = std::collections::HashMap::new();
                                        args.insert(std::borrow::Cow::Borrowed("path"), path.display().to_string().into());
                                        self.add_system_info(tr_args("log-save-success", &args));
                                    }
                                    Err(e) => {
                                        let mut args = std::collections::HashMap::new();
                                        args.insert(std::borrow::Cow::Borrowed("msg"), e.to_string().into());
                                        self.add_system_error(tr_args("log-save-fail", &args));
                                    }
                                }
                            }
                        }
                        
                        ui.checkbox(&mut self.auto_scroll, tr("log-checkbox-autoscroll"));
                    });

                    ui.horizontal(|ui| {
                            ui.label(tr("log-label-ip-filter"));
                            
                            let help_response = {
                                let size = egui::vec2(16.0, 16.0);
                                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
                                if ui.is_rect_visible(rect) {
                                    let painter = ui.painter();
                                    let bg_color = if response.hovered() {
                                        egui::Color32::from_rgb(0, 120, 215)
                                    } else {
                                        egui::Color32::from_rgb(180, 180, 180)
                                    };
                                    painter.circle_filled(rect.center(), 8.0, bg_color);
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "?",
                                        egui::FontId::new(11.0, egui::FontFamily::Proportional),
                                        egui::Color32::WHITE,
                                    );
                                }
                                response
                            };
                            help_response.on_hover_text(tr("log-filter-tooltip"));

                            let is_valid = if self.filter_input.trim().is_empty() {
                                None
                            } else {
                                Some(crate::filter::parse_filter(&self.filter_input).is_ok())
                            };

                            let original_extreme_bg = ui.visuals().extreme_bg_color;
                            let original_text_color = ui.style().visuals.override_text_color;

                            if let Some(valid) = is_valid {
                                if valid {
                                    ui.style_mut().visuals.extreme_bg_color = egui::Color32::from_rgb(0, 80, 0);
                                    ui.style_mut().visuals.override_text_color = Some(egui::Color32::WHITE);
                                } else {
                                    ui.style_mut().visuals.extreme_bg_color = egui::Color32::from_rgb(120, 0, 0);
                                    ui.style_mut().visuals.override_text_color = Some(egui::Color32::WHITE);
                                }
                            }

                            let apply_btn_width = 60.0;
                            let history_btn_width = if !self.filter_history.is_empty() { 30.0 } else { 0.0 };
                            let input_width = (ui.available_width() - apply_btn_width - history_btn_width - 30.0).max(100.0);

                            let text_edit_response = ui.add(
                                egui::TextEdit::singleline(&mut self.filter_input)
                                    .desired_width(input_width)
                            );

                            ui.style_mut().visuals.extreme_bg_color = original_extreme_bg;
                            ui.style_mut().visuals.override_text_color = original_text_color;

                            let is_syntax_valid = is_valid.unwrap_or(true);
                            let enter_pressed = is_syntax_valid
                                && text_edit_response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let apply_clicked = ui.add_enabled(is_syntax_valid, egui::Button::new(tr("log-filter-apply-btn"))).clicked();

                            if enter_pressed || apply_clicked {
                                self.apply_filter();
                            }

                            if !self.filter_history.is_empty() {
                                ui.menu_button("▼", |ui| {
                                    let mut selected_history = None;
                                    for hist in &self.filter_history {
                                        if ui.selectable_label(false, hist).clicked() {
                                            selected_history = Some(hist.clone());
                                            ui.close();
                                        }
                                    }
                                    if let Some(hist) = selected_history {
                                        self.filter_input = hist;
                                        self.apply_filter();
                                    }
                                });
                            }
                        }
                    );

        ui.separator();

        let filtered_indices = self.filtered_indices.clone();
        let is_shift = ui.input(|i| i.modifiers.shift);
        let is_cmd_ctrl = ui.input(|i| i.modifiers.command);


        // 横スクロールエリア
        let (clicked_row, right_clicked_row) = egui::ScrollArea::horizontal()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let mut clicked_row = None;
                        let mut right_clicked_row = None;
                        let is_dark = self.is_dark_theme(ui.ctx());
                        let mut table = TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .vscroll(true)
                            .sense(egui::Sense::click()) // Add click sense to enable selection!
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::initial(45.0))  // No.
                            .column(Column::initial(100.0)) // Time
                            .column(Column::initial(80.0))  // Type
                            .column(Column::initial(110.0)) // Source IP
                            .column(Column::initial(70.0))  // Send Port
                            .column(Column::initial(110.0)) // Dest IP
                            .column(Column::initial(70.0))  // Recv Port
                            .column(Column::initial(60.0))  // Length
                            .column(Column::remainder());  // Info/Payload

                        if let Some(row_pos) = scroll_to_row_idx {
                            table = table.scroll_to_row(row_pos, None);
                        }

                        table = table.stick_to_bottom(self.auto_scroll);

                        table
                            .header(28.0, |mut header| {
                                header.col(|ui| { ui.strong(tr("log-hdr-no")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-time")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-type")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-source-ip")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-send-port")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-dest-ip")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-recv-port")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-length")); });
                                header.col(|ui| { ui.strong(tr("log-hdr-info")); });
                            })
                            .body(|body| {
                                body.rows(32.0, filtered_indices.len(), |mut row| {
                                    LOG_ROW_RENDER_COUNT.fetch_add(1, Ordering::SeqCst);
                                    let row_index = row.index();
                                    let orig_idx = filtered_indices[row_index];
                                    let entry = &self.logs[orig_idx];
                                    let is_selected = self.selected_log_indices.contains(&orig_idx);

                                    let show_context_menu = |ui: &mut egui::Ui| {
                                        if ui.button(tr("log-ctx-copy-csv")).clicked() {
                                            self.copy_selected_logs_to_clipboard(ui, LogExportFormat::Csv);
                                            ui.close();
                                        }
                                        if ui.button(tr("log-ctx-copy-json")).clicked() {
                                            self.copy_selected_logs_to_clipboard(ui, LogExportFormat::Json);
                                            ui.close();
                                        }
                                    };

                                    let (direction_text, mut color) = match entry.direction {
                                        LogDirection::Sent => {
                                            let c = if is_dark {
                                                egui::Color32::from_rgb(100, 220, 100)
                                            } else {
                                                egui::Color32::from_rgb(46, 125, 50)
                                            };
                                            ("SENT", c)
                                        }
                                        LogDirection::Received => {
                                            let c = if is_dark {
                                                egui::Color32::from_rgb(100, 180, 255)
                                            } else {
                                                egui::Color32::from_rgb(25, 118, 210)
                                            };
                                            ("RECV", c)
                                        }
                                        LogDirection::SystemInfo => {
                                            let c = if is_dark {
                                                egui::Color32::from_rgb(200, 200, 200)
                                            } else {
                                                egui::Color32::from_rgb(117, 117, 117)
                                            };
                                            ("INFO", c)
                                        }
                                        LogDirection::SystemError => {
                                            let c = if is_dark {
                                                egui::Color32::from_rgb(255, 90, 90)
                                            } else {
                                                egui::Color32::from_rgb(211, 47, 47)
                                            };
                                            ("ERROR", c)
                                        }
                                    };

                                    if is_selected && !is_dark {
                                        color = match entry.direction {
                                            LogDirection::Sent => egui::Color32::from_rgb(180, 255, 180),
                                            LogDirection::Received => egui::Color32::from_rgb(200, 230, 255),
                                            LogDirection::SystemInfo => egui::Color32::from_rgb(240, 240, 240),
                                            LogDirection::SystemError => egui::Color32::from_rgb(255, 200, 200),
                                        };
                                    }

                                    let time_str = entry.timestamp.format("%H:%M:%S.%3f").to_string();
                                    let preview_truncated = entry.get_preview(self.max_display_data_bytes);

                                    row.set_selected(is_selected);
                                    
                                    let mut clicked = false;

                                    // Use borderless selectable buttons to ensure clicks on the text labels are captured
                                    row.col(|ui| {
                                        let text = egui::RichText::new(format!("#{}", orig_idx + 1)).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(&time_str).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(direction_text).color(color);
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(&entry.src_ip).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(&entry.src_port).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(&entry.dest_ip).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(&entry.dest_port).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(format!("{} B", entry.data.len())).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });
                                    row.col(|ui| {
                                        let text = egui::RichText::new(&preview_truncated).monospace();
                                        let res = ui.add(egui::Button::selectable(is_selected, text).frame(false));
                                        res.context_menu(|ui| show_context_menu(ui));
                                        if res.clicked() {
                                            clicked = true;
                                        }
                                    });

                                    // Handle click events (left and right click)
                                    let row_response = row.response();
                                    
                                    // Right-click logic
                                    if row_response.secondary_clicked() {
                                        right_clicked_row = Some(orig_idx);
                                    }

                                    // Left-click logic
                                    if clicked || (row_response.clicked() && !row_response.secondary_clicked()) {
                                        clicked_row = Some((row_index, orig_idx));
                                    }

                                    row_response.context_menu(|ui| show_context_menu(ui));
                                });
                            });
                        (clicked_row, right_clicked_row)
                    }
                ).inner
            }).inner;

        // Apply selection logic after table ends to satisfy borrow checker (Deferred Mutation Pattern)
        if let Some(orig_idx) = right_clicked_row {
            let is_selected = self.selected_log_indices.contains(&orig_idx);
            if !is_selected {
                self.selected_log_indices.clear();
                self.selected_log_indices.insert(orig_idx);
                self.last_clicked_log_idx = Some(orig_idx);
                self.sync_selected_log_idx();
            }
        } else if let Some((row_index, orig_idx)) = clicked_row {
            if is_shift {
                if let Some(last_clicked) = self.last_clicked_log_idx {
                    let last_clicked_row = filtered_indices.iter().position(|&x| x == last_clicked);
                    if let Some(start_row) = last_clicked_row {
                        let end_row = row_index;
                        let r_start = start_row.min(end_row);
                        let r_end = start_row.max(end_row);
                        if !is_cmd_ctrl {
                            self.selected_log_indices.clear();
                        }
                        for r in r_start..=r_end {
                            if r < filtered_indices.len() {
                                self.selected_log_indices.insert(filtered_indices[r]);
                            }
                        }
                    } else {
                        if !is_cmd_ctrl {
                            self.selected_log_indices.clear();
                        }
                        self.selected_log_indices.insert(orig_idx);
                    }
                } else {
                    if !is_cmd_ctrl {
                        self.selected_log_indices.clear();
                    }
                    self.selected_log_indices.insert(orig_idx);
                }
                self.last_clicked_log_idx = Some(orig_idx);
            } else if is_cmd_ctrl {
                if self.selected_log_indices.contains(&orig_idx) {
                    self.selected_log_indices.remove(&orig_idx);
                } else {
                    self.selected_log_indices.insert(orig_idx);
                }
                self.last_clicked_log_idx = Some(orig_idx);
            } else {
                self.selected_log_indices.clear();
                self.selected_log_indices.insert(orig_idx);
                self.last_clicked_log_idx = Some(orig_idx);
            }
            self.sync_selected_log_idx();
        }

        if old_selection != self.selected_log_idx {
            if let Some(idx) = self.selected_log_idx {
                if idx < self.logs.len() {
                    let entry = &self.logs[idx];
                    let src_port = entry.src_port.as_str();
                    let dest_port = entry.dest_port.as_str();
                    
                    let el_ports: Vec<&str> = self.protocol_config.echonet_lite_port.split(',').map(|s| s.trim()).collect();
                    let syslog_ports: Vec<&str> = self.protocol_config.syslog_port.split(',').map(|s| s.trim()).collect();
                    let snmp_agent_ports: Vec<&str> = self.protocol_config.snmp_agent_port.split(',').map(|s| s.trim()).collect();
                    let snmp_trap_ports: Vec<&str> = self.protocol_config.snmp_trap_port.split(',').map(|s| s.trim()).collect();
                    let dns_ports: Vec<&str> = self.protocol_config.dns_port.split(',').map(|s| s.trim()).collect();
                    let coap_ports: Vec<&str> = self.protocol_config.coap_port.split(',').map(|s| s.trim()).collect();

                    if el_ports.contains(&src_port) || el_ports.contains(&dest_port) {
                        self.inspector_protocol = crate::types::InspectorProtocol::EchonetLite;
                        self.record_inspector_protocol_usage(crate::types::InspectorProtocol::EchonetLite);
                    } else if syslog_ports.contains(&src_port) || syslog_ports.contains(&dest_port) {
                        self.inspector_protocol = crate::types::InspectorProtocol::Syslog;
                        self.record_inspector_protocol_usage(crate::types::InspectorProtocol::Syslog);
                    } else if snmp_agent_ports.contains(&src_port) || snmp_agent_ports.contains(&dest_port)
                        || snmp_trap_ports.contains(&src_port) || snmp_trap_ports.contains(&dest_port)
                    {
                        self.inspector_protocol = crate::types::InspectorProtocol::Snmp;
                        self.record_inspector_protocol_usage(crate::types::InspectorProtocol::Snmp);
                    } else if dns_ports.contains(&src_port) || dns_ports.contains(&dest_port) {
                        self.inspector_protocol = crate::types::InspectorProtocol::Dns;
                        self.record_inspector_protocol_usage(crate::types::InspectorProtocol::Dns);
                    } else if coap_ports.contains(&src_port) || coap_ports.contains(&dest_port) {
                        self.inspector_protocol = crate::types::InspectorProtocol::Coap;
                        self.record_inspector_protocol_usage(crate::types::InspectorProtocol::Coap);
                    } else {
                        self.inspector_protocol = crate::types::InspectorProtocol::Raw;
                    }
                }
            }
        }
    }

    fn copy_selected_logs_to_clipboard(&self, ui: &mut egui::Ui, format: LogExportFormat) {
        if self.selected_log_indices.is_empty() {
            return;
        }

        let content = match format {
            LogExportFormat::Csv => {
                let mut csv_content = String::new();
                csv_content.push_str("No,Timestamp,Direction,Src IP,Src Port,Dest IP,Dest Port,Length,DataHex,DataText\n");
                for &orig_idx in &self.selected_log_indices {
                    if let Some(entry) = self.logs.get(orig_idx) {
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
                        csv_content.push_str(&format!("{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"\n", 
                            orig_idx + 1, time_str, dir_str, entry.src_ip, entry.src_port, entry.dest_ip, entry.dest_port, len_str, hex_str, plain_str));
                    }
                }
                csv_content
            }
            LogExportFormat::Json => {
                let selected_entries: Vec<&LogEntry> = self.selected_log_indices.iter()
                    .filter_map(|&idx| self.logs.get(idx))
                    .collect();
                match serde_json::to_string_pretty(&selected_entries) {
                    Ok(json_str) => json_str,
                    Err(e) => format!("JSON Serialization Error: {}", e),
                }
            }
            _ => String::new(),
        };

        if !content.is_empty() {
            ui.ctx().copy_text(content);
        }
    }
}

// PCAP Helper: prepends raw ethernet, IPv4 and UDP headers to the UDP payloads
pub fn write_pcap_helper(path: &std::path::Path, logs: &[LogEntry], listener_addr_str: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;

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

        packet_data.extend_from_slice(&src_ip_v4.octets());
        packet_data.extend_from_slice(&dest_ip_v4.octets());

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
        packet_data.extend_from_slice(&src_port.to_be_bytes());
        packet_data.extend_from_slice(&dest_port.to_be_bytes());
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
