use eframe::egui;
use crate::UdpStudioState;
use crate::types::{Collection, PacketDefinition, PayloadType, generate_id};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct YamlRequest {
    pub name: String,
    pub target: String,
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
                if ui.button("➕ New").on_hover_text("Create a new empty collection").clicked() {
                    create_collection = true;
                }
                if ui.button("📥 Import").on_hover_text("Import a collection from a YAML file").clicked() {
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
                        ui.colored_label(egui::Color32::from_rgb(120, 130, 140), egui::RichText::new("No collections. Click 'New Collection' to start!").italics());
                    } else {
                        for col_idx in 0..self.collections.len() {
                            let collection = &mut self.collections[col_idx];
                            
                            // Collection Header Row
                            ui.horizontal(|ui| {
                                // Toggle Chevron
                                let caret = if collection.is_expanded { "▼" } else { "▶" };
                                if ui.add(egui::Button::new(caret).frame(false)).clicked() {
                                    toggle_expand = Some(collection.id.clone());
                                }
                                
                                ui.label("📁");
                                
                                // Editable Collection Name
                                let name_edit = ui.add(
                                    egui::TextEdit::singleline(&mut collection.name)
                                        .frame(egui::Frame::NONE)
                                        .desired_width(ui.available_width() - 85.0)
                                        .hint_text("Unnamed Collection")
                                );
                                if name_edit.changed() {
                                    needs_save = true;
                                }
                                
                                // Hover actions on the right
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🗑").on_hover_text("Delete Collection").clicked() {
                                        delete_collection = Some(collection.id.clone());
                                    }
                                    if ui.button("📤").on_hover_text("Export Collection (YAML)").clicked() {
                                        export_collection = Some(collection.id.clone());
                                    }
                                    if ui.button("➕").on_hover_text("Add Request").clicked() {
                                        add_request = Some(collection.id.clone());
                                    }
                                });
                            });
                            
                            // If expanded, list requests inside
                            if collection.is_expanded {
                                if collection.requests.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.add_space(24.0);
                                        ui.colored_label(egui::Color32::from_rgb(100, 110, 120), egui::RichText::new("Empty collection").italics().size(11.0));
                                    });
                                } else {
                                    for req_idx in 0..collection.requests.len() {
                                        let req = &collection.requests[req_idx];
                                        let is_selected = Some(&req.id) == self.selected_request_id.as_ref();
                                        
                                        ui.horizontal(|ui| {
                                            ui.add_space(20.0); // Indentation
                                            
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
                                                "Unnamed Request".to_string()
                                            } else {
                                                req.name.clone()
                                            };
                                            
                                            // Truncate text if too long for the sidebar
                                            let display_name = if label_text.chars().count() > 18 {
                                                format!("{}...", label_text.chars().take(16).collect::<String>())
                                            } else {
                                                label_text
                                            };
                                            
                                            let btn = ui.add(
                                                egui::Button::selectable(is_selected, display_name)
                                                    .frame(false)
                                            );
                                            if btn.clicked() {
                                                select_request = Some(req.id.clone());
                                            }
                                            
                                            // Quick Actions on Right
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("🗑").on_hover_text("Delete Request").clicked() {
                                                    delete_request = Some((collection.id.clone(), req.id.clone()));
                                                }
                                                if ui.button("🚀").on_hover_text("Send Packets").clicked() {
                                                    send_trigger = Some((req.target.clone(), req.payload_type, req.payload.clone()));
                                                }
                                            });
                                        });
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
                ui.heading("📝 Edit Request");
                ui.add_space(4.0);
                
                egui::ScrollArea::vertical().id_salt("collection_editor_scroll").show(ui, |ui| {
                    egui::Grid::new("collection_req_edit_grid")
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            if ui.text_edit_singleline(&mut req.name).changed() {
                                needs_save = true;
                            }
                            ui.end_row();
                            
                            ui.label("Target:");
                            if ui.text_edit_singleline(&mut req.target).changed() {
                                needs_save = true;
                            }
                            ui.end_row();
                            
                            ui.label("Format:");
                            ui.horizontal(|ui| {
                                let r1 = ui.radio_value(&mut req.payload_type, PayloadType::Text, "Text");
                                let r2 = ui.radio_value(&mut req.payload_type, PayloadType::Hex, "Hex");
                                if r1.changed() || r2.changed() {
                                    needs_save = true;
                                }
                            });
                            ui.end_row();
                        });
                    
                    ui.add_space(6.0);
                    let current_target = req.target.clone();
                    if let Some((payload, format, target)) = self.show_echonet_lite_helper(ui, &current_target) {
                        req.payload = payload;
                        req.payload_type = format;
                        req.target = target;
                        needs_save = true;
                    }
                    
                    ui.add_space(6.0);
                    ui.label("Payload:");
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
                    
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("📂 Load to Composer").clicked() {
                            load_to_composer = Some((req.target.clone(), req.payload_type, req.payload.clone()));
                        }
                        if ui.button("🚀 Send").clicked() {
                            send_trigger = Some((req.target.clone(), req.payload_type, req.payload.clone()));
                        }
                    });
                });
                
                // Save the modified request back to the collection mutably (outside borrow loops)
                let mut found = false;
                for col in &mut self.collections {
                    if let Some(r) = col.requests.iter_mut().find(|r| r.id == req.id) {
                        if r.name != req.name || r.target != req.target || r.payload_type != req.payload_type || r.payload != req.payload {
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
            self.collections.push(Collection {
                id: generate_id(),
                name: format!("Collection {}", self.collections.len() + 1),
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
                        target: r.target.clone(),
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
                                self.add_system_error(format!("Failed to export collection: {}", e));
                            } else {
                                self.add_system_info(format!("Collection exported to {}", path.display()));
                            }
                        }
                    }
                    Err(e) => {
                        self.add_system_error(format!("YAML Serialization Error: {}", e));
                    }
                }
            }
        }
        
        if let Some(col_id) = add_request {
            if let Some(col) = self.collections.iter_mut().find(|c| c.id == col_id) {
                let req_id = generate_id();
                col.requests.push(PacketDefinition {
                    id: req_id.clone(),
                    name: format!("Request {}", col.requests.len() + 1),
                    target: "127.0.0.1:9000".to_string(),
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
            self.composer_target = target;
            self.composer_payload_type = p_type;
            self.composer_payload = p_data;
            self.save_config();
        }
        
        if let Some((target, p_type, p_data)) = send_trigger {
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
                                        target: r.target,
                                        payload_type: r.payload_type,
                                        payload: r.payload,
                                    }).collect(),
                                };
                                self.collections.push(new_col);
                                self.save_config(); // Save config with new collection
                                self.add_system_info(format!("Collection imported successfully from {}", path.display()));
                            }
                            Err(e) => {
                                self.add_system_error(format!("Failed to parse YAML collection: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.add_system_error(format!("Failed to read file: {}", e));
                    }
                }
            }
        }
    }
}


