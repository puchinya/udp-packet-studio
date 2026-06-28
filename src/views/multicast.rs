use eframe::egui;
use crate::UdpStudioState;

impl UdpStudioState {
    pub fn show_multicast(&mut self, ui: &mut egui::Ui) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let tr = |key: &str| {
            egui_i18n::set_language(&lang_id);
            egui_i18n::tr!(key)
        };
        let _tr_args = |key: &str, args: &std::collections::HashMap<std::borrow::Cow<'static, str>, egui_i18n::fluent_bundle::FluentValue<'_>>| {
            egui_i18n::set_language(&lang_id);
            let mut fluent_args = egui_i18n::fluent::FluentArgs::new();
            for (k, v) in args {
                fluent_args.set(k.as_ref(), v.clone());
            }
            egui_i18n::translate_fluent(key, &fluent_args)
        };

        let mut join_trigger = None;
        let mut leave_trigger = None;

        // Fetch currently selected socket's state for Multicast
        let socket_idx = self.sockets.iter().position(|s| s.id == self.multicast_selected_socket_id);
        let is_listening = socket_idx.map(|idx| self.sockets[idx].is_listening).unwrap_or(false);
        let multicast_groups = socket_idx.map(|idx| self.sockets[idx].multicast_groups.clone()).unwrap_or_default();

        ui.vertical(|ui| {
            // Socket Selection Dropdown
            ui.horizontal(|ui| {
                ui.label(tr("sockets-lbl-name"));
                let current_socket_name = self.sockets.iter()
                    .find(|s| s.id == self.multicast_selected_socket_id)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Main Socket".to_string());
                
                egui::ComboBox::from_id_salt("multicast_socket_select")
                    .selected_text(&current_socket_name)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for socket in &self.sockets {
                            ui.selectable_value(&mut self.multicast_selected_socket_id, socket.id.clone(), &socket.name);
                        }
                    });
            });
            ui.add_space(8.0);

