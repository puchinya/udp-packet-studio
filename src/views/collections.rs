use eframe::egui;
use crate::UdpStudioState;
use crate::types::{Collection, PacketDefinition, PayloadType, generate_id, validate_payload};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct YamlRequest {
    pub name: String,
    pub target_ip: String,
    pub target_port: String,
    pub payload_type: PayloadType,
    pub payload: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct YamlCollection {
    pub name: String,
    #[serde(default)]
    pub requests: Vec<YamlRequest>,
}


impl UdpStudioState {
    pub fn show_collections(&mut self, ui: &mut egui::Ui) {
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

        let mut toggle_expand = None;
        let mut create_collection = false;
        let mut delete_collection = None;
        let mut add_request = None;
        let mut delete_request = None;
        let mut select_request = None;
        let mut needs_save = false;
        let mut load_to_composer = None;
        let mut send_trigger = None;
        let mut export_collection = None;
        let mut import_collection = false;

        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                if ui.button(tr("collections-new")).on_hover_text(tr("collections-new-tip")).clicked() {
                    create_collection = true;
                }
                if ui.button(tr("collections-import")).on_hover_text(tr("collections-import-tip")).clicked() {
                    import_collection = true;
                }
            });
            
            ui.separator();
            
            // 1. Scrollable List/Tree of Collections (Top Half)
            let tree_height = if self.selected_request_id.is_some() {
                ui.available_height() * 0.55 // Stacked: tree on top 55%, editor on bottom 45%
            } else {
                ui.available_height() // Full height if nothing selected
            };

            ui.allocate_ui(egui::vec2(ui.available_width(), tree_height), |ui| {
                egui::ScrollArea::vertical().id_salt("collections_scroll").show(ui, |ui| {
                    if self.collections.is_empty() {
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::from_rgb(120, 130, 140), egui::RichText::new(tr("collections-empty-list")).italics());
                    } else {
                        for col_idx in 0..self.collections.len() {
                            let collection = &mut self.collections[col_idx];
                            
                            // Collection Header Row
                            {
                            let row_height = ui.spacing().interact_size.y;
                            let row_width = ui.available_width();
                            ui.allocate_ui_with_layout(egui::vec2(row_width, row_height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                // Toggle Chevron
                                let caret = if collection.is_expanded { "▼" } else { "▶" };
                                if ui.add(egui::Button::new(caret).frame(false)).clicked() {
                                    toggle_expand = Some(collection.id.clone());
                                }
                                
                                ui.add(egui::Button::new("📁").frame(false).sense(egui::Sense::hover()));
                                
                                // Editable Collection Name
                                let name_edit = ui.add(
                                    egui::TextEdit::singleline(&mut collection.name)
                                        .frame(egui::Frame::NONE)
                                        .desired_width(ui.available_width() - 85.0)
                                        .hint_text(tr("collections-unnamed-col"))
                                );
                                if name_edit.changed() {
                                    needs_save = true;
                                }
                                
                                // Hover actions on the right
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🗑").on_hover_text(tr("collections-del-col-tip")).clicked() {
                                        delete_collection = Some(collection.id.clone());
                                    }
                                    if ui.button("📤").on_hover_text(tr("collections-exp-col-tip")).clicked() {
                                        export_collection = Some(collection.id.clone());
                                    }
                                    if ui.button("+").on_hover_text(tr("collections-add-req-tip")).clicked() {
                                        add_request = Some(collection.id.clone());
                                    }
                                });
                            });
                            }
                            
                            // If expanded, list requests inside
                            if collection.is_expanded {
                                if collection.requests.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.add_space(24.0);
                                        ui.colored_label(egui::Color32::from_rgb(100, 110, 120), egui::RichText::new(tr("collections-empty-col")).italics().size(11.0));
                                    });
                                } else {
                                    for req_idx in 0..collection.requests.len() {
                                        let req = &collection.requests[req_idx];
                                        let is_selected = Some(&req.id) == self.selected_request_id.as_ref();
                                        
                                        let bg_fill = if is_selected {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        };

                                        let mut delete_clicked = false;
                                        let mut send_clicked = false;
                                        let mut label_clicked = false;

                                        let frame_res = egui::Frame::NONE
                                            .fill(bg_fill)
                                            .corner_radius(egui::CornerRadius::same(4))
                                            .inner_margin(egui::Margin::symmetric(4, 2))
                                            .show(ui, |ui| {
                                                let row_height = ui.spacing().interact_size.y;
                                                let row_width = ui.available_width();
                                                ui.allocate_ui_with_layout(egui::vec2(row_width, row_height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                    ui.add_space(16.0); // Indentation adjusted slightly for frame margins
                                                    
                                                    // Method badge (UDP)
                                                    egui::Frame::NONE
                                                        .fill(egui::Color32::from_rgb(138, 43, 226))
                                                        .corner_radius(egui::CornerRadius::same(3))
                                                        .inner_margin(egui::Margin::symmetric(4, 1))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new("UDP").color(egui::Color32::WHITE).size(9.0).strong());
                                                        });
                                                    
                                                    ui.add_space(2.0);
                                                    
                                                    // Clickable request label button
                                                    let label_text = if req.name.trim().is_empty() {
                                                        tr("collections-unnamed-req")
                                                    } else {
                                                        req.name.clone()
                                                    };
                                                    
                                                    // Truncate text if too long for the sidebar
                                                    let display_name = if label_text.chars().count() > 18 {
                                                        format!("{}...", label_text.chars().take(16).collect::<String>())
                                                    } else {
                                                        label_text
                                                    };
                                                    
                                                    let rich_text = egui::RichText::new(display_name);
                                                    let rich_text = if is_selected {
                                                        rich_text.color(egui::Color32::WHITE).strong()
                                                    } else {
                                                        rich_text
                                                    };
                                                    
                                                    let btn = ui.add(
                                                        egui::Button::selectable(is_selected, rich_text)
                                                            .frame(false)
                                                    );
                                                    if btn.clicked() {
                                                        label_clicked = true;
                                                    }
                                                    
                                                    // Quick Actions on Right
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if ui.button("🗑").on_hover_text(tr("collections-del-req-tip")).clicked() {
                                                            delete_clicked = true;
                                                            delete_request = Some((collection.id.clone(), req.id.clone()));
                                                        }
                                                        let is_payload_valid = validate_payload(&req.payload, req.payload_type).is_ok();
                                                        let send_btn = ui.add_enabled(is_payload_valid, egui::Button::new("🚀"));
                                                        let send_btn = send_btn.on_hover_text(if is_payload_valid { tr("collections-send-tip") } else { tr("collections-invalid-payload-tip") });
                                                        if send_btn.clicked() {
                                                            send_clicked = true;
                                                            send_trigger = Some((format!("{}:{}", req.target_ip, req.target_port), req.payload_type, req.payload.clone()));
                                                        }
                                                    });
                                                });
                                            });

                                        // Make the whole row frame clickable to select the request without intercepting child button clicks
                                        let rect = frame_res.response.rect;
                                        let mut row_clicked = false;
                                        if ui.input(|i| i.pointer.primary_clicked()) {
                                            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                                                if rect.contains(pos) && !delete_clicked && !send_clicked {
                                                    row_clicked = true;
                                                }
                                            }
                                        }
                                        
                                        if label_clicked || row_clicked {
                                            select_request = Some(req.id.clone());
                                        }
                                    }
                                }
                            }
                            ui.add_space(4.0);
                        }
                    }
                });
            });
            
            // 2. Detailed Request Editor Form (Bottom Half)
            let mut req_clone = None;
            if let Some(selected_id) = &self.selected_request_id {
                for col in &self.collections {
                    if let Some(req) = col.requests.iter().find(|r| &r.id == selected_id) {
                        req_clone = Some(req.clone());
                        break;
                    }
                }
            }
            
            if let Some(mut req) = req_clone {
                ui.separator();
                ui.heading(tr("collections-edit-title"));
                ui.add_space(4.0);
                
                egui::ScrollArea::vertical().id_salt("collection_editor_scroll").show(ui, |ui| {
                    egui::Grid::new("collection_req_edit_grid")
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(tr("collections-edit-name"));
                            if ui.text_edit_singleline(&mut req.name).changed() {
                                needs_save = true;
                            }
                            ui.end_row();
                            ui.label(tr("collections-edit-target-ip"));
                            ui.horizontal(|ui| {
                                let mut ip_chosen = None;
                                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                let edit_ip = ui.add(egui::TextEdit::singleline(&mut req.target_ip).desired_width(120.0));
                                if edit_ip.changed() {
                                    needs_save = true;
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
                                    req.target_ip = ip;
                                    needs_save = true;
                                }
                            });
                            ui.end_row();

                            ui.label(tr("collections-edit-target-port"));
                            ui.horizontal(|ui| {
                                let mut port_chosen = None;
                                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                                let edit_port = ui.add(egui::TextEdit::singleline(&mut req.target_port).desired_width(60.0));
                                if edit_port.changed() {
                                    needs_save = true;
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
                                    req.target_port = port;
                                    needs_save = true;
                                }
                            });
                            ui.end_row();
                            ui.label(tr("collections-edit-payload"));
                              ui.horizontal(|ui| {
                                  let r1 = ui.radio_value(&mut req.payload_type, PayloadType::Text, "Text");
                                  ui.add_space(10.0);
                                  let r2 = ui.radio_value(&mut req.payload_type, PayloadType::Hex, "Hex");
                                  if r1.changed() || r2.changed() {
                                      needs_save = true;
                                  }
                              });
                             ui.end_row();
                         });
                     
                     ui.add_space(6.0);
                     let current_target = format!("{}:{}", req.target_ip, req.target_port);
                     if let Some((payload, format, target)) = self.show_echonet_lite_helper(ui, &current_target) {
                         req.payload = payload;
                         req.payload_type = format;
                         if let Some(idx) = target.rfind(':') {
                             let (ip, port) = target.split_at(idx);
                             req.target_ip = ip.to_string();
                             req.target_port = port[1..].to_string();
                         } else {
                             req.target_ip = target;
                             req.target_port = "3610".to_string();
                         }
                         needs_save = true;
                     }
                     
                     ui.add_space(6.0);
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut req.payload)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(4)
                            .desired_width(ui.available_width())
                    );
                    if response.changed() {
                        needs_save = true;
                    }
   
                    let payload_validation = validate_payload(&req.payload, req.payload_type);
                    if let Err(ref err_msg) = payload_validation {
                        ui.add_space(4.0);
                        let mut args = std::collections::HashMap::new();
                        args.insert(std::borrow::Cow::Borrowed("msg"), err_msg.clone().into());
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 100, 100),
                            tr_args("collections-edit-invalid-payload", &args)
                        );
                    }
                    
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(tr("collections-edit-load")).clicked() {
                            load_to_composer = Some((format!("{}:{}", req.target_ip, req.target_port), req.payload_type, req.payload.clone()));
                        }
                        let is_payload_valid = payload_validation.is_ok();
                        let send_btn = ui.add_enabled(
                            is_payload_valid,
                            egui::Button::new(tr("collections-edit-send"))
                        );
                        if send_btn.clicked() {
                            send_trigger = Some((format!("{}:{}", req.target_ip, req.target_port), req.payload_type, req.payload.clone()));
                        }
                    });
                });
                
                // Save the modified request back to the collection mutably (outside borrow loops)
                let mut found = false;
                for col in &mut self.collections {
                    if let Some(r) = col.requests.iter_mut().find(|r| r.id == req.id) {
                        if r.name != req.name || r.target_ip != req.target_ip || r.target_port != req.target_port || r.payload_type != req.payload_type || r.payload != req.payload {
                            *r = req;
                            found = true;
                        }
                        break;
                    }
                }
                if found {
                    needs_save = true;
                }
            }
        });

        // --- Apply deferred modifications (outside borrowing loops to satisfy borrow checker) ---
        if create_collection {
            let col_name = {
                let mut args = std::collections::HashMap::new();
                args.insert(std::borrow::Cow::Borrowed("idx"), (self.collections.len() + 1).into());
                tr_args("collections-created-name", &args)
            };
            self.collections.push(Collection {
                id: generate_id(),
                name: col_name,
                requests: Vec::new(),
                is_expanded: true,
            });
            needs_save = true;
        }
        
        if let Some(col_id) = toggle_expand {
            if let Some(col) = self.collections.iter_mut().find(|c| c.id == col_id) {
                col.is_expanded = !col.is_expanded;
                needs_save = true;
            }
        }
        
        if let Some(col_id) = delete_collection {
            self.collections.retain(|c| c.id != col_id);
            needs_save = true;
        }
 
        if let Some(col_id) = export_collection {
            if let Some(col) = self.collections.iter().find(|c| c.id == col_id) {
                let yaml_col = YamlCollection {
                    name: col.name.clone(),
                    requests: col.requests.iter().map(|r| YamlRequest {
                        name: r.name.clone(),
                        target_ip: r.target_ip.clone(),
                        target_port: r.target_port.clone(),
                        payload_type: r.payload_type,
                        payload: r.payload.clone(),
                    }).collect(),
                };
 
                match serde_yaml::to_string(&yaml_col) {
                    Ok(yaml_str) => {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}.yaml", col.name))
                            .add_filter("YAML File", &["yaml", "yml"])
                            .save_file()
                        {
                            if let Err(e) = std::fs::write(&path, yaml_str) {
                                let mut args = std::collections::HashMap::new();
                                args.insert(std::borrow::Cow::Borrowed("msg"), e.to_string().into());
                                self.add_system_error(tr_args("collections-export-fail", &args));
                            } else {
                                let mut args = std::collections::HashMap::new();
                                args.insert(std::borrow::Cow::Borrowed("path"), path.display().to_string().into());
                                self.add_system_info(tr_args("collections-export-success", &args));
                            }
                        }
                    }
                    Err(e) => {
                        let mut args = std::collections::HashMap::new();
                        args.insert(std::borrow::Cow::Borrowed("msg"), e.to_string().into());
                        self.add_system_error(tr_args("collections-export-fail", &args));
                    }
                }
            }
        }
        
        if let Some(col_id) = add_request {
            if let Some(col) = self.collections.iter_mut().find(|c| c.id == col_id) {
                let req_id = generate_id();
                let req_name = {
                    let mut args = std::collections::HashMap::new();
                    args.insert(std::borrow::Cow::Borrowed("idx"), (col.requests.len() + 1).into());
                    tr_args("collections-req-created-name", &args)
                };
                col.requests.push(PacketDefinition {
                    id: req_id.clone(),
                    name: req_name,
                    target_ip: "127.0.0.1".to_string(),
                    target_port: "9000".to_string(),
                    payload_type: PayloadType::Text,
                    payload: "New Request Payload".to_string(),
                });
                col.is_expanded = true;
                self.selected_request_id = Some(req_id);
                needs_save = true;
            }
        }
        
        if let Some((col_id, req_id)) = delete_request {
            if let Some(col) = self.collections.iter_mut().find(|c| c.id == col_id) {
                col.requests.retain(|r| r.id != req_id);
                if self.selected_request_id.as_ref() == Some(&req_id) {
                    self.selected_request_id = None;
                }
                needs_save = true;
            }
        }
        
        if let Some(req_id) = select_request {
            self.selected_request_id = Some(req_id);
        }
        
        if needs_save {
            self.save_config();
        }
        
        if let Some((target, p_type, p_data)) = load_to_composer {
            if let Some(idx) = target.rfind(':') {
                let (ip, port) = target.split_at(idx);
                self.composer_ip = ip.to_string();
                self.composer_port = port[1..].to_string();
            } else {
                self.composer_ip = target;
                self.composer_port = "9000".to_string();
            }
            self.composer_payload_type = p_type;
            self.composer_payload = p_data;
            self.save_config();
        }
        
        if let Some((target, p_type, p_data)) = send_trigger {
            if let Some(idx) = target.rfind(':') {
                let (ip, port) = target.split_at(idx);
                self.add_to_composer_history(ip.to_string(), port[1..].to_string());
            }
            self.send_packet(&target, p_type, &p_data);
        }
 
        if import_collection {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("YAML File", &["yaml", "yml"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
 
 
                        match serde_yaml::from_str::<YamlCollection>(&content) {
                            Ok(parsed) => {
                                let new_col = Collection {
                                    id: generate_id(),
                                    name: parsed.name,
                                    is_expanded: true,
                                    requests: parsed.requests.into_iter().map(|r| PacketDefinition {
                                        id: generate_id(),
                                        name: r.name,
                                        target_ip: r.target_ip,
                                        target_port: r.target_port,
                                        payload_type: r.payload_type,
                                        payload: r.payload,
                                    }).collect(),
                                };
                                self.collections.push(new_col);
                                self.save_config(); // Save config with new collection
                                let mut args = std::collections::HashMap::new();
                                args.insert(std::borrow::Cow::Borrowed("path"), path.display().to_string().into());
                                self.add_system_info(tr_args("collections-import-success", &args));
                            }
                            Err(e) => {
                                let mut args = std::collections::HashMap::new();
                                args.insert(std::borrow::Cow::Borrowed("msg"), e.to_string().into());
                                self.add_system_error(tr_args("collections-import-fail-parse", &args));
                            }
                        }
                    }
                    Err(e) => {
                        let mut args = std::collections::HashMap::new();
                        args.insert(std::borrow::Cow::Borrowed("msg"), e.to_string().into());
                        self.add_system_error(tr_args("collections-import-fail-read", &args));
                    }
                }
            }
        }
    }
}


