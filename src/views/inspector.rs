use eframe::egui;
use crate::UdpStudioState;
use crate::types::{LogDirection, InspectorProtocol, to_hex_dump};
use crate::mra::MraDatabase;

struct EchonetProperty {
    epc: u8,
    pdc: u8,
    edt: Vec<u8>,
}

impl UdpStudioState {
    pub fn show_inspector(&mut self, ui: &mut egui::Ui) {
        crate::locales::init_translations();
        let lang_id = self.language_id();
        let use_ja = lang_id.starts_with("ja");
        let mut record_proto = None;
        let mra_db = self.mra_db.clone();
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

        ui.vertical(|ui| {
            if self.selected_log_indices.len() > 1 {
                // Do not display inspector if multiple items are selected (Wireshark-like behavior)
            } else if let Some(idx) = self.selected_log_idx {
                if idx < self.logs.len() {
                    let entry = &self.logs[idx];
                    
                    // Selected log metadata header
                    ui.horizontal(|ui| {
                        let mut args_ts = std::collections::HashMap::new();
                        args_ts.insert(std::borrow::Cow::Borrowed("ts"), entry.timestamp.format("%Y-%m-%d %H:%M:%S.%3f").to_string().into());
                        ui.label(tr_args("ins-label-timestamp", &args_ts));
                        ui.separator();
                        let dir_label = match entry.direction {
                            LogDirection::Sent => tr("ins-label-sent-to"),
                            LogDirection::Received => tr("ins-label-received-from"),
                            LogDirection::SystemInfo => tr("ins-label-event-target"),
                            LogDirection::SystemError => tr("ins-label-error-target"),
                        };
                        ui.label(format!("{} {}", dir_label, entry.address));
                        ui.separator();
                        let mut args_size = std::collections::HashMap::new();
                        args_size.insert(std::borrow::Cow::Borrowed("len"), entry.data.len().into());
                        ui.label(tr_args("ins-label-size", &args_size));
                    });
                    
                    ui.add_space(8.0);
                    
                    // Protocol Selector
                    ui.horizontal(|ui| {
                        ui.label(tr("ins-label-decode-as"));
                        ui.selectable_value(&mut self.inspector_protocol, InspectorProtocol::Raw, tr("ins-proto-raw"));
                        ui.selectable_value(&mut self.inspector_protocol, InspectorProtocol::TextAscii, tr("ins-proto-ascii"));
                        
                        let combo_label = match self.inspector_protocol {
                            InspectorProtocol::EchonetLite => tr("ins-proto-echonet"),
                            InspectorProtocol::Syslog => tr("ins-proto-syslog"),
                            InspectorProtocol::Snmp => tr("ins-proto-snmp"),
                            _ => tr("settings-tab-protocols"),
                        };

                        let is_custom = self.inspector_protocols_order.contains(&self.inspector_protocol);
                        let mut selected_proto = self.inspector_protocol;
                        
                        let original_visuals = if is_custom {
                            let orig = ui.visuals().clone();
                            let visuals = ui.visuals_mut();
                            visuals.widgets.inactive.bg_fill = visuals.selection.bg_fill;
                            visuals.widgets.inactive.weak_bg_fill = visuals.selection.bg_fill;
                            visuals.widgets.inactive.fg_stroke = visuals.selection.stroke;
                            visuals.widgets.hovered.bg_fill = visuals.selection.bg_fill;
                            visuals.widgets.hovered.weak_bg_fill = visuals.selection.bg_fill;
                            visuals.widgets.hovered.fg_stroke = visuals.selection.stroke;
                            Some(orig)
                        } else {
                            None
                        };

                        let combo_res = egui::ComboBox::from_id_salt("inspector_custom_protocol_combo")
                            .selected_text(combo_label)
                            .show_ui(ui, |ui| {
                                if let Some(ref orig) = original_visuals {
                                    *ui.visuals_mut() = orig.clone();
                                }
                                let mut changed = false;
                                for &proto in &self.inspector_protocols_order {
                                    let label = match proto {
                                        InspectorProtocol::EchonetLite => tr("ins-proto-echonet"),
                                        InspectorProtocol::Syslog => tr("ins-proto-syslog"),
                                        InspectorProtocol::Snmp => tr("ins-proto-snmp"),
                                        _ => "".to_string(),
                                    };
                                    if ui.selectable_value(&mut selected_proto, proto, label).clicked() {
                                        changed = true;
                                    }
                                }
                                changed
                            });
                        
                        if let Some(orig) = original_visuals {
                            *ui.visuals_mut() = orig;
                        }
                        
                        if combo_res.inner.unwrap_or(false) {
                            self.inspector_protocol = selected_proto;
                            record_proto = Some(selected_proto);
                        }
                    });
                    
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Decode details
                    egui::ScrollArea::vertical()
                        .id_salt("inspector_content_scroll")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            match self.inspector_protocol {
                                InspectorProtocol::Raw => {
                                    let mut dump = to_hex_dump(&entry.data);
                                    let lines = dump.lines().count();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut dump)
                                            .font(egui::TextStyle::Monospace)
                                            .code_editor()
                                            .desired_width(ui.available_width())
                                            .desired_rows(lines)
                                            .interactive(false)
                                    );
                                }
                                InspectorProtocol::TextAscii => {
                                    let mut ascii_rep = to_ascii_inspector(&entry.data);
                                    let lines = ascii_rep.lines().count();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut ascii_rep)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(ui.available_width())
                                            .desired_rows(lines)
                                            .interactive(false)
                                    );
                                }
                                InspectorProtocol::EchonetLite => {
                                    if entry.data.len() < 12 {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 100, 100),
                                            tr("ins-el-err-too-short")
                                        );
                                    } else {
                                        let ehd1 = entry.data[0];
                                        let ehd2 = entry.data[1];
                                        
                                        if ehd1 != 0x10 {
                                            let mut args = std::collections::HashMap::new();
                                            args.insert(std::borrow::Cow::Borrowed("val"), format!("{:02X}", ehd1).into());
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 180, 100),
                                                tr_args("ins-el-warn-ehd1", &args)
                                            );
                                            ui.add_space(4.0);
                                        }
                                        
