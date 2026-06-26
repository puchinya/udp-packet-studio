use eframe::egui;
use crate::UdpStudioState;
use crate::types::{PacketDefinition, PayloadType, generate_id};

impl UdpStudioState {
    pub fn generate_echonet_lite_hex(&self) -> Result<String, String> {
        let ehd = "1081";
        
        let tid_clean: String = self.el_tid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if tid_clean.len() != 4 {
            return Err("Transaction ID (TID) must be exactly 2 bytes (4 hex characters)".to_string());
        }
        
        let seoj_clean: String = self.el_seoj.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if seoj_clean.len() != 6 {
            return Err("Source Object (SEOJ) must be exactly 3 bytes (6 hex characters)".to_string());
        }
        
        let deoj_raw = match self.el_deoj_preset {
            0 => "013001", // Home Air Conditioner
            1 => "028801", // Low-voltage Smart Electric Meter
            2 => "0EF001", // Node Profile
            _ => &self.el_deoj_custom,
        };
        let deoj_clean: String = deoj_raw.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if deoj_clean.len() != 6 {
            return Err("Destination Object (DEOJ) must be exactly 3 bytes (6 hex characters)".to_string());
        }
        
        let esv = match self.el_esv_preset {
            0 => "62", // Get (Property Read Request)
            1 => "61", // SetC (Property Write Request, Response Required)
            2 => "60", // SetI (Property Write Request, No Response Required)
            3 => "73", // INF (Property Notification)
            _ => "62",
        };
        
        let opc = "01"; // Default to 1 property per frame for simplicity
        
        let epc_raw = match self.el_epc_preset {
            0 => "80", // Operation Status (ON/OFF)
            1 => "B0", // Operation Mode (Auto/Cool/Heat/etc.)
            2 => "E0", // Measured Instantaneous Power
            _ => &self.el_epc_custom,
        };
        let epc_clean: String = epc_raw.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if epc_clean.len() != 2 {
            return Err("Property Code (EPC) must be exactly 1 byte (2 hex characters)".to_string());
        }
        
        let (pdc, edt_clean) = if esv == "62" {
            ("00".to_string(), "".to_string())
        } else {
            let edt_c: String = self.el_edt.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if edt_c.is_empty() {
                return Err("Property Data (EDT) cannot be empty for Set/Write requests".to_string());
            }
            if edt_c.len() % 2 != 0 {
                return Err("Property Data (EDT) must have an even number of hex characters".to_string());
            }
            let pdc_val = edt_c.len() / 2;
            (format!("{:02x}", pdc_val), edt_c)
        };
        
        let raw_hex = format!("{}{}{}{}{}{}{}{}{}", ehd, tid_clean, seoj_clean, deoj_clean, esv, opc, epc_clean, pdc, edt_clean);
        
        let mut formatted = String::new();
        for (i, c) in raw_hex.chars().enumerate() {
            formatted.push(c);
            if i % 2 == 1 && i + 1 < raw_hex.len() {
                formatted.push(' ');
            }
        }
        
        Ok(formatted)
    }