            // Listener Status Warning
            if !is_listening {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(45, 20, 20))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), tr("mc-status-offline"));
                            ui.add(egui::Label::new(tr("mc-status-offline-tip")).wrap());
                        });
                    });
                ui.add_space(10.0);
            }

            // 1. Join Multicast Group Form
            egui::Frame::NONE
                .fill(ui.visuals().widgets.inactive.bg_fill)
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.strong(tr("mc-join-title"));
                        ui.add_space(8.0);

                        egui::Grid::new("multicast_join_grid")
                            .num_columns(2)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(tr("mc-label-multicast-ip"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.multicast_input_addr)
                                        .desired_width(120.0)
                                        .hint_text("e.g. 224.0.23.0")
                                );
                                ui.end_row();

                                ui.label(tr("mc-label-interface-ip"));
                                
                                let mut ifaces = vec!["0.0.0.0".to_string(), "127.0.0.1".to_string()];
                                if let Ok(addrs) = get_if_addrs::get_if_addrs() {
                                    for iface in addrs {
                                        if iface.ip().is_ipv4() {
                                            let ip_str = iface.ip().to_string();
                                            if !ifaces.contains(&ip_str) {
                                                ifaces.push(ip_str);
                                            }
                                        }
                                    }
                                }
                                
                                egui::ComboBox::from_id_salt("multicast_interface_combo")
                                    .selected_text(&self.multicast_input_interface)
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        for ip in ifaces {
                                            ui.selectable_value(&mut self.multicast_input_interface, ip.clone(), ip.clone());
                                        }
                                    });
                                ui.end_row();
                            });

                        ui.add_space(10.0);

                        // Quick Presets (dropdown menu)
                        ui.horizontal(|ui| {
                            let presets: &[(&str, &str)] = &[
                                ("ECHONET Lite", "224.0.23.0"),
                                ("SSDP",         "239.255.255.250"),
                                ("mDNS",         "224.0.0.251"),
                                ("All-Nodes",    "224.0.0.1"),
                            ];

                            let mut selected: Option<&str> = None;

                            ui.menu_button(format!("{} ▾", tr("mc-label-presets")), |ui| {
                                ui.set_min_width(180.0);
                                ui.add_space(2.0);
                                for (name, ip) in presets {
                                    // 行全体を1つのヒットエリアとして確保
                                    let row_height = 26.0;
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), row_height),
                                        egui::Sense::click(),
                                    );

                                    // ホバー／クリック時の背景ハイライト（行全体）
                                    if response.hovered() || response.is_pointer_button_down_on() {
                                        ui.painter().rect_filled(
                                            rect,
                                            egui::CornerRadius::same(4),
                                            ui.visuals().widgets.hovered.bg_fill,
                                        );
                                    }

                                    // プロトコル名（左寄せ）
                                    ui.painter().text(
                                        rect.left_center() + egui::vec2(8.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        *name,
                                        egui::FontId::proportional(13.0),
                                        if response.hovered() {
                                            ui.visuals().widgets.hovered.text_color()
                                        } else {
                                            ui.visuals().text_color()
                                        },
                                    );

                                    // マルチキャストアドレス（右寄せ）
                                    ui.painter().text(
                                        rect.right_center() - egui::vec2(8.0, 0.0),
                                        egui::Align2::RIGHT_CENTER,
                                        *ip,
                                        egui::FontId::monospace(12.0),
                                        if response.hovered() {
                                            ui.visuals().widgets.hovered.text_color()
                                        } else {
                                            ui.visuals().weak_text_color()
                                        },
                                    );

                                    if response.clicked() {
                                        selected = Some(ip);
                                        ui.close();
                                    }
                                }
                                ui.add_space(2.0);
                            });

                            if let Some(ip) = selected {
                                self.multicast_input_addr = ip.to_string();
                            }
                        });

                        ui.add_space(12.0);

                        let join_btn = ui.add_enabled(
                            is_listening,
                            egui::Button::new(tr("mc-btn-join")).min_size(egui::vec2(120.0, 26.0))
                        );
                        if join_btn.clicked() {
                            join_trigger = Some((
                                self.multicast_input_addr.trim().to_string(),
                                self.multicast_input_interface.trim().to_string()
                            ));
                        }
                    });
                });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // 2. Joined Groups List
            ui.strong(tr("mc-title-joined-list"));
            ui.add_space(6.0);

            if multicast_groups.is_empty() {
                ui.add(egui::Label::new(
                    egui::RichText::new(tr("mc-no-memberships"))
                        .color(egui::Color32::from_rgb(120, 130, 140))
                        .italics()
                ).wrap());
            } else {
                egui::Grid::new("multicast_joined_grid")
                    .num_columns(3)
                    .spacing([15.0, 8.0])
                    .show(ui, |ui| {
                        // Table Header
                        ui.colored_label(egui::Color32::from_rgb(180, 190, 200), egui::RichText::new(tr("mc-hdr-multicast-addr")).strong());
                        ui.colored_label(egui::Color32::from_rgb(180, 190, 200), egui::RichText::new(tr("mc-hdr-interface-addr")).strong());
                        ui.label("");
                        ui.end_row();

                        for group in &multicast_groups {
                            ui.label(&group.multi_addr);
                            ui.label(&group.interface_addr);
                            
                            if ui.button("🗑").on_hover_text(tr("mc-btn-leave")).clicked() {
                                leave_trigger = Some((group.multi_addr.clone(), group.interface_addr.clone()));
                            }
                            ui.end_row();
                        }
                    });
            }
        });

        // Apply mutations outside borrow scopes
        if let Some((m_addr, i_addr)) = join_trigger {
            if m_addr.is_empty() || i_addr.is_empty() {
                self.add_system_error(tr("mc-err-empty-fields"));
            } else {
                self.udp_worker.send(crate::udp_worker::UdpCommand::JoinMulticast {
                    id: self.multicast_selected_socket_id.clone(),
                    multi_addr: m_addr,
                    interface_addr: i_addr,
                });
            }
        }

        if let Some((m_addr, i_addr)) = leave_trigger {
            self.udp_worker.send(crate::udp_worker::UdpCommand::LeaveMulticast {
                id: self.multicast_selected_socket_id.clone(),
                multi_addr: m_addr,
                interface_addr: i_addr,
            });
        }
    }
}