                                        let tid_h = entry.data[2];
                                        let tid_l = entry.data[3];
                                        let seoj = &entry.data[4..7];
                                        let deoj = &entry.data[7..10];
                                        let esv = entry.data[10];
                                        let opc = entry.data[11];
                                        
                                        // Frame structure grid
                                        egui::Grid::new("el_inspector_grid")
                                            .num_columns(2)
                                            .spacing([12.0, 6.0])
                                            .show(ui, |ui| {
                                                ui.label(tr("ins-el-label-ehd1"));
                                                ui.monospace(format!("0x{:02X} (ECHONET Lite)", ehd1));
                                                ui.end_row();
                                                
                                                ui.label(tr("ins-el-label-ehd2"));
                                                let fmt_str = if ehd2 == 0x81 { "1" } else { "2" };
                                                let mut args_fmt = std::collections::HashMap::new();
                                                args_fmt.insert(std::borrow::Cow::Borrowed("fmt"), fmt_str.into());
                                                ui.monospace(format!("0x{:02X} ({})", ehd2, tr_args("ins-el-format", &args_fmt)));
                                                ui.end_row();
                                                
                                                ui.label(tr("ins-el-label-tid"));
                                                ui.monospace(format!("0x{:02X}{:02X}", tid_h, tid_l));
                                                ui.end_row();
                                                
                                                ui.label(tr("ins-el-label-seoj"));
                                                ui.label(translate_object(seoj, &mra_db, use_ja));
                                                ui.end_row();
                                                
                                                ui.label(tr("ins-el-label-deoj"));
                                                ui.label(translate_object(deoj, &mra_db, use_ja));
                                                ui.end_row();
                                                
                                                ui.label(tr("ins-el-label-esv"));
                                                ui.label(translate_esv(esv));
                                                ui.end_row();
                                                
                                                ui.label(tr("ins-el-label-opc"));
                                                ui.monospace(format!("{}", opc));
                                                ui.end_row();
                                            });
                                        
