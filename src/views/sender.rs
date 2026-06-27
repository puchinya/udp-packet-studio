use eframe::egui;
use crate::UdpStudioState;
use crate::types::{PacketDefinition, PayloadType, generate_id, validate_payload};

impl UdpStudioState {
    pub fn generate_echonet_lite_hex(&self) -> Result<String, String> {
        let ehd = "1081";
        
        let tid_clean: String = self.el_tid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if tid_clean.len() != 4 {
            return Err(self.tr("el-err-tid"));
        }
        
        let seoj_clean: String = self.el_seoj.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if seoj_clean.len() != 6 {
            return Err(self.tr("el-err-seoj"));
        }
        
        let deoj_raw = match self.el_deoj_preset {
            0 => "013001", // Home Air Conditioner
            1 => "028801", // Low-voltage Smart Electric Meter
            2 => "0EF001", // Node Profile
            _ => &self.el_deoj_custom,
        };
        let deoj_clean: String = deoj_raw.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if deoj_clean.len() != 6 {
            return Err(self.tr("el-err-deoj"));
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
            return Err(self.tr("el-err-epc"));
        }
        
        let (pdc, edt_clean) = if esv == "62" {
            ("00".to_string(), "".to_string())
        } else {
            let edt_c: String = self.el_edt.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if edt_c.is_empty() {
                return Err(self.tr("el-err-edt-empty"));
            }
            if edt_c.len() % 2 != 0 {
                return Err(self.tr("el-err-edt-even"));
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

        ui.checkbox(&mut self.el_show_helper, tr("el-helper-checkbox"));
        
        let mut result = None;
        if self.el_show_helper {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong(tr("el-builder-title"));
                ui.add_space(8.0);
                
                let mut generate_clicked = false;
                egui::Grid::new("el_grid_shared")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        ui.label(tr("el-label-tid"));
                        ui.text_edit_singleline(&mut self.el_tid);
                        ui.end_row();
                        
                        ui.label(tr("el-label-seoj"));
                        ui.text_edit_singleline(&mut self.el_seoj);
                        ui.end_row();
                        
                        ui.label(tr("el-label-deoj"));
                        ui.horizontal(|ui| {
                            let deoj_label = match self.el_deoj_preset {
                                0 => tr("el-deoj-preset-ac"),
                                1 => tr("el-deoj-preset-meter"),
                                2 => tr("el-deoj-preset-node"),
                                _ => tr("el-deoj-preset-custom").to_string(),
                            };
                            egui::ComboBox::from_id_salt("deoj_combo_shared")
                                .selected_text(deoj_label)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.el_deoj_preset, 0, tr("el-deoj-preset-ac"));
                                    ui.selectable_value(&mut self.el_deoj_preset, 1, tr("el-deoj-preset-meter"));
                                    ui.selectable_value(&mut self.el_deoj_preset, 2, tr("el-deoj-preset-node"));
                                    ui.selectable_value(&mut self.el_deoj_preset, 3, tr("el-deoj-preset-custom"));
                                });
                            if self.el_deoj_preset == 3 {
                                ui.text_edit_singleline(&mut self.el_deoj_custom);
                            }
                        });
                        ui.end_row();
                        
                        ui.label(tr("el-label-esv"));
                        let esv_label = match self.el_esv_preset {
                            0 => tr("el-esv-preset-get"),
                            1 => tr("el-esv-preset-setc"),
                            2 => tr("el-esv-preset-seti"),
                            3 => tr("el-esv-preset-inf"),
                            _ => tr("el-esv-preset-get").to_string(),
                        };
                        egui::ComboBox::from_id_salt("esv_combo_shared")
                            .selected_text(esv_label)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.el_esv_preset, 0, tr("el-esv-preset-get"));
                                ui.selectable_value(&mut self.el_esv_preset, 1, tr("el-esv-preset-setc"));
                                ui.selectable_value(&mut self.el_esv_preset, 2, tr("el-esv-preset-seti"));
                                ui.selectable_value(&mut self.el_esv_preset, 3, tr("el-esv-preset-inf"));
                            });
                        ui.end_row();
                        
                        ui.label(tr("el-label-epc"));
                        ui.horizontal(|ui| {
                            let epc_label = match self.el_epc_preset {
                                0 => tr("el-epc-preset-status"),
                                1 => tr("el-epc-preset-mode"),
                                2 => tr("el-epc-preset-power"),
                                _ => tr("el-epc-preset-custom").to_string(),
                            };
                            egui::ComboBox::from_id_salt("epc_combo_shared")
                                .selected_text(epc_label)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.el_epc_preset, 0, tr("el-epc-preset-status"));
                                    ui.selectable_value(&mut self.el_epc_preset, 1, tr("el-epc-preset-mode"));
                                    ui.selectable_value(&mut self.el_epc_preset, 2, tr("el-epc-preset-power"));
                                    ui.selectable_value(&mut self.el_epc_preset, 3, tr("el-epc-preset-custom"));
                                });
                            if self.el_epc_preset == 3 {
                                ui.text_edit_singleline(&mut self.el_epc_custom);
                            }
                        });
                        ui.end_row();
                        
                        if self.el_esv_preset != 0 {
                            ui.label(tr("el-label-edt"));
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut self.el_edt);
                                if self.el_epc_preset == 0 {
                                    if ui.small_button(tr("el-edt-on")).clicked() {
                                        self.el_edt = "30".to_string();
                                    }
                                    if ui.small_button(tr("el-edt-off")).clicked() {
                                        self.el_edt = "31".to_string();
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    });
                
                ui.add_space(8.0);
                if ui.button(tr("el-btn-generate")).clicked() {
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
                            let mut args = std::collections::HashMap::new();
                            args.insert(std::borrow::Cow::Borrowed("msg"), e.clone().into());
                            self.add_system_error(tr_args("el-err-prefix", &args));
                        }
                    }
                }
            });
        }
        result
    }

    pub fn show_sender(&mut self, ui: &mut egui::Ui) {
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

        let mut send_trigger = false;
        let mut save_trigger = false;

        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt("composer_scroll").show(ui, |ui| {
                egui::Grid::new("composer_grid")
                    .num_columns(2)
                    .spacing([12.0, 12.0])
                    .show(ui, |ui| {

                        ui.label(tr("composer-payload-format"));
                        ui.horizontal(|ui| {
                            let r1 = ui.radio_value(&mut self.composer_payload_type, PayloadType::Text, tr("composer-format-text"));
                            let r2 = ui.radio_value(&mut self.composer_payload_type, PayloadType::Hex, tr("composer-format-hex"));
                            if r1.changed() || r2.changed() {
                                self.save_config();
                            }
                        });
                        ui.end_row();
                        
                        ui.label(tr("composer-dest-addr"));
                        ui.horizontal(|ui| {
                            let mut ip_chosen = None;
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                            let edit_ip = ui.add(egui::TextEdit::singleline(&mut self.composer_ip).desired_width(120.0));
                            if edit_ip.changed() {
                                self.save_config();
                            }
                            ui.menu_button("▾", |ui| {
                                ui.set_min_width(120.0);
                                if self.composer_ip_history.is_empty() {
                                    ui.weak("No history");
                                } else {
                                    for h in &self.composer_ip_history {
                                        if ui.button(h).clicked() {
                                            ip_chosen = Some(h.clone());
                                            ui.close();
                                        }
                                    }
                                }
                            });
                            if let Some(ip) = ip_chosen {
                                self.composer_ip = ip;
                                self.save_config();
                            }
                            
                            ui.label(":");
                            
                            let mut port_chosen = None;
                            let edit_port = ui.add(egui::TextEdit::singleline(&mut self.composer_port).desired_width(60.0));
                            if edit_port.changed() {
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
                                if !self.composer_port_history.is_empty() {
                                    ui.separator();
                                    for h in &self.composer_port_history {
                                        if ui.button(h).clicked() {
                                            port_chosen = Some(h.clone());
                                            ui.close();
                                        }
                                    }
                                }
                            });
                            if let Some(port) = port_chosen {
                                self.composer_port = port;
                                self.save_config();
                            }
                        });
                        ui.end_row();
                    });
                
                ui.add_space(8.0);
                
                let current_target = format!("{}:{}", self.composer_ip, self.composer_port);
                if let Some((payload, format, target)) = self.show_echonet_lite_helper(ui, &current_target) {
                    self.composer_payload = payload;
                    self.composer_payload_type = format;
                    if let Some(idx) = target.rfind(':') {
                        let (ip, port) = target.split_at(idx);
                        self.composer_ip = ip.to_string();
                        self.composer_port = port[1..].to_string();
                    } else {
                        self.composer_ip = target;
                        self.composer_port = "3610".to_string();
                    }
                    self.save_config();
                }
                
                ui.add_space(10.0);
                ui.label(tr("composer-payload-content"));
                
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

                let payload_validation = validate_payload(&self.composer_payload, self.composer_payload_type);
                if let Err(ref err_msg) = payload_validation {
                    ui.add_space(4.0);
                    let mut args = std::collections::HashMap::new();
                    args.insert(std::borrow::Cow::Borrowed("msg"), err_msg.clone().into());
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 100, 100),
                        tr_args("composer-invalid-payload", &args)
                    );
                }
                
                ui.add_space(15.0);
                
                ui.horizontal(|ui| {
                    let is_bound = self.is_listening;
                    let is_payload_valid = payload_validation.is_ok();
                    let send_btn = ui.add_enabled(
                        is_bound && is_payload_valid, 
                        egui::Button::new(tr("composer-btn-send")).min_size(egui::vec2(120.0, 32.0))
                    );
                    
                    if send_btn.clicked() {
                        send_trigger = true;
                    }
                    
                    if !is_bound {
                        ui.colored_label(egui::Color32::from_rgb(255, 120, 120), tr("composer-start-listener-tip"));
                    }
                });
                
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);
                
                ui.heading(tr("composer-save-title"));
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(tr("composer-save-name"));
                    ui.text_edit_singleline(&mut self.composer_name);
                });
                
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.label(tr("composer-save-collection"));
                    
                    if self.collections.is_empty() {
                        ui.label(tr("composer-save-no-collections"));
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
                    
                    if ui.button(tr("composer-btn-save")).clicked() {
                        save_trigger = true;
                    }
                });
            });
        });

        // Apply deferred actions outside borrowing scopes
        if send_trigger {
            let ip = self.composer_ip.trim().to_string();
            let port = self.composer_port.trim().to_string();
            self.add_to_composer_history(ip.clone(), port.clone());
            let target = format!("{}:{}", ip, port);
            let payload_type = self.composer_payload_type;
            let payload = self.composer_payload.clone();
            self.send_packet(&target, payload_type, &payload);
        }
        if save_trigger {
            let name = if self.composer_name.trim().is_empty() {
                let total_reqs: usize = self.collections.iter().map(|c| c.requests.len()).sum();
                let mut args = std::collections::HashMap::new();
                args.insert(std::borrow::Cow::Borrowed("idx"), (total_reqs + 1).into());
                tr_args("composer-save-created-req", &args)
            } else {
                self.composer_name.clone()
            };
            
            let new_def = PacketDefinition {
                id: generate_id(),
                name,
                target_ip: self.composer_ip.clone(),
                target_port: self.composer_port.clone(),
                payload_type: self.composer_payload_type,
                payload: self.composer_payload.clone(),
            };
            
            if self.collections.is_empty() {
                self.collections.push(crate::types::Collection {
                    id: generate_id(),
                    name: tr("composer-save-default-col"),
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