    pub fn show_echonet_lite_helper(&mut self, ui: &mut egui::Ui, current_target: &str) -> Option<(String, PayloadType, String)> {
        ui.checkbox(&mut self.el_show_helper, "💡 ECHONET Lite Packet Helper");
        
        let mut result = None;
        if self.el_show_helper {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("💡 ECHONET Lite Frame Builder");
                ui.add_space(8.0);
                
                let mut generate_clicked = false;
                egui::Grid::new("el_grid_shared")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Transaction ID (TID):");
                        ui.text_edit_singleline(&mut self.el_tid);
                        ui.end_row();
                        
                        ui.label("Source Object (SEOJ):");
                        ui.text_edit_singleline(&mut self.el_seoj);
                        ui.end_row();
                        
                        ui.label("Destination Object (DEOJ):");
                        ui.horizontal(|ui| {
                            let deoj_label = match self.el_deoj_preset {
                                0 => "Home Air Conditioner (013001)",
                                1 => "Smart Electric Meter (028801)",
                                2 => "Node Profile Object (0EF001)",
                                _ => "Custom Object...",
                            };
                            egui::ComboBox::from_id_salt("deoj_combo_shared")
                                .selected_text(deoj_label)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.el_deoj_preset, 0, "Home Air Conditioner (013001)");
                                    ui.selectable_value(&mut self.el_deoj_preset, 1, "Smart Electric Meter (028801)");
                                    ui.selectable_value(&mut self.el_deoj_preset, 2, "Node Profile Object (0EF001)");
                                    ui.selectable_value(&mut self.el_deoj_preset, 3, "Custom Object...");
                                });
                            if self.el_deoj_preset == 3 {
                                ui.text_edit_singleline(&mut self.el_deoj_custom);
                            }
                        });
                        ui.end_row();
                        
                        ui.label("Service Code (ESV):");
                        let esv_label = match self.el_esv_preset {
                            0 => "Get (0x62 - Property Read Request)",
                            1 => "SetC (0x61 - Property Write, Response Req)",
                            2 => "SetI (0x60 - Property Write, No Response)",
                            3 => "INF (0x73 - Property Notification)",
                            _ => "Custom...",
                        };
                        egui::ComboBox::from_id_salt("esv_combo_shared")
                            .selected_text(esv_label)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.el_esv_preset, 0, "Get (0x62 - Property Read Request)");
                                ui.selectable_value(&mut self.el_esv_preset, 1, "SetC (0x61 - Property Write, Response Req)");
                                ui.selectable_value(&mut self.el_esv_preset, 2, "SetI (0x60 - Property Write, No Response)");
                                ui.selectable_value(&mut self.el_esv_preset, 3, "INF (0x73 - Property Notification)");
                            });
                        ui.end_row();
                        
                        ui.label("Property Code (EPC):");
                        ui.horizontal(|ui| {
                            let epc_label = match self.el_epc_preset {
                                0 => "Operation Status (0x80)",
                                1 => "Operation Mode (0xB0)",
                                2 => "Instantaneous Power (0xE0)",
                                _ => "Custom Property...",
                            };
                            egui::ComboBox::from_id_salt("epc_combo_shared")
                                .selected_text(epc_label)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.el_epc_preset, 0, "Operation Status (0x80)");
                                    ui.selectable_value(&mut self.el_epc_preset, 1, "Operation Mode (0xB0)");
                                    ui.selectable_value(&mut self.el_epc_preset, 2, "Instantaneous Power (0xE0)");
                                    ui.selectable_value(&mut self.el_epc_preset, 3, "Custom Property...");
                                });
                            if self.el_epc_preset == 3 {
                                ui.text_edit_singleline(&mut self.el_epc_custom);
                            }
                        });
                        ui.end_row();
                        
                        if self.el_esv_preset != 0 {
                            ui.label("Property Data (EDT, hex):");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.el_edt);
                                if self.el_epc_preset == 0 {
                                    if ui.small_button("ON (30)").clicked() {
                                        self.el_edt = "30".to_string();
                                    }
                                    if ui.small_button("OFF (31)").clicked() {
                                        self.el_edt = "31".to_string();
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    });
                
                ui.add_space(8.0);
                if ui.button("⚙️ Generate and Insert ECHONET Lite Hex").clicked() {
                    generate_clicked = true;
                }

                if generate_clicked {
                    match self.generate_echonet_lite_hex() {
                        Ok(hex_str) => {
                            let mut target = current_target.trim().to_string();
                            if target.is_empty() || target == "127.0.0.1:9000" {
                                target = "127.0.0.1:3610".to_string();
                            } else if !target.contains(':') {
                                target = format!("{}:3610", target);
                            }
                            result = Some((hex_str, PayloadType::Hex, target));
                        }
                        Err(e) => {
                            self.add_system_error(format!("ECHONET Lite builder error: {}", e));
                        }
                    }
                }
            });
        }
        result
    }

    pub fn show_sender(&mut self, ui: &mut egui::Ui) {
        let mut send_trigger = false;
        let mut save_trigger = false;

        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt("composer_scroll").show(ui, |ui| {
                egui::Grid::new("composer_grid")
                    .num_columns(2)
                    .spacing([12.0, 12.0])
                    .show(ui, |ui| {
                        ui.label("Destination Address:");
                        if ui.text_edit_singleline(&mut self.composer_target).changed() {
                            self.save_config();
                        }
                        ui.end_row();
                        
                        ui.label("Payload Format:");
                        ui.horizontal(|ui| {
                            let r1 = ui.radio_value(&mut self.composer_payload_type, PayloadType::Text, "Text (UTF-8)");
                            let r2 = ui.radio_value(&mut self.composer_payload_type, PayloadType::Hex, "Hex (Spaces optional)");
                            if r1.changed() || r2.changed() {
                                self.save_config();
                            }
                        });
                        ui.end_row();
                    });
                
                ui.add_space(8.0);
                
                let current_target = self.composer_target.clone();
                if let Some((payload, format, target)) = self.show_echonet_lite_helper(ui, &current_target) {
                    self.composer_payload = payload;
                    self.composer_payload_type = format;
                    self.composer_target = target;
                    self.save_config();
                }
                
                ui.add_space(10.0);
                ui.label("Payload Content:");
                
                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.composer_payload)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_rows(8)
                        .desired_width(ui.available_width())
                );
                if response.changed() {
                    self.save_config();
                }
                
                ui.add_space(15.0);
                
                ui.horizontal(|ui| {
                    let is_bound = self.is_listening;
                    let send_btn = ui.add_enabled(
                        is_bound, 
                        egui::Button::new("🚀 Send").min_size(egui::vec2(120.0, 32.0))
                    );
                    
                    if send_btn.clicked() {
                        send_trigger = true;
                    }
                    
                    if !is_bound {
                        ui.colored_label(egui::Color32::from_rgb(255, 120, 120), "⚠️ Start listener socket first.");
                    }
                });
                
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.heading("💾 Save Request to Collection");
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.composer_name);
                });
                
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label("Collection:");
                    
                    if self.collections.is_empty() {
                        ui.label("No collections. Click 'Save' to create one.");
                    } else {
                        if self.composer_selected_collection_idx >= self.collections.len() {
                            self.composer_selected_collection_idx = 0;
                        }
                        let current_name = self.collections[self.composer_selected_collection_idx].name.clone();
                        egui::ComboBox::from_id_salt("save_collection_combo")
                            .selected_text(current_name)
                            .show_ui(ui, |ui| {
                                for (idx, collection) in self.collections.iter().enumerate() {
                                    ui.selectable_value(&mut self.composer_selected_collection_idx, idx, &collection.name);
                                }
                            });
                    }
                    
                    if ui.button("💾 Save").clicked() {
                        save_trigger = true;
                    }
                });
            });
        });

        // Apply deferred actions outside borrowing scopes
        if send_trigger {
            let target = self.composer_target.clone();
            let payload_type = self.composer_payload_type;
            let payload = self.composer_payload.clone();
            self.send_packet(&target, payload_type, &payload);
        }
        if save_trigger {
            let name = if self.composer_name.trim().is_empty() {
                let total_reqs: usize = self.collections.iter().map(|c| c.requests.len()).sum();
                format!("Request {}", total_reqs + 1)
            } else {
                self.composer_name.clone()
            };
            
            let new_def = PacketDefinition {
                id: generate_id(),
                name,
                target: self.composer_target.clone(),
                payload_type: self.composer_payload_type,
                payload: self.composer_payload.clone(),
            };
            
            if self.collections.is_empty() {
                self.collections.push(crate::types::Collection {
                    id: generate_id(),
                    name: "My Requests".to_string(),
                    requests: vec![new_def.clone()],
                    is_expanded: true,
                });
                self.composer_selected_collection_idx = 0;
            } else {
                if self.composer_selected_collection_idx >= self.collections.len() {
                    self.composer_selected_collection_idx = 0;
                }
                self.collections[self.composer_selected_collection_idx].requests.push(new_def.clone());
            }
            
            self.selected_request_id = Some(new_def.id);
            self.composer_name.clear();
            self.save_config();
        }
    }
}