                                        ui.add_space(10.0);
                                        ui.strong(tr("ins-el-title-props"));
                                        ui.add_space(4.0);
                                        
                                        // Parse properties
                                        let mut properties = Vec::new();
                                        let mut curr_offset = 12;
                                        let mut is_malformed = false;
                                        
                                        for _ in 0..opc {
                                            if curr_offset + 2 > entry.data.len() {
                                                is_malformed = true;
                                                break;
                                            }
                                            let epc = entry.data[curr_offset];
                                            let pdc = entry.data[curr_offset + 1];
                                            
                                            curr_offset += 2;
                                            
                                            if curr_offset + (pdc as usize) > entry.data.len() {
                                                is_malformed = true;
                                                break;
                                            }
                                            
                                            let edt = entry.data[curr_offset..curr_offset + (pdc as usize)].to_vec();
                                            curr_offset += pdc as usize;
                                            
                                            properties.push(EchonetProperty { epc, pdc, edt });
                                        }
                                        
                                        // Render properties directly
                                        for (prop_idx, prop) in properties.iter().enumerate() {
                                            egui::Frame::NONE
                                                .fill(ui.visuals().widgets.inactive.bg_fill)
                                                .corner_radius(egui::CornerRadius::same(4))
                                                .inner_margin(egui::Margin::symmetric(10, 8))
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.strong(format!("#{}:", prop_idx + 1));
                                                            ui.label("EPC:");
                                                            ui.monospace(format!("0x{:02X}", prop.epc));
                                                            ui.label(translate_epc(prop.epc, deoj, &mra_db, use_ja));
                                                        });
                                                        ui.add_space(2.0);
                                                        ui.horizontal(|ui| {
                                                            ui.label("PDC:");
                                                            ui.monospace(format!("{}", prop.pdc));
                                                            ui.separator();
                                                            ui.label("EDT:");
                                                            ui.monospace(translate_edt(prop.epc, &prop.edt));
                                                        });
                                                    });
                                                });
                                            ui.add_space(6.0);
                                        }
                                        
                                        if is_malformed {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 100, 100),
                                                tr("ins-el-err-malformed")
                                            );
                                        }
                                    }
                                }
                                InspectorProtocol::Syslog => {
                                    if let Some(syslog) = crate::syslog::parse_syslog(&entry.data) {
                                        egui::Grid::new("syslog_inspector_grid")
                                            .num_columns(2)
                                            .spacing([12.0, 6.0])
                                            .show(ui, |ui| {
                                                ui.label(tr("ins-syslog-rfc"));
                                                ui.monospace(&syslog.rfc);
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-priority"));
                                                ui.monospace(format!("{}", syslog.priority));
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-facility"));
                                                ui.label(format!("{} ({})", syslog.facility, crate::syslog::facility_name(syslog.facility)));
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-severity"));
                                                ui.label(format!("{} ({})", syslog.severity, crate::syslog::severity_name(syslog.severity)));
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-timestamp"));
                                                ui.monospace(&syslog.timestamp);
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-hostname"));
                                                ui.monospace(&syslog.hostname);
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-appname"));
                                                ui.monospace(&syslog.app_name);
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-procid"));
                                                ui.monospace(&syslog.proc_id);
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-msgid"));
                                                ui.monospace(&syslog.msg_id);
                                                ui.end_row();

                                                ui.label(tr("ins-syslog-message"));
                                                ui.label(&syslog.message);
                                                ui.end_row();
                                            });
                                    } else {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 100, 100),
                                            "Failed to parse packet as Syslog (not starting with '<PRI>')"
                                        );
                                    }
                                }
                                InspectorProtocol::Snmp => {
                                    match crate::snmp::parse_snmp(&entry.data) {
                                        Ok(snmp) => {
                                            egui::Grid::new("snmp_inspector_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 6.0])
                                                .show(ui, |ui| {
                                                    ui.label(tr("ins-snmp-version"));
                                                    let ver_name = if snmp.version == 0 { "v1" } else { "v2c" };
                                                    ui.monospace(format!("{} (value: {})", ver_name, snmp.version));
                                                    ui.end_row();

                                                    ui.label(tr("ins-snmp-community"));
                                                    ui.monospace(&snmp.community);
                                                    ui.end_row();

                                                    ui.label(tr("ins-snmp-pdutype"));
                                                    ui.monospace(format!("{} (0x{:02X})", crate::snmp::pdu_type_name(snmp.pdu_type), snmp.pdu_type));
                                                    ui.end_row();

                                                    ui.label(tr("ins-snmp-reqid"));
                                                    ui.monospace(format!("{}", snmp.request_id));
                                                    ui.end_row();

                                                    ui.label(tr("ins-snmp-errstatus"));
                                                    ui.monospace(format!("{}", snmp.error_status));
                                                    ui.end_row();

                                                    ui.label(tr("ins-snmp-errindex"));
                                                    ui.monospace(format!("{}", snmp.error_index));
                                                    ui.end_row();
                                                });

                                            ui.add_space(10.0);
                                            ui.strong(tr("ins-snmp-varbinds"));
                                            ui.add_space(4.0);

                                            for (i, (oid, val)) in snmp.varbinds.iter().enumerate() {
                                                egui::Frame::NONE
                                                    .fill(ui.visuals().widgets.inactive.bg_fill)
                                                    .corner_radius(egui::CornerRadius::same(4))
                                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                                    .show(ui, |ui| {
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.strong(format!("#{}:", i + 1));
                                                                ui.label("OID:");
                                                                ui.monospace(oid);
                                                            });
                                                            ui.add_space(2.0);
                                                            ui.horizontal(|ui| {
                                                                ui.label("Value:");
                                                                ui.monospace(val.to_string_repr());
                                                            });
                                                        });
                                                    });
                                                ui.add_space(6.0);
                                            }
                                        }
                                        Err(err_msg) => {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 100, 100),
                                                format!("Failed to parse SNMP packet: {}", err_msg)
                                            );
                                        }
                                    }
                                }
                            }
                        });
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(tr("ins-select-log-item"));
                });
            }
        });

        if let Some(proto) = record_proto {
            self.record_inspector_protocol_usage(proto);
        }
    }
}

