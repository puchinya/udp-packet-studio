#[allow(deprecated)]
#[test]
fn test_resize_handles_interaction() {
    let ctx = egui::Context::default();
    
    // 1. Test Hover: Move pointer to NW corner [6.0, 6.0]
    let mut raw_input = egui::RawInput::default();
    raw_input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    raw_input.events.push(egui::Event::PointerMoved(egui::pos2(6.0, 6.0)));

    // Frame 1: Register pointer position
    let _ = ctx.run_ui(raw_input, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                udp_packet_studio::show_resize_handles(ui);
            });
    });

    // Frame 2: Check hover response
    let raw_input2 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    let full_output = ctx.run_ui(raw_input2, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                udp_packet_studio::show_resize_handles(ui);
            });
    });

    // The cursor icon should be set to ResizeNwSe
    assert_eq!(full_output.platform_output.cursor_icon, egui::CursorIcon::ResizeNwSe);

    // 2. Test Drag: Press and drag NW corner
    let ctx = egui::Context::default();
    
    // Frame 1: Move to [6.0, 6.0]
    let mut raw_input = egui::RawInput::default();
    raw_input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    raw_input.events.push(egui::Event::PointerMoved(egui::pos2(6.0, 6.0)));
    let _ = ctx.run_ui(raw_input, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                udp_packet_studio::show_resize_handles(ui);
            });
    });

    // Frame 2: Press down and drag
    let mut raw_input2 = egui::RawInput::default();
    raw_input2.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    raw_input2.events.push(egui::Event::PointerButton {
        pos: egui::pos2(6.0, 6.0),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw_input2.events.push(egui::Event::PointerMoved(egui::pos2(10.0, 10.0)));

    let full_output = ctx.run_ui(raw_input2, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                udp_packet_studio::show_resize_handles(ui);
            });
    });

    // Check if ViewportCommand::BeginResize(ResizeDirection::NorthWest) was sent
    let mut found_resize_command = false;
    for (_, viewport_output) in full_output.viewport_output.iter() {
        for command in &viewport_output.commands {
            if let egui::ViewportCommand::BeginResize(egui::viewport::ResizeDirection::NorthWest) = command {
                found_resize_command = true;
            }
        }
    }
    assert!(found_resize_command, "Expected ViewportCommand::BeginResize(NorthWest) to be sent");
}

