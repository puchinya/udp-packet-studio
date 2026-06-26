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
    use udp_packet_studio::types::{PayloadType, LoggerCommand, LogExportFormat, InspectorProtocol};
    use udp_packet_studio::udp_worker::{UdpWorker, UdpCommand, UdpEvent};

    let ctx = egui::Context::default();
    let (tx_event, rx_event) = channel();
    
    // Spawn the worker
    let worker = UdpWorker::spawn(tx_event, ctx.clone());
    
    // Bind to an ephemeral port
    worker.send(UdpCommand::Bind("127.0.0.1:0".to_string()));
    let bound_addr = match rx_event.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(UdpEvent::Bound(addr)) => addr,
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
        composer_target: partner_addr.to_string(),
        composer_payload_type: PayloadType::Text,
        composer_payload: "Hello GUI World!".to_string(),
        composer_name: "Test Name".to_string(),
        logs: Vec::new(),
        selected_log_idx: None,
        filter_text: String::new(),
        auto_scroll: true,
        log_export_format: LogExportFormat::Csv,
        filtered_indices: Vec::new(),
        listener_addr: "127.0.0.1:0".to_string(),
        is_listening: true, // Needed to enable the send button
        bound_addr: Some(bound_addr.to_string()),
        listener_error: None,
        udp_worker: worker,
        rx_event,
        el_tid: "0001".to_string(),
        el_seoj: "05FF01".to_string(),
        el_deoj_preset: 0,
        el_deoj_custom: "013001".to_string(),
        el_esv_preset: 0,
        el_epc_preset: 0,
        el_epc_custom: "80".to_string(),
        el_edt: "30".to_string(),
        el_show_helper: false,
        multicast_groups: Vec::new(),
        multicast_input_addr: "224.0.23.0".to_string(),
        multicast_input_interface: "0.0.0.0".to_string(),
        inspector_protocol: InspectorProtocol::Raw,
        auto_save_enabled: false,
        auto_save_dir: String::new(),
        auto_save_format: LogExportFormat::Csv,
        settings_open: false,
        tx_logger,
    };

    // Frame 1: Render the GUI to determine button layout & coordinate
    let mut raw_input1 = egui::RawInput::default();
    raw_input1.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1100.0, 700.0)));
    
    let full_output = ctx.run_ui(raw_input1, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            state.show_sender(ui);
        });
    });

    let click_pos = find_text_center(&full_output.shapes, "🚀 Send Packet")
        .expect("Expected '🚀 Send Packet' text to be rendered on screen");

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