// ASCII Formatter: highlights control characters
fn to_ascii_inspector(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &b in bytes {
        if b.is_ascii_graphic() || b == b' ' {
            result.push(b as char);
        } else {
            let label = match b {
                0 => "[NUL]",
                1 => "[SOH]",
                2 => "[STX]",
                3 => "[ETX]",
                4 => "[EOT]",
                9 => "[TAB]",
                10 => "[LF]",
                13 => "[CR]",
                _ => &format!("[0x{:02X}]", b),
            };
            result.push_str(label);
        }
    }
    result
}

// ECHONET Lite Translators
fn translate_object(obj_bytes: &[u8], mra_db: &MraDatabase, use_ja: bool) -> String {
    if obj_bytes.len() != 3 {
        return egui_i18n::tr!("ins-el-obj-unknown");
    }
    let group = obj_bytes[0];
    let class = obj_bytes[1];
    let instance = obj_bytes[2];
    
    let name = if let Some(info) = mra_db.classes.get(&(group, class)) {
        if use_ja { info.name_ja.clone() } else { info.name_en.clone() }
    } else {
        egui_i18n::tr!("ins-el-obj-custom")
    };
    format!("{} (0x{:02X}{:02X} inst:{:02X})", name, group, class, instance)
}

fn translate_esv(esv: u8) -> String {
    match esv {
        0x60 => egui_i18n::tr!("ins-el-esv-seti"),
        0x61 => egui_i18n::tr!("ins-el-esv-setc"),
        0x62 => egui_i18n::tr!("ins-el-esv-get"),
        0x63 => egui_i18n::tr!("ins-el-esv-inf-req"),
        0x71 => egui_i18n::tr!("ins-el-esv-set-res"),
        0x72 => egui_i18n::tr!("ins-el-esv-get-res"),
        0x73 => egui_i18n::tr!("ins-el-esv-inf"),
        0x74 => egui_i18n::tr!("ins-el-esv-infc"),
        0x50 => egui_i18n::tr!("ins-el-esv-seti-sna"),
        0x51 => egui_i18n::tr!("ins-el-esv-setc-sna"),
        0x52 => egui_i18n::tr!("ins-el-esv-get-sna"),
        0x53 => egui_i18n::tr!("ins-el-esv-inf-sna"),
        _ => egui_i18n::tr!("ins-el-esv-unknown", { esv: format!("{:02X}", esv) }),
    }
}