fn find_text_center(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<egui::Pos2> {
    for clipped in shapes {
        if let egui::epaint::Shape::Text(text_shape) = &clipped.shape {
            if text_shape.galley.text().contains(text) {
                let rect = text_shape.galley.rect;
                let world_pos = text_shape.pos;
                return Some(world_pos + rect.center().to_vec2());
            }
        }
    }
    None
}

#[allow(deprecated)]
#[test]
fn test_gui_triggered_communication() {
    use std::net::UdpSocket;
    use std::sync::mpsc::channel;
    use udp_packet_studio::UdpStudioState;
    use udp_packet_studio::locales::LanguageSetting;
    use udp_packet_studio::types::{PayloadType, LoggerCommand, LogExportFormat, InspectorProtocol, AboutTab};
    use udp_packet_studio::udp_worker::{UdpWorker, UdpCommand, UdpEvent};

    let ctx = egui::Context::default();
    let (tx_event, rx_event) = channel();
    
    // Spawn the worker
    let worker = UdpWorker::spawn(tx_event, ctx.clone());
    
    // Bind to an ephemeral port
    worker.send(UdpCommand::Bind { id: "main".to_string(), addr: "127.0.0.1:0".to_string() });
    let bound_addr = match rx_event.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(UdpEvent::Bound { id: _, addr }) => addr,
        other => panic!("Expected Bound event, got {:?}", other),
    };

    // Bind a mock socket as the communication partner
    let partner = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind partner socket");
    let partner_addr = partner.local_addr().expect("Failed to get partner local addr");

    // Create a mock logger channel
    let (tx_logger, _rx_logger) = channel::<LoggerCommand>();

    // Construct the state with test values
    let mut state = UdpStudioState {
        collections: Vec::new(),
        selected_request_id: None,
        composer_selected_collection_idx: 0,
        composer_ip: partner_addr.ip().to_string(),
        composer_port: partner_addr.port().to_string(),
        composer_ip_history: Vec::new(),
        composer_port_history: Vec::new(),
        composer_payload_type: PayloadType::Text,
        composer_payload: "Hello GUI World!".to_string(),
        composer_name: "Test Name".to_string(),
        logs: Vec::new(),
        selected_log_idx: None,
        filter_text: String::new(),
        filter_input: String::new(),
        filter_history: Vec::new(),
        auto_scroll: true,
        log_export_format: LogExportFormat::Csv,
        filtered_indices: Vec::new(),
        sockets: vec![udp_packet_studio::types::ActiveSocketState {
            id: "main".to_string(),
            name: "Main Socket".to_string(),
            ip: "127.0.0.1".to_string(),
            port: bound_addr.port().to_string(),
            is_listening: true,
            bound_addr: Some(bound_addr.to_string()),
            error: None,
            bind_time: None,
            multicast_groups: Vec::new(),
        }],
        selected_socket_id: "main".to_string(),
        multicast_selected_socket_id: "main".to_string(),
        listener_ip_history: Vec::new(),
        listener_port_history: Vec::new(),
        udp_worker: worker,
        rx_event,
        el_tid: "0001".to_string(),
        el_seoj: "05FF01".to_string(),
        el_deoj_preset: 0,
        el_deoj_custom: "0EF001".to_string(),
        el_deoj_eoj: String::new(),
        el_esv_preset: 0,
        el_properties: vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: String::new() }],
        el_show_helper: false,
        multicast_input_addr: "224.0.23.0".to_string(),
        multicast_input_interface: "0.0.0.0".to_string(),
        inspector_protocol: InspectorProtocol::Raw,
        auto_save_enabled: false,
        auto_save_dir: String::new(),
        auto_save_format: LogExportFormat::Csv,
        settings_open: false,
        settings_reset_confirm_open: false,
        about_open: false,
        about_tab: AboutTab::Info,
        tx_logger,
        language_setting: LanguageSetting::English,
        mra_db: udp_packet_studio::mra::MraDatabase::load_empty(),
        max_display_data_bytes: 128,
        max_log_lines: 10000,
    };

    // Frame 1: Render the GUI to determine button layout & coordinate
    let mut raw_input1 = egui::RawInput::default();
    raw_input1.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    
    let full_output = ctx.run_ui(raw_input1, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_sender(ui);
        });
    });

    let click_pos = find_text_center(&full_output.shapes, "🚀 Send")
        .expect("Expected '🚀 Send' text to be rendered on screen");

    // Frame 2: Move mouse to button and Press Down
    let mut raw_input2 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input2.events.push(egui::Event::PointerMoved(click_pos));
    raw_input2.events.push(egui::Event::PointerButton {
        pos: click_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });

    let _ = ctx.run_ui(raw_input2, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_sender(ui);
        });
    });

    // Frame 3: Release Mouse Button (Triggers the Button::clicked() event)
    let mut raw_input3 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input3.events.push(egui::Event::PointerButton {
        pos: click_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });

    let _ = ctx.run_ui(raw_input3, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_sender(ui);
        });
    });

    // Assert: The packet sent from the GUI must be received by the mock partner socket
    let mut buf = [0u8; 1024];
    let (amt, from_addr) = partner.recv_from(&mut buf).expect("Failed to receive packet from worker initiated by GUI click");
    assert_eq!(&buf[..amt], b"Hello GUI World!");
    assert_eq!(from_addr, bound_addr);
}

