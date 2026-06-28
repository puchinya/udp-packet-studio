use eframe::egui;
use crate::UdpStudioState;
use crate::types::{PacketDefinition, PayloadType, ElBuilderProperty, generate_id, validate_payload};

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

        // Resolve DEOJ: preset 0 = custom (el_deoj_custom), preset >= 1 = MRA class (el_deoj_eoj + "01")
        let deoj_clean: String = if self.el_deoj_preset == 0 {
            // Custom mode: use the raw text field
            self.el_deoj_custom.chars().filter(|c| c.is_ascii_hexdigit()).collect()
        } else if !self.el_deoj_eoj.is_empty() {
            // MRA class selected: el_deoj_eoj is 4 hex chars (group+class), append instance "01"
            let eoj: String = self.el_deoj_eoj.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if eoj.len() == 4 {
                format!("{}01", eoj)
            } else {
                eoj
            }
        } else {
            // Fallback
            self.el_deoj_custom.chars().filter(|c| c.is_ascii_hexdigit()).collect()
        };
        if deoj_clean.len() != 6 {
            return Err(self.tr("el-err-deoj"));
        }

        let esv = match self.el_esv_preset {
            0 => "62", // Get
            1 => "61", // SetC
            2 => "60", // SetI
            3 => "73", // INF
            _ => "62",
        };
        let is_get = esv == "62";

        if self.el_properties.is_empty() {
            return Err(self.tr("el-err-epc"));
        }

        let opc = format!("{:02x}", self.el_properties.len());

        let mut props_hex = String::new();
        for prop in &self.el_properties {
            let epc_clean: String = prop.epc.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if epc_clean.len() != 2 {
                return Err(self.tr("el-err-epc"));
            }
            if is_get {
                props_hex.push_str(&epc_clean);
                props_hex.push_str("00");
            } else {
                let edt_c: String = prop.edt.chars().filter(|c| c.is_ascii_hexdigit()).collect();
                if edt_c.is_empty() {
                    return Err(self.tr("el-err-edt-empty"));
                }
                if edt_c.len() % 2 != 0 {
                    return Err(self.tr("el-err-edt-even"));
                }
                let pdc_val = edt_c.len() / 2;
                props_hex.push_str(&epc_clean);
                props_hex.push_str(&format!("{:02x}", pdc_val));
                props_hex.push_str(&edt_c);
            }
        }

        let raw_hex = format!("{}{}{}{}{}{}{}", ehd, tid_clean, seoj_clean, deoj_clean, esv, opc, props_hex);

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

        // determine which language label to use for MRA names
        let use_ja = lang_id.starts_with("ja");

        ui.checkbox(&mut self.el_show_helper, tr("el-helper-checkbox"));

        let mut result = None;
        if self.el_show_helper {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong(tr("el-builder-title"));
                ui.add_space(8.0);

                let mut generate_clicked = false;

                // ── Build sorted list of MRA classes for DEOJ dropdown ──────────────
                let mut class_list: Vec<(String, String)> = self.mra_db.classes.iter().map(|((g, c), info)| {
                    let eoj_4 = format!("{:02X}{:02X}", g, c);
                    let label = if use_ja {
                        format!("{} ({})", info.name_ja, eoj_4)
                    } else {
                        format!("{} ({})", info.name_en, eoj_4)
                    };
                    (eoj_4, label)
                }).collect();
                class_list.sort_by(|a, b| a.0.cmp(&b.0));
                // Prepend "Custom"
                class_list.insert(0, ("__custom__".to_string(), tr("el-deoj-preset-custom").to_string()));

                egui::Grid::new("el_grid_shared")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        // TID
                        ui.label(tr("el-label-tid"));
                        ui.text_edit_singleline(&mut self.el_tid);
                        ui.end_row();

                        // SEOJ
                        ui.label(tr("el-label-seoj"));
                        ui.text_edit_singleline(&mut self.el_seoj);
                        ui.end_row();

                        // DEOJ
                        ui.label(tr("el-label-deoj"));
                        ui.horizontal(|ui| {
                            // resolve current label
                            let current_deoj_label = if self.el_deoj_preset == 0 {
                                tr("el-deoj-preset-custom").to_string()
                            } else if self.el_deoj_preset - 1 < class_list.len().saturating_sub(1) {
                                class_list[self.el_deoj_preset].1.clone()
                            } else {
                                tr("el-deoj-preset-custom").to_string()
                            };

                            egui::ComboBox::from_id_salt("deoj_combo_mra")
                                .selected_text(current_deoj_label)
                                .width(260.0)
                                .show_ui(ui, |ui| {
                                    for (idx, (eoj, label)) in class_list.iter().enumerate() {
                                        let selected = &mut self.el_deoj_preset;
                                        if ui.selectable_value(selected, idx, label).clicked() {
                                            if eoj == "__custom__" {
                                                self.el_deoj_eoj = String::new();
                                            } else {
                                                self.el_deoj_eoj = eoj.clone();
                                                // auto-populate EPC list with class props
                                                if let Some(info) = self.mra_db.classes.get(&(
                                                    u8::from_str_radix(&eoj[0..2], 16).unwrap_or(0),
                                                    u8::from_str_radix(&eoj[2..4], 16).unwrap_or(0),
                                                )) {
                                                    // reset EPC list to first property of this class
                                                    let first_epc = info.properties.keys()
                                                        .filter(|&&e| e >= 0xE0) // device-specific EPCs
                                                        .copied().min()
                                                        .or_else(|| info.properties.keys().copied().min())
                                                        .map(|e| format!("{:02X}", e))
                                                        .unwrap_or_else(|| "80".to_string());
                                                    self.el_properties = vec![ElBuilderProperty { epc: first_epc, edt: String::new() }];
                                                }
                                            }
                                        }
                                    }
                                });

                            // Custom text field
                            if self.el_deoj_preset == 0 {
                                ui.text_edit_singleline(&mut self.el_deoj_custom);
                            }
                        });
                        ui.end_row();

                        // ESV
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
                    });

                // ── EPC list (multi-row) ──────────────────────────────────────────────
                let is_get = self.el_esv_preset == 0;

                // Resolve EPC dropdown items for the selected class
                let epc_list: Vec<(String, String)> = {
                    let eoj_key = if self.el_deoj_preset == 0 {
                        None
                    } else {
                        let eoj = &self.el_deoj_eoj;
                        if eoj.len() == 4 {
                            let g = u8::from_str_radix(&eoj[0..2], 16).ok();
                            let c = u8::from_str_radix(&eoj[2..4], 16).ok();
                            g.zip(c)
                        } else {
                            None
                        }
                    };

                    if let Some((g, c)) = eoj_key {
                        if let Some(info) = self.mra_db.classes.get(&(g, c)) {
                            let mut list: Vec<(String, String)> = info.properties.iter().map(|(epc, prop)| {
                                let epc_str = format!("{:02X}", epc);
                                let label = if use_ja {
                                    format!("0x{} – {}", epc_str, prop.name_ja)
                                } else {
                                    format!("0x{} – {}", epc_str, prop.name_en)
                                };
                                (epc_str, label)
                            }).collect();
                            list.sort_by(|a, b| a.0.cmp(&b.0));
                            list
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                };

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                ui.strong(tr("el-label-epc"));
                ui.add_space(4.0);

                let mut remove_idx: Option<usize> = None;
                let props_len = self.el_properties.len();

                for (i, prop) in self.el_properties.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", i + 1));

                        // EPC dropdown (if MRA class known) or text
                        if !epc_list.is_empty() {
                            let current_epc_label = epc_list.iter()
                                .find(|(e, _)| *e == prop.epc.to_uppercase())
                                .map(|(_, l)| l.clone())
                                .unwrap_or_else(|| format!("0x{}", prop.epc));
                            egui::ComboBox::from_id_salt(format!("epc_combo_{}", i))
                                .selected_text(current_epc_label)
                                .width(220.0)
                                .show_ui(ui, |ui| {
                                    for (epc_str, label) in &epc_list {
                                        if ui.selectable_label(
                                            prop.epc.to_uppercase() == *epc_str,
                                            label,
                                        ).clicked() {
                                            prop.epc = epc_str.clone();
                                        }
                                    }
                                });
                        } else {
                            ui.label("EPC:");
                            ui.add(egui::TextEdit::singleline(&mut prop.epc)
                                .desired_width(40.0)
                                .hint_text("e.g. 80"));
                        }

                        // EDT field (hidden for GET)
                        if !is_get {
                            ui.label("EDT:");
                            ui.add(egui::TextEdit::singleline(&mut prop.edt)
                                .desired_width(90.0)
                                .hint_text("hex bytes"));
                        }

                        // Remove button (only if more than 1 row)
                        if props_len > 1 && ui.small_button("✖").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }

                if let Some(idx) = remove_idx {
                    self.el_properties.remove(idx);
                }

                ui.add_space(4.0);
                if ui.small_button(tr("el-btn-add-epc")).clicked() {
                    self.el_properties.push(ElBuilderProperty {
                        epc: "80".to_string(),
                        edt: String::new(),
                    });
                }

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
            // Listener Status Warning
            let is_listening = self.get_selected_socket().map(|s| s.is_listening).unwrap_or(false);
            if !is_listening {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(45, 20, 20))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), tr("composer-start-listener-tip"));
                        });
                    });
                ui.add_space(10.0);
            }

            egui::ScrollArea::vertical().id_salt("composer_scroll").show(ui, |ui| {
                egui::Grid::new("composer_grid")
                    .num_columns(2)
                    .spacing([12.0, 12.0])
                    .show(ui, |ui| {
                        // Row 1: Target IP
                        ui.label(tr("collections-edit-target-ip"));
                        ui.horizontal(|ui| {
                            let mut ip_chosen: Option<String> = None;
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                            let edit_ip = ui.add(egui::TextEdit::singleline(&mut self.composer_ip).desired_width(120.0));
                            if edit_ip.changed() {
                                self.save_config();
                            }
                            ui.menu_button("▾", |ui| {
                                ui.set_min_width(220.0);

                                // ── Presets ──────────────────────────────────────
                                ui.strong(tr("composer-ip-preset-section"));
                                ui.separator();

                                // Loopback
                                if ui.button("127.0.0.1  (Loopback)").clicked() {
                                    ip_chosen = Some("127.0.0.1".to_string());
                                    ui.close();
                                }
                                // Global broadcast
                                if ui.button("255.255.255.255  (Broadcast)").clicked() {
                                    ip_chosen = Some("255.255.255.255".to_string());
                                    ui.close();
                                }
                                // ECHONET Lite multicast
                                if ui.button("224.0.23.0  (ECHONET Lite Multicast)").clicked() {
                                    ip_chosen = Some("224.0.23.0".to_string());
                                    ui.close();
                                }

                                // NIF broadcast addresses
                                if let Ok(ifaces) = get_if_addrs::get_if_addrs() {
                                    let mut shown_any = false;
                                    for iface in &ifaces {
                                        if let get_if_addrs::IfAddr::V4(ref v4) = iface.addr {
                                            if let Some(broadcast) = v4.broadcast {
                                                let bc_str = broadcast.to_string();
                                                // skip loopback broadcast
                                                if bc_str == "127.255.255.255" { continue; }
                                                if !shown_any {
                                                    ui.separator();
                                                    ui.weak(tr("composer-ip-preset-nif-bcast"));
                                                    shown_any = true;
                                                }
                                                let label = format!("{}  ({})", bc_str, iface.name);
                                                if ui.button(&label).clicked() {
                                                    ip_chosen = Some(bc_str);
                                                    ui.close();
                                                }
                                            }
                                        }
                                    }
                                }

                                // ── History ──────────────────────────────────────
                                if !self.composer_ip_history.is_empty() {
                                    ui.separator();
                                    ui.weak(tr("composer-ip-history-section"));
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
                        });
                        ui.end_row();

                        // Row 2: Target Port
                        ui.label(tr("collections-edit-target-port"));
                        ui.horizontal(|ui| {
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

                        // Row 3: Format
                        ui.label(tr("collections-edit-payload"));
                        ui.horizontal(|ui| {
                            let r1 = ui.radio_value(&mut self.composer_payload_type, PayloadType::Text, "Text");
                            ui.add_space(10.0);
                            let r2 = ui.radio_value(&mut self.composer_payload_type, PayloadType::Hex, "Hex")
                                .on_hover_text(tr("collections-edit-hex-tip"));
                            if r1.changed() || r2.changed() {
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
                    let is_bound = self.get_selected_socket().map(|s| s.is_listening).unwrap_or(false);
                    let is_payload_valid = payload_validation.is_ok();
                    let is_ip_valid = !self.composer_ip.trim().is_empty();
                    let is_port_valid = self.composer_port.trim().parse::<u16>().is_ok();

                    let send_btn = ui.add_enabled(
                        is_bound && is_payload_valid && is_ip_valid && is_port_valid,
                        egui::Button::new(tr("composer-btn-send")).min_size(egui::vec2(120.0, 32.0))
                    );

                    if send_btn.clicked() {
                        send_trigger = true;
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