fn translate_epc(epc: u8, deoj: &[u8], mra_db: &MraDatabase, use_ja: bool) -> String {
    // Try to look up using the DEOJ's class from MRA
    if deoj.len() >= 2 {
        let group = deoj[0];
        let class = deoj[1];
        if let Some(class_info) = mra_db.classes.get(&(group, class)) {
            if let Some(prop) = class_info.properties.get(&epc) {
                return if use_ja { prop.name_ja.clone() } else { prop.name_en.clone() };
            }
        }
    }
    // Fallback to hardcoded super-class properties
    match epc {
        0x80 => egui_i18n::tr!("ins-el-epc-status"),
        0x81 => egui_i18n::tr!("ins-el-epc-location"),
        0x82 => egui_i18n::tr!("ins-el-epc-version"),
        0x83 => egui_i18n::tr!("ins-el-epc-id"),
        0x88 => egui_i18n::tr!("ins-el-epc-fault"),
        0x8A => egui_i18n::tr!("ins-el-epc-manufacturer"),
        0xB0 => egui_i18n::tr!("ins-el-epc-mode"),
        0xC0 => egui_i18n::tr!("ins-el-epc-temp"),
        0xC1 => egui_i18n::tr!("ins-el-epc-temp-cool"),
        0xD6 => egui_i18n::tr!("ins-el-epc-node-instances"),
        0xD7 => egui_i18n::tr!("ins-el-epc-node-classes"),
        _ => "".to_string(),
    }
}

fn translate_edt(epc: u8, edt: &[u8]) -> String {
    if edt.is_empty() {
        return egui_i18n::tr!("ins-el-edt-empty");
    }
    match epc {
        0x80 => {
            if edt[0] == 0x30 {
                egui_i18n::tr!("ins-el-edt-on")
            } else if edt[0] == 0x31 {
                egui_i18n::tr!("ins-el-edt-off")
            } else {
                egui_i18n::tr!("ins-el-edt-unknown", { val: format!("{:02X}", edt[0]) })
            }
        }
        0x88 => {
            if edt[0] == 0x41 {
                egui_i18n::tr!("ins-el-edt-fault")
            } else if edt[0] == 0x42 {
                egui_i18n::tr!("ins-el-edt-normal")
            } else {
                egui_i18n::tr!("ins-el-edt-unknown", { val: format!("{:02X}", edt[0]) })
            }
        }
        0xB0 => {
            match edt[0] {
                0x41 => egui_i18n::tr!("ins-el-edt-auto"),
                0x42 => egui_i18n::tr!("ins-el-edt-cool"),
                0x43 => egui_i18n::tr!("ins-el-edt-heat"),
                0x44 => egui_i18n::tr!("ins-el-edt-dehumid"),
                0x45 => egui_i18n::tr!("ins-el-edt-circulator"),
                _ => egui_i18n::tr!("ins-el-edt-unknown", { val: format!("{:02X}", edt[0]) }),
            }
        }
        0xC0 | 0xC1 => {
            egui_i18n::tr!("ins-el-edt-temp", { temp: edt[0].to_string(), val: format!("{:02X}", edt[0]) })
        }
        _ => {
            edt.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ")
        }
    }
}