#[allow(deprecated)]
#[test]
fn test_collections_gui_interactions() {
    use std::net::UdpSocket;
    use std::sync::mpsc::channel;
    use udp_packet_studio::UdpStudioState;
    use udp_packet_studio::locales::LanguageSetting;
    use udp_packet_studio::types::{PayloadType, LoggerCommand, LogExportFormat, InspectorProtocol, AboutTab, Collection, PacketDefinition};
    use udp_packet_studio::udp_worker::{UdpWorker, UdpCommand, UdpEvent};

    let ctx = egui::Context::default();
    let (tx_event, rx_event) = channel();
    
    // Spawn the worker
    let worker = UdpWorker::spawn(tx_event, ctx.clone());
    
    // Bind to an ephemeral port
    worker.send(UdpCommand::Bind { id: "main".to_string(), addr: "127.0.0.1:0".to_string() });
    let bound_addr = match rx_event.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(UdpEvent::Bound { id: _, addr }) => addr,
        other => panic!("Expected Bound event, got {:?}", other),
    };

    // Bind a mock socket as the communication partner
    let partner = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind partner socket");
    let partner_addr = partner.local_addr().expect("Failed to get partner local addr");

    // Create a mock logger channel
    let (tx_logger, _rx_logger) = channel::<LoggerCommand>();

    // Construct the collections state with a pre-existing collection and request
    let test_req_id = "req_123".to_string();
    let test_req = PacketDefinition {
        id: test_req_id.clone(),
        name: "Test Request".to_string(),
        target_ip: partner_addr.ip().to_string(),
        target_port: partner_addr.port().to_string(),
        payload_type: PayloadType::Text,
        payload: "Collection Packets".to_string(),
    };

    let test_col = Collection {
        id: "col_123".to_string(),
        name: "Test Collection".to_string(),
        requests: vec![test_req],
        is_expanded: true,
    };

    // Construct the state with test values
    let mut state = UdpStudioState {
        collections: vec![test_col],
        selected_request_id: Some(test_req_id.clone()),
        composer_selected_collection_idx: 0,
        composer_ip: "127.0.0.1".to_string(),
        composer_port: "9000".to_string(),
        composer_ip_history: Vec::new(),
        composer_port_history: Vec::new(),
        composer_payload_type: PayloadType::Hex,
        composer_payload: "AABBCC".to_string(),
        composer_name: "Composer Request".to_string(),
        logs: Vec::new(),
        selected_log_idx: None,
        filter_text: String::new(),
        filter_input: String::new(),
        filter_history: Vec::new(),
        auto_scroll: true,
        log_export_format: LogExportFormat::Csv,
        filtered_indices: Vec::new(),
        sockets: vec![udp_packet_studio::types::ActiveSocketState {
            id: "main".to_string(),
            name: "Main Socket".to_string(),
            ip: "127.0.0.1".to_string(),
            port: bound_addr.port().to_string(),
            is_listening: true,
            bound_addr: Some(bound_addr.to_string()),
            error: None,
            bind_time: None,
            multicast_groups: Vec::new(),
        }],
        selected_socket_id: "main".to_string(),
        multicast_selected_socket_id: "main".to_string(),
        listener_ip_history: Vec::new(),
        listener_port_history: Vec::new(),
        udp_worker: worker,
        rx_event,
        el_tid: "0001".to_string(),
        el_seoj: "05FF01".to_string(),
        el_deoj_preset: 0,
        el_deoj_custom: "0EF001".to_string(),
        el_deoj_eoj: String::new(),
        el_esv_preset: 0,
        el_properties: vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: String::new() }],
        el_show_helper: false,
        multicast_input_addr: "224.0.23.0".to_string(),
        multicast_input_interface: "0.0.0.0".to_string(),
        inspector_protocol: InspectorProtocol::Raw,
        auto_save_enabled: false,
        auto_save_dir: String::new(),
        auto_save_format: LogExportFormat::Csv,
        settings_open: false,
        settings_reset_confirm_open: false,
        about_open: false,
        about_tab: AboutTab::Info,
        tx_logger,
        language_setting: LanguageSetting::English,
        mra_db: udp_packet_studio::mra::MraDatabase::load_empty(),
        max_display_data_bytes: 128,
        max_log_lines: 10000,
    };

    // ----------------------------------------------------
    // TEST 1: Quick Send Request (🚀 icon next to request name)
    // ----------------------------------------------------
    let mut raw_input1 = egui::RawInput::default();
    raw_input1.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    
    let full_output = ctx.run_ui(raw_input1, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    // Find the 🚀 icon centers. The first one will be in the request list.
    let centers = find_all_text_centers(&full_output.shapes, "🚀");
    assert!(!centers.is_empty(), "Expected at least one '🚀' to be rendered");
    let send_pos = centers[0];

    // Trigger hover and click on the 🚀 button
    let mut raw_input2 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input2.events.push(egui::Event::PointerMoved(send_pos));
    raw_input2.events.push(egui::Event::PointerButton {
        pos: send_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    let _ = ctx.run_ui(raw_input2, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    let mut raw_input3 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input3.events.push(egui::Event::PointerButton {
        pos: send_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    let _ = ctx.run_ui(raw_input3, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    // Assert: The packet sent from Collections list button must be received by partner socket
    let mut buf = [0u8; 1024];
    let (amt, from_addr) = partner.recv_from(&mut buf).expect("Failed to receive packet from Collections quick send click");
    assert_eq!(&buf[..amt], b"Collection Packets");
    assert_eq!(from_addr, bound_addr);

    // ----------------------------------------------------
    // TEST 2: Load to Composer Button (📂 Load to Composer in request editor)
    // ----------------------------------------------------
    let mut raw_input4 = egui::RawInput::default();
    raw_input4.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    let full_output = ctx.run_ui(raw_input4, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    let load_pos = find_text_center(&full_output.shapes, "📂 Load to Composer")
        .expect("Expected '📂 Load to Composer' text to be rendered");

    // Click "Load to Composer"
    let mut raw_input5 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input5.events.push(egui::Event::PointerMoved(load_pos));
    raw_input5.events.push(egui::Event::PointerButton {
        pos: load_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    let _ = ctx.run_ui(raw_input5, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    let mut raw_input6 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input6.events.push(egui::Event::PointerButton {
        pos: load_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    let _ = ctx.run_ui(raw_input6, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    // Assert: The composer state has been populated with the loaded request's target, type, and payload
    assert_eq!(state.composer_ip, partner_addr.ip().to_string());
    assert_eq!(state.composer_port, partner_addr.port().to_string());
    assert_eq!(state.composer_payload_type, PayloadType::Text);
    assert_eq!(state.composer_payload, "Collection Packets".to_string());

    // ----------------------------------------------------
    // TEST 3: Create a New Collection (+ New button)
    // ----------------------------------------------------
    let mut raw_input7 = egui::RawInput::default();
    raw_input7.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    let full_output = ctx.run_ui(raw_input7, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    let new_pos = find_text_center(&full_output.shapes, "+ New")
        .expect("Expected '+ New' button to be rendered");

    // Click "+ New"
    let mut raw_input8 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input8.events.push(egui::Event::PointerMoved(new_pos));
    raw_input8.events.push(egui::Event::PointerButton {
        pos: new_pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    let _ = ctx.run_ui(raw_input8, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    let mut raw_input9 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0))),
        ..Default::default()
    };
    raw_input9.events.push(egui::Event::PointerButton {
        pos: new_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    let _ = ctx.run_ui(raw_input9, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_collections(ui);
        });
    });

    // Assert: A new collection has been added to state
    assert_eq!(state.collections.len(), 2);
    assert_eq!(state.collections[1].name, "Collection 2");
}

fn find_all_text_centers(shapes: &[egui::epaint::ClippedShape], text: &str) -> Vec<egui::Pos2> {
    let mut centers = Vec::new();
    for clipped in shapes {
        if let egui::epaint::Shape::Text(text_shape) = &clipped.shape {
            if text_shape.galley.text().contains(text) {
                let rect = text_shape.galley.rect;
                let world_pos = text_shape.pos;
                centers.push(world_pos + rect.center().to_vec2());
            }
        }
    }
    centers
}

#[allow(deprecated)]
#[test]
fn test_log_limit_and_truncation_gui() {
    use std::sync::mpsc::channel;
    use udp_packet_studio::UdpStudioState;
    use udp_packet_studio::locales::LanguageSetting;
    use udp_packet_studio::types::{PayloadType, LoggerCommand, LogExportFormat, InspectorProtocol, AboutTab, LogEntry, LogDirection};
    use udp_packet_studio::udp_worker::UdpWorker;

    let ctx = egui::Context::default();
    let (tx_event, rx_event) = channel();
    let worker = UdpWorker::spawn(tx_event, ctx.clone());
    let (tx_logger, _rx_logger) = channel::<LoggerCommand>();

    let mut state = UdpStudioState {
        collections: Vec::new(),
        selected_request_id: None,
        composer_selected_collection_idx: 0,
        composer_ip: "127.0.0.1".to_string(),
        composer_port: "9000".to_string(),
        composer_ip_history: Vec::new(),
        composer_port_history: Vec::new(),
        composer_payload_type: PayloadType::Text,
        composer_payload: String::new(),
        composer_name: String::new(),
        logs: Vec::new(),
        selected_log_idx: None,
        filter_text: String::new(),
        filter_input: String::new(),
        filter_history: Vec::new(),
        auto_scroll: true,
        log_export_format: LogExportFormat::Csv,
        filtered_indices: Vec::new(),
        sockets: Vec::new(),
        selected_socket_id: "main".to_string(),
        multicast_selected_socket_id: "main".to_string(),
        listener_ip_history: Vec::new(),
        listener_port_history: Vec::new(),
        udp_worker: worker,
        rx_event,
        el_tid: "0001".to_string(),
        el_seoj: "05FF01".to_string(),
        el_deoj_preset: 0,
        el_deoj_custom: "0EF001".to_string(),
        el_deoj_eoj: String::new(),
        el_esv_preset: 0,
        el_properties: vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: String::new() }],
        el_show_helper: false,
        multicast_input_addr: "224.0.23.0".to_string(),
        multicast_input_interface: "0.0.0.0".to_string(),
        inspector_protocol: InspectorProtocol::Raw,
        auto_save_enabled: false,
        auto_save_dir: String::new(),
        auto_save_format: LogExportFormat::Csv,
        settings_open: false,
        settings_reset_confirm_open: false,
        about_open: false,
        about_tab: AboutTab::Info,
        tx_logger,
        language_setting: LanguageSetting::English,
        mra_db: udp_packet_studio::mra::MraDatabase::load_empty(),
        max_display_data_bytes: 128, // display limit
        max_log_lines: 10000,         // stored limit
    };

    // 1. Add 10,005 items, each with 1KB data.
    let data_1kb = vec![0xAA; 1024]; // 1KB data
    for _ in 0..10005 {
        let entry = LogEntry::new(
            chrono::Local::now(),
            LogDirection::Received,
            std::net::SocketAddr::from(([127, 0, 0, 1], 9000)),
            data_1kb.clone(),
        );
        state.push_log(entry);
    }

    // Verify logs count is capped at 10,000
    assert_eq!(state.logs.len(), 10000);

    // Verify each log contains exactly 1024 bytes (1KB)
    for entry in &state.logs {
        assert_eq!(entry.data.len(), 1024);
    }

    // Check dynamic preview generation in data model
    let expected_preview_start = vec!["AA"; 128].join(" ");
    let expected_full_preview = format!("{}...", expected_preview_start);
    assert_eq!(state.logs[0].get_preview(state.max_display_data_bytes), expected_full_preview);

    // 2. Render the log viewer GUI.
    let mut raw_input = egui::RawInput::default();
    raw_input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    let full_output = ctx.run_ui(raw_input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_log_viewer(ui);
        });
    });

    // Verify that the truncated preview text is rendered in the GUI
    // Search for a prefix to avoid galley wrapped layout mismatches
    let partial_expected_preview = vec!["AA"; 20].join(" ");
    let rendered_previews = find_all_text_centers(&full_output.shapes, &partial_expected_preview);
    assert!(!rendered_previews.is_empty(), "Expected to find some previews rendered in the log table");

    // 3. Verify Inspector displays full data (no truncation)
    // Select the first log item
    state.selected_log_idx = Some(0);
    state.inspector_protocol = InspectorProtocol::Raw;

    let mut raw_input2 = egui::RawInput::default();
    raw_input2.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    let inspector_output = ctx.run_ui(raw_input2, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_inspector(ui);
        });
    });

    // The inspector raw protocol hex dump for 1024 bytes has 64 rows of 16 bytes.
    // e.g. "0000: AA AA AA AA AA AA AA AA AA AA AA AA AA AA AA AA"
    // Let's check that the line prefix "03F0:" (for byte offset 1008 = 0x03F0) is rendered,
    // which proves that the full dump (up to 1024 bytes) is rendered.
    let found_last_offset = find_text_center(&inspector_output.shapes, "03f0:");
    assert!(found_last_offset.is_some(), "Expected inspector to display the full 1024 bytes (found 03f0: offset)");
}


