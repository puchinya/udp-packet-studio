use eframe::egui;
use crate::UdpStudioState;
use crate::types::{PacketDefinition, PayloadType, ElBuilderProperty, generate_id, validate_payload, FormatChangeResult};

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

        // Resolve DEOJ: use self.el_deoj_custom directly
        let deoj_clean: String = self.el_deoj_custom.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if deoj_clean.len() != 6 {
            return Err(self.tr("el-err-deoj"));
        }

        let esv = match self.el_esv_preset {
            0 => "62", // Get
            1 => "61", // SetC
            2 => "60", // SetI
            3 => "63", // INF_REQ
            4 => "73", // INF
            5 => "7A", // INFC
            6 => "6E", // SetGet
            _ => "62",
        };
        let is_get = esv == "62" || esv == "63";

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

    pub fn show_echonet_lite_helper(&mut self, ui: &mut egui::Ui, is_req: bool) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };

        // determine which language label to use for MRA names
        let use_ja = lang_id.starts_with("ja");

        let (el_tid, el_seoj, el_deoj_preset, el_deoj_custom, el_deoj_eoj, el_esv_preset, el_properties) = if is_req {
            (&mut self.req_el_tid, &mut self.req_el_seoj, &mut self.req_el_deoj_preset, &mut self.req_el_deoj_custom, &mut self.req_el_deoj_eoj, &mut self.req_el_esv_preset, &mut self.req_el_properties)
        } else {
            (&mut self.el_tid, &mut self.el_seoj, &mut self.el_deoj_preset, &mut self.el_deoj_custom, &mut self.el_deoj_eoj, &mut self.el_esv_preset, &mut self.el_properties)
        };

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong(tr("el-builder-title"));
            ui.add_space(8.0);

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

            egui::Grid::new(if is_req { "el_grid_shared_req" } else { "el_grid_shared_composer" })
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    // TID
                    ui.label(tr("el-label-tid"));
                    ui.text_edit_singleline(el_tid);
                    ui.end_row();

                    // SEOJ
                    ui.label(tr("el-label-seoj"));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(el_seoj).desired_width(70.0));
                        
                        let current_seoj = el_seoj.trim().to_uppercase();
                        let matched_seoj_label = if current_seoj.len() >= 4 {
                            class_list.iter()
                                .find(|(eoj_4, _)| current_seoj.starts_with(eoj_4))
                                .map(|(_, l)| l.clone())
                                .unwrap_or_else(|| tr("el-deoj-preset-custom").to_string())
                        } else {
                            tr("el-deoj-preset-custom").to_string()
                        };

                        egui::ComboBox::from_id_salt(if is_req { "seoj_combo_mra_req" } else { "seoj_combo_mra" })
                            .selected_text(matched_seoj_label)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for (eoj_4, label) in &class_list {
                                    if eoj_4 == "__custom__" {
                                        // custom
                                    } else {
                                        let is_selected = current_seoj.starts_with(eoj_4);
                                        if ui.selectable_label(is_selected, label).clicked() {
                                            let inst = if current_seoj.len() >= 6 { &current_seoj[4..6] } else { "01" };
                                            *el_seoj = format!("{}{}", eoj_4, inst);
                                        }
                                    }
                                }
                            });
                    });
                    ui.end_row();

                    // DEOJ
                    ui.label(tr("el-label-deoj"));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(el_deoj_custom).desired_width(70.0));
                        
                        let current_deoj = el_deoj_custom.trim().to_uppercase();
                        let matched_deoj_label = if current_deoj.len() >= 4 {
                            class_list.iter()
                                .find(|(eoj_4, _)| current_deoj.starts_with(eoj_4))
                                .map(|(_, l)| l.clone())
                                .unwrap_or_else(|| tr("el-deoj-preset-custom").to_string())
                        } else {
                            tr("el-deoj-preset-custom").to_string()
                        };

                        egui::ComboBox::from_id_salt(if is_req { "deoj_combo_mra_req" } else { "deoj_combo_mra" })
                            .selected_text(matched_deoj_label)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for (idx, (eoj_4, label)) in class_list.iter().enumerate() {
                                    if eoj_4 == "__custom__" {
                                        if ui.selectable_label(*el_deoj_preset == 0, label).clicked() {
                                            *el_deoj_preset = 0;
                                            *el_deoj_eoj = String::new();
                                        }
                                    } else {
                                        let is_selected = current_deoj.starts_with(eoj_4);
                                        if ui.selectable_label(is_selected, label).clicked() {
                                            *el_deoj_preset = idx;
                                            *el_deoj_eoj = eoj_4.clone();
                                            let inst = if current_deoj.len() >= 6 { &current_deoj[4..6] } else { "01" };
                                            *el_deoj_custom = format!("{}{}", eoj_4, inst);

                                            // auto-populate EPC list with class props
                                            if let Some(info) = self.mra_db.classes.get(&(
                                                u8::from_str_radix(&eoj_4[0..2], 16).unwrap_or(0),
                                                u8::from_str_radix(&eoj_4[2..4], 16).unwrap_or(0),
                                            )) {
                                                let first_epc = info.properties.keys()
                                                    .filter(|&&e| e >= 0xE0) // device-specific EPCs
                                                    .copied().min()
                                                    .or_else(|| info.properties.keys().copied().min())
                                                    .map(|e| format!("{:02X}", e))
                                                    .unwrap_or_else(|| "80".to_string());
                                                *el_properties = vec![ElBuilderProperty { epc: first_epc, edt: String::new() }];
                                            }
                                        }
                                    }
                                }
                            });
                    });
                    ui.end_row();

                    // ESV
                    ui.label(tr("el-label-esv"));
                    let esv_label = match *el_esv_preset {
                        0 => tr("el-esv-preset-get"),
                        1 => tr("el-esv-preset-setc"),
                        2 => tr("el-esv-preset-seti"),
                        3 => tr("el-esv-preset-infreq"),
                        4 => tr("el-esv-preset-inf"),
                        5 => tr("el-esv-preset-infc"),
                        6 => tr("el-esv-preset-setget"),
                        _ => tr("el-esv-preset-get").to_string(),
                    };
                    egui::ComboBox::from_id_salt(if is_req { "esv_combo_shared_req" } else { "esv_combo_shared" })
                        .selected_text(esv_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(el_esv_preset, 0, tr("el-esv-preset-get"));
                            ui.selectable_value(el_esv_preset, 1, tr("el-esv-preset-setc"));
                            ui.selectable_value(el_esv_preset, 2, tr("el-esv-preset-seti"));
                            ui.selectable_value(el_esv_preset, 3, tr("el-esv-preset-infreq"));
                            ui.selectable_value(el_esv_preset, 4, tr("el-esv-preset-inf"));
                            ui.selectable_value(el_esv_preset, 5, tr("el-esv-preset-infc"));
                            ui.selectable_value(el_esv_preset, 6, tr("el-esv-preset-setget"));
                        });
                    ui.end_row();
                });

            // ── EPC list (multi-row) ──────────────────────────────────────────────
            // preset 0=Get, 3=INF_REQ -> no EDT needed
            let is_get = *el_esv_preset == 0 || *el_esv_preset == 3;

            // Resolve EPC dropdown items for the selected class
            let epc_list: Vec<(String, String)> = {
                let deoj_raw = el_deoj_custom.trim().to_uppercase();
                let deoj_clean = deoj_raw.trim_start_matches("0X");
                let eoj_key = if deoj_clean.len() >= 4 {
                    let g = u8::from_str_radix(&deoj_clean[0..2], 16).ok();
                    let c = u8::from_str_radix(&deoj_clean[2..4], 16).ok();
                    g.zip(c)
                } else {
                    None
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
            let props_len = el_properties.len();

            for (i, prop) in el_properties.iter_mut().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("#{}", i + 1));

                    // EPC Group
                    ui.horizontal(|ui| {
                        ui.label("EPC:");
                        ui.add(egui::TextEdit::singleline(&mut prop.epc).desired_width(30.0));

                        // EPC dropdown (if MRA class properties are available)
                        if !epc_list.is_empty() {
                            let epc_raw = prop.epc.trim().to_uppercase();
                            let epc_clean = epc_raw.trim_start_matches("0X");
                            let current_epc_label = epc_list.iter()
                                .find(|(e, _)| *e == epc_clean)
                                .map(|(_, l)| l.clone())
                                .unwrap_or_else(|| format!("Custom (0x{})", prop.epc));
                            egui::ComboBox::from_id_salt(format!("epc_combo_{}_{}", if is_req { "req" } else { "composer" }, i))
                                .selected_text(current_epc_label)
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    for (epc_str, label) in &epc_list {
                                        let epc_raw = prop.epc.trim().to_uppercase();
                                        let epc_clean = epc_raw.trim_start_matches("0X");
                                        let is_selected = epc_clean == *epc_str;
                                        if ui.selectable_label(is_selected, label).clicked() {
                                            prop.epc = epc_str.clone();
                                        }
                                    }
                                });
                        }
                    });

                    // EDT Group (hidden for GET)
                    if !is_get {
                        ui.horizontal(|ui| {
                            ui.label("EDT:");
                            ui.add(egui::TextEdit::singleline(&mut prop.edt)
                                .desired_width(50.0)
                                .hint_text("hex"));

                            // Resolve EDT candidates from MRA property info
                            let edt_candidates = {
                                let deoj_raw = el_deoj_custom.trim().to_uppercase();
                                let deoj_clean = deoj_raw.trim_start_matches("0X");
                                let class_key = if deoj_clean.len() >= 4 {
                                    let g = u8::from_str_radix(&deoj_clean[0..2], 16).ok();
                                    let c = u8::from_str_radix(&deoj_clean[2..4], 16).ok();
                                    g.zip(c)
                                } else {
                                    None
                                };

                                let epc_raw = prop.epc.trim().to_uppercase();
                                let epc_clean = epc_raw.trim_start_matches("0X");
                                let epc_val = u8::from_str_radix(epc_clean, 16).ok();
                                
                                if let (Some((g, c)), Some(epc)) = (class_key, epc_val) {
                                    self.mra_db.classes.get(&(g, c))
                                        .and_then(|info| info.properties.get(&epc))
                                        .map(|prop_info| prop_info.edt_candidates.clone())
                                        .unwrap_or_default()
                                } else {
                                    Vec::new()
                                }
                            };

                            if !edt_candidates.is_empty() {
                                let current_edt_label = edt_candidates.iter()
                                    .find(|(val, _, _)| val.to_uppercase() == prop.edt.trim().to_uppercase())
                                    .map(|(val, name_ja, name_en)| {
                                        let name = if use_ja { name_ja } else { name_en };
                                        format!("0x{} – {}", val, name)
                                    })
                                    .unwrap_or_else(|| format!("Custom (0x{})", prop.edt));

                                egui::ComboBox::from_id_salt(format!("edt_combo_{}_{}", if is_req { "req" } else { "composer" }, i))
                                    .selected_text(current_edt_label)
                                    .width(180.0)
                                    .show_ui(ui, |ui| {
                                        for (val, name_ja, name_en) in &edt_candidates {
                                            let name = if use_ja { name_ja } else { name_en };
                                            let label = format!("0x{} – {}", val, name);
                                            let is_selected = prop.edt.trim().to_uppercase() == val.to_uppercase();
                                            if ui.selectable_label(is_selected, label).clicked() {
                                                prop.edt = val.clone();
                                            }
                                        }
                                    });
                                }
                            });
                        }

                        // Remove button (only if more than 1 row)
                        if props_len > 1 && ui.small_button("✖").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                    ui.add_space(4.0);
                }

                if let Some(idx) = remove_idx {
                    el_properties.remove(idx);
                }

                ui.add_space(4.0);
                if ui.small_button(tr("el-btn-add-epc")).clicked() {
                    el_properties.push(ElBuilderProperty {
                        epc: "80".to_string(),
                        edt: String::new(),
                    });
                }
            });
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

        let is_helper_active = self.composer_payload_type == PayloadType::EchonetLite
            || self.composer_payload_type == PayloadType::Syslog
            || self.composer_payload_type == PayloadType::Snmp
            || self.composer_payload_type == PayloadType::Dns
            || self.composer_payload_type == PayloadType::Coap;

        if is_helper_active {
            if let Ok(bytes) = self.generate_helper_bytes(false, self.composer_payload_type) {
                self.composer_payload = match self.composer_payload_type {
                    PayloadType::EchonetLite | PayloadType::Snmp | PayloadType::Dns | PayloadType::Coap => {
                        bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ")
                    }
                    PayloadType::Syslog => {
                        String::from_utf8(bytes).unwrap_or_default()
                    }
                    _ => String::new(),
                };
            }
        }

        ui.vertical(|ui| {
            // Listener Status Warning
            let is_listening = self.get_selected_socket().map(|s| s.is_listening).unwrap_or(false);
            if !is_listening {
                let is_dark = self.is_dark_theme(ui.ctx());
                let (warn_bg, warn_fg) = if is_dark {
                    (egui::Color32::from_rgb(45, 20, 20), egui::Color32::from_rgb(255, 120, 120))
                } else {
                    (egui::Color32::from_rgb(253, 237, 237), egui::Color32::from_rgb(95, 33, 32))
                };
                egui::Frame::NONE
                    .fill(warn_bg)
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(warn_fg, tr("composer-start-listener-tip"));
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
                                 // Multicast Submenu
                                 ui.menu_button(tr("composer-ip-preset-multicast"), |ui| {
                                     if ui.button("224.0.23.0  (ECHONET Lite)").clicked() {
                                         ip_chosen = Some("224.0.23.0".to_string());
                                         ui.close();
                                     }
                                     if ui.button("224.0.0.251  (mDNS IPv4)").clicked() {
                                         ip_chosen = Some("224.0.0.251".to_string());
                                         ui.close();
                                     }
                                 });

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
                                ui.menu_button(tr("composer-port-preset-section"), |ui| {
                                    for item in &self.preset_ports_order {
                                        let label = format!("{} : {}", item.protocol, item.port);
                                        if ui.button(&label).clicked() {
                                            port_chosen = Some((Some(item.protocol.clone()), item.port.clone()));
                                            ui.close();
                                        }
                                    }
                                });
                                if !self.composer_port_history.is_empty() {
                                    ui.separator();
                                    for h in &self.composer_port_history {
                                        if ui.button(h).clicked() {
                                            port_chosen = Some((None, h.clone()));
                                            ui.close();
                                        }
                                    }
                                }
                            });
                            if let Some((opt_proto, port)) = port_chosen {
                                self.composer_port = port;
                                if let Some(proto) = opt_proto {
                                    let port_val = self.composer_port.clone();
                                    self.record_preset_port_usage(&proto, &port_val);
                                } else {
                                    self.save_config();
                                }
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(tr("collections-edit-format"));
                    ui.add_space(8.0);

                    let avail_w = ui.available_width();
                    let mut selected_format = match self.composer_payload_type {
                        PayloadType::Text => 0,
                        PayloadType::Hex => 1,
                        _ => 2,
                    };

                    let mut r1_changed = false;
                    let mut r2_changed = false;
                    let mut r3_changed = false;
                    let mut dropdown_changed_to = None;

                    if avail_w < 280.0 {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let r1 = ui.radio_value(&mut selected_format, 0, "Text");
                                r1_changed = r1.changed();
                                ui.add_space(10.0);
                                let r2 = ui.radio_value(&mut selected_format, 1, "Hex")
                                    .on_hover_text(tr("collections-edit-hex-tip"));
                                r2_changed = r2.changed();
                            });
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                let r3 = ui.radio_value(&mut selected_format, 2, "");
                                r3_changed = r3.changed();

                                let active_proto = match self.composer_payload_type {
                                    PayloadType::EchonetLite => PayloadType::EchonetLite,
                                    PayloadType::Syslog => PayloadType::Syslog,
                                    PayloadType::Snmp => PayloadType::Snmp,
                                    PayloadType::Dns => PayloadType::Dns,
                                    PayloadType::Coap => PayloadType::Coap,
                                    _ => self.composer_selected_proto,
                                };
                                let current_proto_name = match active_proto {
                                    PayloadType::EchonetLite => "ECHONET Lite",
                                    PayloadType::Syslog => "Syslog",
                                    PayloadType::Snmp => "SNMP",
                                    _ => "ECHONET Lite",
                                };

                                egui::ComboBox::from_id_salt("composer_protocol_select")
                                    .selected_text(current_proto_name)
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        for proto in &self.protocol_mru {
                                            if ui.selectable_label(current_proto_name == proto, proto).clicked() {
                                                dropdown_changed_to = Some(proto.clone());
                                            }
                                        }
                                    });
                            });
                        });
                    } else {
                        let r1 = ui.radio_value(&mut selected_format, 0, "Text");
                        r1_changed = r1.changed();
                        ui.add_space(10.0);
                        let r2 = ui.radio_value(&mut selected_format, 1, "Hex")
                            .on_hover_text(tr("collections-edit-hex-tip"));
                        r2_changed = r2.changed();
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let r3 = ui.radio_value(&mut selected_format, 2, "");
                            r3_changed = r3.changed();

                            let active_proto = match self.composer_payload_type {
                                PayloadType::EchonetLite => PayloadType::EchonetLite,
                                PayloadType::Syslog => PayloadType::Syslog,
                                PayloadType::Snmp => PayloadType::Snmp,
                                PayloadType::Dns => PayloadType::Dns,
                                PayloadType::Coap => PayloadType::Coap,
                                _ => self.composer_selected_proto,
                            };
                            let current_proto_name = match active_proto {
                                PayloadType::EchonetLite => "ECHONET Lite",
                                PayloadType::Syslog => "Syslog",
                                PayloadType::Snmp => "SNMP",
                                PayloadType::Dns => "DNS",
                                PayloadType::Coap => "CoAP",
                                _ => "ECHONET Lite",
                            };

                            egui::ComboBox::from_id_salt("composer_protocol_select")
                                .selected_text(current_proto_name)
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for proto in &self.protocol_mru {
                                        if ui.selectable_label(current_proto_name == proto, proto).clicked() {
                                            dropdown_changed_to = Some(proto.clone());
                                        }
                                    }
                                });
                        });
                    }

                    if r1_changed || r2_changed || r3_changed {
                        let to_type = match selected_format {
                            0 => PayloadType::Text,
                            1 => PayloadType::Hex,
                            _ => {
                                self.composer_selected_proto
                            }
                        };
                        let current_payload = self.composer_payload.clone();
                        let res = self.change_payload_format(false, None, self.composer_payload_type, to_type, &current_payload);
                        match res {
                            FormatChangeResult::Immediate { new_payload } => {
                                self.composer_payload_type = to_type;
                                self.composer_payload = new_payload;
                                if selected_format == 2 {
                                    self.composer_selected_proto = to_type;
                                    let proto_name = match to_type {
                                        PayloadType::EchonetLite => "ECHONET Lite",
                                        PayloadType::Syslog => "Syslog",
                                        PayloadType::Snmp => "SNMP",
                                        PayloadType::Dns => "DNS",
                                        PayloadType::Coap => "CoAP",
                                        _ => "",
                                    };
                                    if !proto_name.is_empty() {
                                        self.update_protocol_mru(proto_name);
                                    }
                                }
                                self.save_config();
                            }
                            FormatChangeResult::Pending(pending) => {
                                self.composer_pending_format_change = Some(pending);
                            }
                        }
                    }

                    if let Some(new_proto) = dropdown_changed_to {
                        let to_type = match new_proto.as_str() {
                            "ECHONET Lite" => PayloadType::EchonetLite,
                            "Syslog" => PayloadType::Syslog,
                            "SNMP" => PayloadType::Snmp,
                            "DNS" => PayloadType::Dns,
                            "CoAP" => PayloadType::Coap,
                            _ => PayloadType::EchonetLite,
                        };
                        self.composer_selected_proto = to_type;
                        let current_payload = self.composer_payload.clone();
                        let res = self.change_payload_format(false, None, self.composer_payload_type, to_type, &current_payload);
                        match res {
                            FormatChangeResult::Immediate { new_payload } => {
                                self.composer_payload_type = to_type;
                                self.composer_payload = new_payload;
                                self.composer_selected_proto = to_type;
                                self.update_protocol_mru(&new_proto);
                                self.save_config();
                            }
                            FormatChangeResult::Pending(pending) => {
                                self.composer_pending_format_change = Some(pending);
                            }
                        }
                    }
                });

                ui.add_space(8.0);

                // Helper views rendering reactively based on format selection
                if self.composer_payload_type == PayloadType::EchonetLite {
                    self.show_echonet_lite_helper(ui, false);
                } else if self.composer_payload_type == PayloadType::Syslog {
                    self.show_syslog_helper(ui, false);
                } else if self.composer_payload_type == PayloadType::Snmp {
                    self.show_snmp_helper(ui, false);
                } else if self.composer_payload_type == PayloadType::Dns {
                    self.show_dns_helper(ui, false);
                } else if self.composer_payload_type == PayloadType::Coap {
                    self.show_coap_helper(ui, false);
                }

                ui.add_space(10.0);

                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.composer_payload)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_rows(8)
                        .desired_width(ui.available_width())
                        .interactive(!is_helper_active)
                );
                if response.changed() {
                    self.save_config();
                }

                let payload_validation = validate_payload(&self.composer_payload, self.composer_payload_type);
                if let Err(ref err_msg) = payload_validation {
                    ui.add_space(4.0);
                    let mut args = std::collections::HashMap::new();
                    args.insert(std::borrow::Cow::Borrowed("msg"), err_msg.clone().into());
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr_args("composer-invalid-payload", &args))
                                .color(egui::Color32::from_rgb(255, 100, 100))
                        ).wrap()
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
            self.send_packet(&target, payload_type, &payload, false);
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

    pub fn show_syslog_helper(&mut self, ui: &mut egui::Ui, is_req: bool) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };

        let (
            syslog_protocol_version,
            syslog_facility,
            syslog_severity,
            syslog_auto_timestamp,
            syslog_timestamp,
            syslog_hostname,
            syslog_app_name,
            syslog_proc_id,
            syslog_msg_id,
            syslog_msg,
        ) = if is_req {
            (
                &mut self.req_syslog_protocol_version,
                &mut self.req_syslog_facility,
                &mut self.req_syslog_severity,
                &mut self.req_syslog_auto_timestamp,
                &mut self.req_syslog_timestamp,
                &mut self.req_syslog_hostname,
                &mut self.req_syslog_app_name,
                &mut self.req_syslog_proc_id,
                &mut self.req_syslog_msg_id,
                &mut self.req_syslog_msg,
            )
        } else {
            (
                &mut self.syslog_protocol_version,
                &mut self.syslog_facility,
                &mut self.syslog_severity,
                &mut self.syslog_auto_timestamp,
                &mut self.syslog_timestamp,
                &mut self.syslog_hostname,
                &mut self.syslog_app_name,
                &mut self.syslog_proc_id,
                &mut self.syslog_msg_id,
                &mut self.syslog_msg,
            )
        };

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("Syslog Builder");
            ui.add_space(8.0);

            egui::Grid::new(if is_req { "syslog_grid_shared_req" } else { "syslog_grid_shared" })
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    // Protocol Version
                    ui.label(tr("syslog-version"));
                    egui::ComboBox::from_id_salt(if is_req { "syslog_version_combo_req" } else { "syslog_version_combo" })
                        .selected_text(if *syslog_protocol_version == 0 { "RFC 3164 (BSD)" } else { "RFC 5424 (IETF)" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(syslog_protocol_version, 0, "RFC 3164 (BSD)");
                            ui.selectable_value(syslog_protocol_version, 1, "RFC 5424 (IETF)");
                        });
                    ui.end_row();

                    // Facility
                    ui.label(tr("ins-syslog-facility"));
                    let current_fac = *syslog_facility as u8;
                    let fac_label = format!("{} ({})", current_fac, crate::syslog::facility_name(current_fac));
                    egui::ComboBox::from_id_salt(if is_req { "syslog_facility_combo_req" } else { "syslog_facility_combo" })
                        .selected_text(fac_label)
                        .show_ui(ui, |ui| {
                            for f in 0..24 {
                                let label = format!("{} ({})", f, crate::syslog::facility_name(f));
                                ui.selectable_value(syslog_facility, f as usize, label);
                            }
                        });
                    ui.end_row();

                    // Severity
                    ui.label(tr("ins-syslog-severity"));
                    let current_sev = *syslog_severity as u8;
                    let sev_label = crate::syslog::severity_name(current_sev);
                    egui::ComboBox::from_id_salt(if is_req { "syslog_severity_combo_req" } else { "syslog_severity_combo" })
                        .selected_text(sev_label)
                        .show_ui(ui, |ui| {
                            for s in 0..8 {
                                let label = crate::syslog::severity_name(s);
                                ui.selectable_value(syslog_severity, s as usize, label);
                            }
                        });
                    ui.end_row();

                    // Auto Timestamp
                    ui.label("");
                    ui.checkbox(syslog_auto_timestamp, tr("syslog-auto-ts"));
                    ui.end_row();

                    // Custom Timestamp (if auto is disabled)
                    if !*syslog_auto_timestamp {
                        ui.label(tr("ins-syslog-timestamp"));
                        ui.text_edit_singleline(syslog_timestamp);
                        ui.end_row();
                    }

                    // Hostname
                    ui.label(tr("syslog-hostname-lbl"));
                    ui.text_edit_singleline(syslog_hostname);
                    ui.end_row();

                    // App Name
                    ui.label(tr("syslog-appname-lbl"));
                    ui.text_edit_singleline(syslog_app_name);
                    ui.end_row();

                    if *syslog_protocol_version == 1 {
                        // Proc ID
                        ui.label(tr("syslog-procid-lbl"));
                        ui.text_edit_singleline(syslog_proc_id);
                        ui.end_row();

                        // Msg ID
                        ui.label(tr("syslog-msgid-lbl"));
                        ui.text_edit_singleline(syslog_msg_id);
                        ui.end_row();
                    }

                    // Message
                    ui.label(tr("syslog-msg-lbl"));
                    ui.text_edit_singleline(syslog_msg);
                    ui.end_row();
                });
        });
    }

    pub fn show_snmp_helper(&mut self, ui: &mut egui::Ui, is_req: bool) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };

        let (
            snmp_version,
            snmp_community,
            snmp_pdu_type,
            snmp_request_id,
            snmp_error_status,
            snmp_error_index,
            snmp_varbinds,
        ) = if is_req {
            (
                &mut self.req_snmp_version,
                &mut self.req_snmp_community,
                &mut self.req_snmp_pdu_type,
                &mut self.req_snmp_request_id,
                &mut self.req_snmp_error_status,
                &mut self.req_snmp_error_index,
                &mut self.req_snmp_varbinds,
            )
        } else {
            (
                &mut self.snmp_version,
                &mut self.snmp_community,
                &mut self.snmp_pdu_type,
                &mut self.snmp_request_id,
                &mut self.snmp_error_status,
                &mut self.snmp_error_index,
                &mut self.snmp_varbinds,
            )
        };

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("SNMP Builder");
            ui.add_space(8.0);

            egui::Grid::new(if is_req { "snmp_grid_shared_req" } else { "snmp_grid_shared" })
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    // SNMP Version
                    ui.label(tr("snmp-version-lbl"));
                    egui::ComboBox::from_id_salt(if is_req { "snmp_version_combo_req" } else { "snmp_version_combo" })
                        .selected_text(if *snmp_version == 0 { "v1" } else { "v2c" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(snmp_version, 0, "v1");
                            ui.selectable_value(snmp_version, 1, "v2c");
                        });
                    ui.end_row();

                    // Community
                    ui.label(tr("snmp-community-lbl"));
                    ui.text_edit_singleline(snmp_community);
                    ui.end_row();

                    // PDU Type
                    ui.label(tr("snmp-pdutype-lbl"));
                    let pdu_label = match *snmp_pdu_type {
                        0 => "GetRequest (0xa0)",
                        1 => "GetNextRequest (0xa1)",
                        2 => "SetRequest (0xa3)",
                        3 => "Trap (v2) (0xa7)",
                        _ => "GetRequest (0xa0)",
                    };
                    egui::ComboBox::from_id_salt(if is_req { "snmp_pdu_combo_req" } else { "snmp_pdu_combo" })
                        .selected_text(pdu_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(snmp_pdu_type, 0, "GetRequest (0xa0)");
                            ui.selectable_value(snmp_pdu_type, 1, "GetNextRequest (0xa1)");
                            ui.selectable_value(snmp_pdu_type, 2, "SetRequest (0xa3)");
                            ui.selectable_value(snmp_pdu_type, 3, "Trap (v2) (0xa7)");
                        });
                    ui.end_row();

                    // Request ID
                    ui.label(tr("snmp-reqid-lbl"));
                    ui.add(egui::DragValue::new(snmp_request_id));
                    ui.end_row();

                    if *snmp_pdu_type == 2 || *snmp_pdu_type == 3 {
                        // Error Status
                        ui.label(tr("snmp-errstatus-lbl"));
                        ui.add(egui::DragValue::new(snmp_error_status));
                        ui.end_row();

                        // Error Index
                        ui.label(tr("snmp-errindex-lbl"));
                        ui.add(egui::DragValue::new(snmp_error_index));
                        ui.end_row();
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.strong(tr("ins-snmp-varbinds"));
            ui.add_space(4.0);

            let mut remove_idx = None;
            let varbinds_len = snmp_varbinds.len();

            egui::ScrollArea::vertical()
                .id_salt(if is_req { "snmp_vars_scroll_req" } else { "snmp_vars_scroll" })
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, vb) in snmp_varbinds.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{}", i + 1));
                                
                                ui.label(tr("snmp-varbind-oid"));
                                ui.add(egui::TextEdit::singleline(&mut vb.oid).desired_width(120.0));

                                if varbinds_len > 1 && ui.small_button("✖").clicked() {
                                    remove_idx = Some(i);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(tr("snmp-varbind-type"));
                                let type_label = match vb.value_type {
                                    crate::types::SnmpValueType::Integer => "Integer",
                                    crate::types::SnmpValueType::OctetString => "Octet String",
                                    crate::types::SnmpValueType::ObjectId => "Object ID",
                                    crate::types::SnmpValueType::Null => "Null",
                                    crate::types::SnmpValueType::IpAddress => "IP Address",
                                    crate::types::SnmpValueType::Counter32 => "Counter32",
                                    crate::types::SnmpValueType::Gauge32 => "Gauge32",
                                    crate::types::SnmpValueType::TimeTicks => "TimeTicks",
                                };
                                egui::ComboBox::from_id_salt(format!("snmp_valtype_{}_{}", if is_req { "req" } else { "composer" }, i))
                                    .selected_text(type_label)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::Null, "Null");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::Integer, "Integer");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::OctetString, "Octet String");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::ObjectId, "Object ID");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::IpAddress, "IP Address");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::Counter32, "Counter32");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::Gauge32, "Gauge32");
                                        ui.selectable_value(&mut vb.value_type, crate::types::SnmpValueType::TimeTicks, "TimeTicks");
                                    });

                                if vb.value_type != crate::types::SnmpValueType::Null {
                                    ui.label(tr("snmp-varbind-val"));
                                    ui.add(egui::TextEdit::singleline(&mut vb.value).desired_width(120.0));
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }
                });

            if let Some(idx) = remove_idx {
                snmp_varbinds.remove(idx);
            }

            ui.add_space(4.0);
            if ui.small_button(tr("snmp-varbind-add")).clicked() {
                snmp_varbinds.push(crate::types::SnmpVarBindState {
                    oid: "1.3.6.1.2.1.1.1.0".to_string(),
                    value_type: crate::types::SnmpValueType::Null,
                    value: String::new(),
                });
            }
        });
    }

    pub fn show_dns_helper(&mut self, ui: &mut egui::Ui, is_req: bool) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };

        let (
            dns_transaction_id,
            dns_flags,
            dns_qname,
            dns_qtype,
            dns_qclass,
        ) = if is_req {
            (
                &mut self.req_dns_transaction_id,
                &mut self.req_dns_flags,
                &mut self.req_dns_qname,
                &mut self.req_dns_qtype,
                &mut self.req_dns_qclass,
            )
        } else {
            (
                &mut self.dns_transaction_id,
                &mut self.dns_flags,
                &mut self.dns_qname,
                &mut self.dns_qtype,
                &mut self.dns_qclass,
            )
        };

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("DNS / mDNS Builder");
            ui.add_space(8.0);

            egui::Grid::new(if is_req { "dns_grid_req" } else { "dns_grid" })
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    // Transaction ID
                    ui.label(tr("dns-tid"));
                    ui.add(egui::DragValue::new(dns_transaction_id));
                    ui.end_row();

                    // Flags
                    ui.label(tr("dns-flags"));
                    let mut flags_preset = if *dns_flags == 0x0100 {
                        0
                    } else if *dns_flags == 0x0000 {
                        1
                    } else {
                        2
                    };
                    
                    let original_visuals = if flags_preset == 2 {
                        let orig = ui.visuals().clone();
                        let visuals = ui.visuals_mut();
                        visuals.widgets.inactive.bg_fill = visuals.selection.bg_fill;
                        visuals.widgets.inactive.fg_stroke = visuals.selection.stroke;
                        Some(orig)
                    } else {
                        None
                    };

                    let combo_label = match flags_preset {
                        0 => "Standard Query (0x0100)".to_string(),
                        1 => "mDNS Query (0x0000)".to_string(),
                        _ => format!("Custom (0x{:04X})", *dns_flags),
                    };

                    egui::ComboBox::from_id_salt(if is_req { "dns_flags_combo_req" } else { "dns_flags_combo" })
                        .selected_text(combo_label)
                        .show_ui(ui, |ui| {
                            if let Some(ref orig) = original_visuals {
                                *ui.visuals_mut() = orig.clone();
                            }
                            if ui.selectable_value(&mut flags_preset, 0, "Standard Query (0x0100)").clicked() {
                                *dns_flags = 0x0100;
                            }
                            if ui.selectable_value(&mut flags_preset, 1, "mDNS Query (0x0000)").clicked() {
                                *dns_flags = 0x0000;
                            }
                            ui.selectable_value(&mut flags_preset, 2, "Custom...");
                        });
                    
                    if let Some(orig) = original_visuals {
                        *ui.visuals_mut() = orig;
                    }
                    ui.end_row();

                    if flags_preset == 2 {
                        ui.label("");
                        ui.horizontal(|ui| {
                            ui.label("0x");
                            let mut val = *dns_flags;
                            let res = ui.add(egui::DragValue::new(&mut val).custom_formatter(|v, _| format!("{:04X}", v as u32)));
                            if res.changed() {
                                *dns_flags = val;
                            }
                        });
                        ui.end_row();
                    }

                    // Query Name (QNAME)
                    ui.label(tr("dns-qname"));
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(dns_qname);
                        ui.menu_button("▾", |ui| {
                            if ui.button("_services._dns-sd._udp.local  (mDNS Service Discovery)").clicked() {
                                *dns_qname = "_services._dns-sd._udp.local".to_string();
                                *dns_qtype = 12; // PTR
                                *dns_flags = 0x0000;
                                ui.close();
                            }
                            if ui.button("local  (mDNS Root)").clicked() {
                                *dns_qname = "local".to_string();
                                ui.close();
                            }
                            if ui.button("google.com  (DNS Sample)").clicked() {
                                *dns_qname = "google.com".to_string();
                                *dns_qtype = 1; // A
                                *dns_flags = 0x0100; // Standard Query
                                ui.close();
                            }
                        });
                    });
                    ui.end_row();

                    // Query Type (QTYPE)
                    ui.label(tr("dns-qtype"));
                    let current_type_name = crate::dns::qtype_name(*dns_qtype);
                    let type_label = format!("{} ({})", current_type_name, *dns_qtype);
                    egui::ComboBox::from_id_salt(if is_req { "dns_qtype_combo_req" } else { "dns_qtype_combo" })
                        .selected_text(type_label)
                        .show_ui(ui, |ui| {
                            let types = [
                                ("A", 1),
                                ("AAAA", 28),
                                ("PTR", 12),
                                ("TXT", 16),
                                ("SRV", 33),
                                ("ANY", 255),
                                ("MX", 15),
                                ("NS", 2),
                                ("CNAME", 5),
                            ];
                            for (name, val) in &types {
                                ui.selectable_value(dns_qtype, *val, format!("{} ({})", name, val));
                            }
                        });
                    ui.end_row();

                    // Query Class (QCLASS)
                    ui.label(tr("dns-qclass"));
                    let current_class_name = crate::dns::qclass_name(*dns_qclass);
                    let class_label = format!("{} (0x{:04X})", current_class_name, *dns_qclass);
                    egui::ComboBox::from_id_salt(if is_req { "dns_qclass_combo_req" } else { "dns_qclass_combo" })
                        .selected_text(class_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(dns_qclass, 1, "IN (0x0001)");
                            ui.selectable_value(dns_qclass, 0x8001, "IN (unicast-response) (0x8001)");
                        });
                    ui.end_row();
                });
        });
    }

    pub fn show_coap_helper(&mut self, ui: &mut egui::Ui, is_req: bool) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };

        let (
            coap_version,
            coap_mtype,
            coap_code,
            coap_message_id,
            coap_token,
            coap_options,
            coap_payload,
        ) = if is_req {
            (
                &mut self.req_coap_version,
                &mut self.req_coap_mtype,
                &mut self.req_coap_code,
                &mut self.req_coap_message_id,
                &mut self.req_coap_token,
                &mut self.req_coap_options,
                &mut self.req_coap_payload,
            )
        } else {
            (
                &mut self.coap_version,
                &mut self.coap_mtype,
                &mut self.coap_code,
                &mut self.coap_message_id,
                &mut self.coap_token,
                &mut self.coap_options,
                &mut self.coap_payload,
            )
        };

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("CoAP Builder");
            ui.add_space(8.0);

            egui::Grid::new(if is_req { "coap_grid_req" } else { "coap_grid" })
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    // Version
                    ui.label(tr("coap-version-lbl"));
                    ui.add(egui::DragValue::new(coap_version).range(0..=3));
                    ui.end_row();

                    // Message Type
                    ui.label(tr("coap-type-lbl"));
                    let mtype_label = match *coap_mtype {
                        0 => "Confirmable (CON)",
                        1 => "Non-confirmable (NON)",
                        2 => "Acknowledgement (ACK)",
                        3 => "Reset (RST)",
                        _ => "Confirmable (CON)",
                    };
                    egui::ComboBox::from_id_salt(if is_req { "coap_mtype_combo_req" } else { "coap_mtype_combo" })
                        .selected_text(mtype_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(coap_mtype, 0, "Confirmable (CON)");
                            ui.selectable_value(coap_mtype, 1, "Non-confirmable (NON)");
                            ui.selectable_value(coap_mtype, 2, "Acknowledgement (ACK)");
                            ui.selectable_value(coap_mtype, 3, "Reset (RST)");
                        });
                    ui.end_row();

                    // Code
                    ui.label(tr("coap-code-lbl"));
                    let current_code_name = crate::coap::code_name(*coap_code);
                    egui::ComboBox::from_id_salt(if is_req { "coap_code_combo_req" } else { "coap_code_combo" })
                        .selected_text(&current_code_name)
                        .show_ui(ui, |ui| {
                            let codes = [
                                ("GET", 1),
                                ("POST", 2),
                                ("PUT", 3),
                                ("DELETE", 4),
                                ("2.01 Created", 65),
                                ("2.02 Deleted", 66),
                                ("2.03 Valid", 67),
                                ("2.04 Changed", 68),
                                ("2.05 Content", 69),
                                ("4.00 Bad Request", 128),
                                ("4.01 Unauthorized", 129),
                                ("4.03 Forbidden", 131),
                                ("4.04 Not Found", 132),
                                ("4.05 Method Not Allowed", 133),
                                ("5.00 Internal Server Error", 160),
                                ("5.03 Service Unavailable", 163),
                            ];
                            for (name, val) in &codes {
                                ui.selectable_value(coap_code, *val, format!("{} ({})", name, val));
                            }
                        });
                    ui.end_row();

                    // Message ID
                    ui.label(tr("coap-message-id-lbl"));
                    ui.add(egui::DragValue::new(coap_message_id));
                    ui.end_row();

                    // Token
                    ui.label(tr("coap-token-lbl"));
                    ui.text_edit_singleline(coap_token);
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.strong(tr("coap-options-title"));
            ui.add_space(4.0);

            let mut remove_idx = None;

            egui::ScrollArea::vertical()
                .id_salt(if is_req { "coap_options_scroll_req" } else { "coap_options_scroll" })
                .max_height(150.0)
                .show(ui, |ui| {
                    for (i, opt) in coap_options.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{}", i + 1));
                                
                                ui.label(tr("coap-option-num"));
                                let mut opt_num = opt.number.trim().parse::<u16>().unwrap_or(11);
                                let current_opt_name = crate::coap::option_name(opt_num);
                                let opt_label = format!("{} ({})", current_opt_name, opt_num);
                                
                                egui::ComboBox::from_id_salt(format!("coap_optnum_{}_{}", if is_req { "req" } else { "composer" }, i))
                                    .selected_text(opt_label)
                                    .show_ui(ui, |ui| {
                                        let options_list = [
                                            ("Uri-Host", 3),
                                            ("Uri-Port", 7),
                                            ("Uri-Path", 11),
                                            ("Content-Format", 12),
                                            ("Max-Age", 14),
                                            ("Uri-Query", 15),
                                            ("Accept", 17),
                                            ("Location-Path", 8),
                                            ("Location-Query", 20),
                                            ("Proxy-Uri", 35),
                                            ("Proxy-Scheme", 39),
                                            ("Size1", 60),
                                        ];
                                        for (name, val) in &options_list {
                                            if ui.selectable_value(&mut opt_num, *val, format!("{} ({})", name, val)).clicked() {
                                                opt.number = opt_num.to_string();
                                            }
                                        }
                                    });

                                ui.label(tr("coap-option-val"));
                                ui.add(egui::TextEdit::singleline(&mut opt.value).desired_width(120.0));

                                if ui.small_button("✖").clicked() {
                                    remove_idx = Some(i);
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }
                });

            if let Some(idx) = remove_idx {
                coap_options.remove(idx);
            }

            ui.add_space(4.0);
            if ui.small_button(tr("coap-option-add")).clicked() {
                coap_options.push(crate::coap::CoapOptionState {
                    number: "11".to_string(), // Uri-Path default
                    value: String::new(),
                });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.strong("Payload (Hex)");
            ui.add_space(4.0);
            ui.text_edit_singleline(coap_payload);
        });
    }
}
