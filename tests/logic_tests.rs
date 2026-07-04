use std::net::SocketAddr;
use std::sync::mpsc::channel;
use chrono::Local;
use udp_packet_studio::UdpStudioState;
use udp_packet_studio::types::{LogEntry, LogDirection, PayloadType, LogExportFormat, InspectorProtocol, AboutTab, SettingsTab, AppTheme, validate_payload};
use udp_packet_studio::udp_worker::UdpWorker;
use udp_packet_studio::views::collections::{YamlCollection, YamlRequest};
use udp_packet_studio::views::log_viewer::write_pcap_helper;

fn make_test_state() -> UdpStudioState {
    let (tx, rx) = channel();
    let worker = UdpWorker::spawn(tx, egui::Context::default());
    let (tx_logger, _) = channel();
    UdpStudioState {
        theme: AppTheme::System,
        collections: Vec::new(),
        selected_request_id: None,
        composer_selected_collection_idx: 0,
        composer_ip: String::new(),
        composer_port: String::new(),
        composer_ip_history: Vec::new(),
        composer_port_history: Vec::new(),
        composer_payload_type: PayloadType::Text,
        composer_payload: String::new(),
        composer_name: String::new(),
        logs: Vec::new(),
        selected_log_idx: None,
        selected_log_indices: std::collections::BTreeSet::new(),
        last_clicked_log_idx: None,
        filter_text: String::new(),
        filter_input: String::new(),
        filter_history: Vec::new(),
        dock_state_serialized: None,
        reset_layout_requested: false,
        auto_scroll: true,
        log_export_format: LogExportFormat::Csv,
        filtered_indices: Vec::new(),
        sockets: vec![udp_packet_studio::types::ActiveSocketState {
            id: "main".to_string(),
            name: "Main Socket".to_string(),
            ip: "0.0.0.0".to_string(),
            port: "9000".to_string(),
            is_listening: false,
            bound_addr: None,
            error: None,
            bind_time: None,
            multicast_groups: Vec::new(),
        }],
        selected_socket_id: "main".to_string(),
        multicast_selected_socket_id: "main".to_string(),
        listener_ip_history: Vec::new(),
        listener_port_history: Vec::new(),
        udp_worker: worker,
        rx_event: rx,
        el_tid: "0001".to_string(),
        el_seoj: "05FF01".to_string(),
        el_deoj_preset: 0,
        el_deoj_custom: "0EF001".to_string(),
        el_deoj_eoj: String::new(),
        el_esv_preset: 0,
        el_properties: vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: String::new() }],
        el_show_helper: false,
        syslog_show_helper: false,
        syslog_protocol_version: 0,
        syslog_facility: 1,
        syslog_severity: 5,
        syslog_auto_timestamp: true,
        syslog_timestamp: String::new(),
        syslog_hostname: "localhost".to_string(),
        syslog_app_name: "udp-packet-studio".to_string(),
        syslog_proc_id: "-".to_string(),
        syslog_msg_id: "-".to_string(),
        syslog_msg: "Hello, Syslog!".to_string(),
        snmp_show_helper: false,
        snmp_version: 1,
        snmp_community: "public".to_string(),
        snmp_pdu_type: 0,
        snmp_request_id: 1,
        snmp_error_status: 0,
        snmp_error_index: 0,
        snmp_varbinds: vec![udp_packet_studio::types::SnmpVarBindState {
            oid: "1.3.6.1.2.1.1.1.0".to_string(),
            value_type: udp_packet_studio::types::SnmpValueType::Null,
            value: String::new(),
        }],
        multicast_input_addr: "224.0.23.0".to_string(),
        multicast_input_interface: "0.0.0.0".to_string(),
        inspector_protocol: InspectorProtocol::Raw,
        auto_save_enabled: false,
        auto_save_dir: String::new(),
        auto_save_format: LogExportFormat::Csv,
        settings_open: false,
        settings_reset_confirm_open: false,
        settings_tab: SettingsTab::General,
        about_open: false,
        about_tab: AboutTab::Info,
        tx_logger,
        language_setting: udp_packet_studio::locales::LanguageSetting::English,
        mra_db: udp_packet_studio::mra::MraDatabase::load_empty(),
        max_display_data_bytes: 128,
        max_log_lines: 10000,
        protocol_config: udp_packet_studio::types::ProtocolConfig::default(),
        settings_selected_proto_tab: 0,
        inspector_protocols_order: vec![
            udp_packet_studio::types::InspectorProtocol::EchonetLite,
            udp_packet_studio::types::InspectorProtocol::Syslog,
            udp_packet_studio::types::InspectorProtocol::Snmp,
            udp_packet_studio::types::InspectorProtocol::Dns,
            udp_packet_studio::types::InspectorProtocol::Coap,
        ],
        preset_ports_order: vec![
            udp_packet_studio::types::PresetPortItem { protocol: "ECHONET Lite".to_string(), port: "3610".to_string() },
            udp_packet_studio::types::PresetPortItem { protocol: "Syslog".to_string(), port: "514".to_string() },
            udp_packet_studio::types::PresetPortItem { protocol: "SNMP Agent".to_string(), port: "161".to_string() },
            udp_packet_studio::types::PresetPortItem { protocol: "SNMP Trap".to_string(), port: "162".to_string() },
            udp_packet_studio::types::PresetPortItem { protocol: "DNS".to_string(), port: "53".to_string() },
            udp_packet_studio::types::PresetPortItem { protocol: "CoAP".to_string(), port: "5683".to_string() },
        ],
        protocol_mru: vec!["ECHONET Lite".to_string(), "Syslog".to_string(), "SNMP".to_string(), "DNS".to_string(), "CoAP".to_string()],
        composer_pending_format_change: None,
        collection_pending_format_change: None,
        last_selected_request_id: None,
        req_el_tid: "0001".to_string(),
        req_el_seoj: "05FF01".to_string(),
        req_el_deoj_preset: 0,
        req_el_deoj_custom: "0EF001".to_string(),
        req_el_deoj_eoj: String::new(),
        req_el_esv_preset: 0,
        req_el_properties: vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: String::new() }],
        req_syslog_protocol_version: 0,
        req_syslog_facility: 1,
        req_syslog_severity: 5,
        req_syslog_auto_timestamp: true,
        req_syslog_timestamp: String::new(),
        req_syslog_hostname: "localhost".to_string(),
        req_syslog_app_name: "udp-packet-studio".to_string(),
        req_syslog_proc_id: "-".to_string(),
        req_syslog_msg_id: "-".to_string(),
        req_syslog_msg: "Hello, Syslog!".to_string(),
        req_snmp_version: 1,
        req_snmp_community: "public".to_string(),
        req_snmp_pdu_type: 0,
        req_snmp_request_id: 1,
        req_snmp_error_status: 0,
        req_snmp_error_index: 0,
        req_snmp_varbinds: vec![udp_packet_studio::types::SnmpVarBindState {
            oid: "1.3.6.1.2.1.1.1.0".to_string(),
            value_type: udp_packet_studio::types::SnmpValueType::Null,
            value: String::new(),
        }],
        editing_request_id: None,
        req_el_show_helper: false,
        req_syslog_show_helper: false,
        req_snmp_show_helper: false,
        composer_selected_proto: PayloadType::EchonetLite,
        collections_selected_proto: PayloadType::EchonetLite,
        dns_show_helper: false,
        dns_transaction_id: 1,
        dns_flags: 0x0100,
        dns_qname: "google.com".to_string(),
        dns_qtype: 1,
        dns_qclass: 1,
        coap_show_helper: false,
        coap_version: 1,
        coap_mtype: 0,
        coap_code: 1,
        coap_message_id: 1,
        coap_token: "DEAD".to_string(),
        coap_options: vec![
            udp_packet_studio::coap::CoapOptionState { number: "11".to_string(), value: "sensors".to_string() }
        ],
        coap_payload: String::new(),
        req_dns_show_helper: false,
        req_dns_transaction_id: 1,
        req_dns_flags: 0x0100,
        req_dns_qname: "google.com".to_string(),
        req_dns_qtype: 1,
        req_dns_qclass: 1,
        req_coap_show_helper: false,
        req_coap_version: 1,
        req_coap_mtype: 0,
        req_coap_code: 1,
        req_coap_message_id: 1,
        req_coap_token: "DEAD".to_string(),
        req_coap_options: vec![
            udp_packet_studio::coap::CoapOptionState { number: "11".to_string(), value: "sensors".to_string() }
        ],
        req_coap_payload: String::new(),
    }
}

#[test]
fn test_long_multibyte_system_error_preview() {
    let msg = "送信エラーが発生しました。アドレスの解決に失敗したか、指定されたホスト名またはIPアドレスが正しくありません。再度設定を確認してください。";
    // Ensure the byte length is > 80, but character count is < 80.
    assert!(msg.len() > 80);
    assert!(msg.chars().count() < 80);
    
    let entry = LogEntry::new(
        Local::now(),
        LogDirection::SystemError,
        SocketAddr::from(([0, 0, 0, 0], 0)),
        msg.as_bytes().to_vec(),
    );
    
    // Should not be truncated
    assert!(!entry.preview_str.ends_with("..."));
    assert_eq!(entry.preview_str, msg);

    // Now test one that is longer than 80 characters
    let long_msg = "送信エラーが発生しました。アドレスの解決に失敗したか、指定されたホスト名またはIPアドレスが正しくありません。再度設定を確認してください。再度設定を確認してください。";
    assert!(long_msg.chars().count() > 80);

    let entry_long = LogEntry::new(
        Local::now(),
        LogDirection::SystemError,
        SocketAddr::from(([0, 0, 0, 0], 0)),
        long_msg.as_bytes().to_vec(),
    );

    assert!(entry_long.preview_str.ends_with("..."));
    assert_eq!(entry_long.preview_str.chars().count(), 80); // 77 chars from msg + "..." (3 chars)
}

#[test]
fn test_yaml_serialization_roundtrip() {
    let original = YamlCollection {
        name: "Test Collection".to_string(),
        requests: vec![
            YamlRequest {
                name: "Get Operation".to_string(),
                target_ip: "127.0.0.1".to_string(),
                target_port: "3610".to_string(),
                payload_type: PayloadType::Hex,
                payload: "10 81 00 01 05".to_string(),
            },
            YamlRequest {
                name: "Plain Text".to_string(),
                target_ip: "192.168.1.100".to_string(),
                target_port: "9000".to_string(),
                payload_type: PayloadType::Text,
                payload: "Hello world".to_string(),
            },
        ],
    };

    let serialized = serde_yaml::to_string(&original).unwrap();
    let deserialized: YamlCollection = serde_yaml::from_str(&serialized).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_yaml_deserialization_defaults() {
    let yaml_str = "name: Empty Collection\n";
    let deserialized: YamlCollection = serde_yaml::from_str(yaml_str).unwrap();

    assert_eq!(deserialized.name, "Empty Collection");
    assert!(deserialized.requests.is_empty());
}

#[test]
fn test_generate_echonet_lite_hex_get() {
    let mut state = make_test_state();
    state.el_tid = "000A".to_string();
    state.el_seoj = "05FF01".to_string();
    // preset 0 = custom, el_deoj_custom = "0EF001"
    state.el_deoj_preset = 0;
    state.el_deoj_custom = "0EF001".to_string();
    state.el_esv_preset = 0; // Get
    state.el_properties = vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: String::new() }];

    let result = state.generate_echonet_lite_hex().unwrap();
    // EHD=1081 TID=000A SEOJ=05FF01 DEOJ=0EF001 ESV=62 OPC=01 EPC=80 PDC=00
    assert_eq!(result, "10 81 00 0A 05 FF 01 0E F0 01 62 01 80 00");
}

#[test]
fn test_generate_echonet_lite_hex_set() {
    let mut state = make_test_state();
    state.el_tid = "1234".to_string();
    state.el_seoj = "05FF01".to_string();
    state.el_deoj_preset = 0;
    state.el_deoj_custom = "013001".to_string();
    state.el_esv_preset = 1; // SetC
    state.el_properties = vec![udp_packet_studio::types::ElBuilderProperty { epc: "80".to_string(), edt: "30".to_string() }];

    let result = state.generate_echonet_lite_hex().unwrap();
    // EHD=1081 TID=1234 SEOJ=05FF01 DEOJ=013001 ESV=61 OPC=01 EPC=80 PDC=01 EDT=30
    assert_eq!(result, "10 81 12 34 05 FF 01 01 30 01 61 01 80 01 30");
}

#[test]
fn test_csv_formatting() {
    use chrono::TimeZone;
    let timestamp = chrono::Local.with_ymd_and_hms(2026, 6, 13, 12, 0, 0).unwrap();
    let logs = vec![
        LogEntry::new(
            timestamp,
            LogDirection::Sent,
            "127.0.0.1:9000".parse().unwrap(),
            b"Hello".to_vec(),
        ),
        LogEntry::new(
            timestamp,
            LogDirection::Received,
            "192.168.1.50:5000".parse().unwrap(),
            vec![0x10, 0x81, 0x00, 0x01],
        ),
    ];

    let mut csv_content = String::new();
    csv_content.push_str("No,Timestamp,Direction,Src IP,Src Port,Dest IP,Dest Port,Length,DataHex,DataText\n");
    for (idx, entry) in logs.iter().enumerate() {
        let time_str = entry.timestamp.format("%Y-%m-%d %H:%M:%S.%3f").to_string();
        let dir_str = match entry.direction {
            LogDirection::Sent => "SENT",
            LogDirection::Received => "RECV",
            LogDirection::SystemInfo => "INFO",
            LogDirection::SystemError => "ERROR",
        };
        
        let is_system = entry.direction == LogDirection::SystemInfo || entry.direction == LogDirection::SystemError;
        
        let src_ip_str = if is_system {
            "-".to_string()
        } else if entry.direction == LogDirection::Sent {
            entry.local_ip.clone().unwrap_or_else(|| "0.0.0.0".to_string())
        } else {
            entry.address.ip().to_string()
        };

        let send_port_str = if is_system {
            "-".to_string()
        } else if entry.direction == LogDirection::Sent {
            entry.local_port.clone().unwrap_or_else(|| "0".to_string())
        } else {
            entry.address.port().to_string()
        };

        let dest_ip_str = if is_system {
            "-".to_string()
        } else if entry.direction == LogDirection::Sent {
            entry.address.ip().to_string()
        } else {
            entry.local_ip.clone().unwrap_or_else(|| "0.0.0.0".to_string())
        };

        let recv_port_str = if is_system {
            "-".to_string()
        } else if entry.direction == LogDirection::Sent {
            entry.address.port().to_string()
        } else {
            entry.local_port.clone().unwrap_or_else(|| "0".to_string())
        };

        let len_str = entry.data.len().to_string();
        let hex_str = entry.data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
        let plain_str = String::from_utf8_lossy(&entry.data).replace('\n', " ").replace('"', "\"\"");
        csv_content.push_str(&format!("{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"\n", 
            idx + 1, time_str, dir_str, src_ip_str, send_port_str, dest_ip_str, recv_port_str, len_str, hex_str, plain_str));
    }

    assert!(csv_content.contains("1,\"2026-06-13 12:00:00.000\",\"SENT\",\"0.0.0.0\",\"0\",\"127.0.0.1\",\"9000\",5,\"48 65 6C 6C 6F\",\"Hello\""));
    assert!(csv_content.contains("2,\"2026-06-13 12:00:00.000\",\"RECV\",\"192.168.1.50\",\"5000\",\"0.0.0.0\",\"0\",4,\"10 81 00 01\""));
}

#[test]
fn test_write_pcap_helper() {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join("test_output.pcap");

    let timestamp = chrono::Local::now();
    let logs = vec![
        LogEntry::new(
            timestamp,
            LogDirection::Sent,
            "127.0.0.1:9000".parse().unwrap(),
            b"Hello".to_vec(),
        ),
        LogEntry::new(
            timestamp,
            LogDirection::Received,
            "192.168.1.50:5000".parse().unwrap(),
            vec![0x10, 0x81, 0x00, 0x01],
        ),
        LogEntry::new(
            timestamp,
            LogDirection::SystemInfo,
            "0.0.0.0:0".parse().unwrap(),
            b"System started".to_vec(),
        ),
    ];

    let result = write_pcap_helper(&path, &logs, "127.0.0.1:9000");
    assert!(result.is_ok());

    // Verify file size and header presence
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 24);
    
    // Check magic number
    let magic = &bytes[0..4];
    assert!(magic == &[0xa1, 0xb2, 0xc3, 0xd4] || magic == &[0xd4, 0xc3, 0xb2, 0xa1]);

    // Clean up
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_validate_payload() {
    // Test text validation
    assert!(validate_payload("Hello", PayloadType::Text).is_ok());
    assert!(validate_payload("", PayloadType::Text).is_ok()); // empty is ok

    // Test hex validation
    assert!(validate_payload("12 34 ab CD", PayloadType::Hex).is_ok());
    assert!(validate_payload("12:34-ab,CD", PayloadType::Hex).is_ok());
    assert!(validate_payload("", PayloadType::Hex).is_ok()); // empty is ok
    assert!(validate_payload("12 3", PayloadType::Hex).is_err()); // odd length
    assert!(validate_payload("12 3x", PayloadType::Hex).is_err()); // invalid char 'x'
}

#[test]
fn test_mra_candidates_loading() {
    let db = udp_packet_studio::mra::MraDatabase::load();
    // 0x0130 is Air Conditioner
    let class_info = db.classes.get(&(0x01, 0x30)).expect("Air Conditioner class not found");
    
    // 0xB0 is Operation Mode Setting (inline enum)
    let prop_b0 = class_info.properties.get(&0xB0).expect("0xB0 property not found");
    assert!(!prop_b0.edt_candidates.is_empty(), "0xB0 edt_candidates should not be empty");
    let has_cooling = prop_b0.edt_candidates.iter().any(|(val, name_ja, _)| val == "42" && name_ja == "冷房");
    assert!(has_cooling, "Candidate '42' (冷房) not found in 0xB0");

    // 0x80 is Operation Status (referenced via definitions.json)
    let prop_80 = class_info.properties.get(&0x80).expect("0x80 property not found");
    assert!(!prop_80.edt_candidates.is_empty(), "0x80 edt_candidates should not be empty");
    let has_on = prop_80.edt_candidates.iter().any(|(val, name_ja, _)| val == "30" && name_ja == "ON");
    assert!(has_on, "Candidate '30' (ON) not found in 0x80");
}

#[test]
fn test_udp_worker_unreachable_port_continuation() {
    use std::time::Duration;
    use std::net::UdpSocket;
    use udp_packet_studio::udp_worker::{UdpWorker, UdpCommand, UdpEvent};

    let (tx_event, rx_event) = channel();
    let ctx = egui::Context::default();
    let worker = UdpWorker::spawn(tx_event, ctx);

    // Bind a socket (let's bind to localhost with dynamic port)
    let socket_id = "test_socket".to_string();
    worker.send(UdpCommand::Bind {
        id: socket_id.clone(),
        addr: "127.0.0.1:0".to_string(),
    });

    // Wait for the bound event and get the local address
    let mut bound_addr = None;
    for _ in 0..50 {
        if let Ok(event) = rx_event.recv_timeout(Duration::from_millis(10)) {
            if let UdpEvent::Bound { id, addr } = event {
                if id == socket_id {
                    bound_addr = Some(addr);
                    break;
                }
            }
        }
    }
    let bound_addr = bound_addr.expect("Failed to bind socket");

    // Now, send a packet to an unreachable port (e.g. 127.0.0.1:1 - port 1 is reserved and almost certainly closed/unreachable)
    worker.send(UdpCommand::Send {
        id: socket_id.clone(),
        target: "127.0.0.1:1".to_string(),
        data: b"unreachable test".to_vec(),
    });

    // Wait for a short duration to let error occur (if any, e.g. WSAECONNRESET on Windows)
    std::thread::sleep(Duration::from_millis(100));

    // Drain and look for UdpEvent::Sent and potential UdpEvent::Error
    let mut got_sent = false;
    let mut _got_error = false;
    while let Ok(event) = rx_event.try_recv() {
        match event {
            UdpEvent::Sent { id, .. } if id == socket_id => {
                got_sent = true;
            }
            UdpEvent::Error { id, err } if id == socket_id => {
                println!("Note: Got expected/potential socket error: {}", err);
                _got_error = true;
            }
            _ => {}
        }
    }
    assert!(got_sent, "Packet should have been sent");

    // Even if an error occurred (or did not occur on some OSes), the socket thread should still be alive.
    // Let's verify by sending a valid packet from an external socket to our bound socket.
    let external_socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind external socket");
    let test_payload = b"still alive!".to_vec();
    external_socket.send_to(&test_payload, bound_addr).expect("Failed to send from external socket");

    // Wait for the received event
    let mut got_received = false;
    for _ in 0..100 {
        if let Ok(event) = rx_event.recv_timeout(Duration::from_millis(10)) {
            if let UdpEvent::Received { id, data, .. } = event {
                if id == socket_id {
                    assert_eq!(data, test_payload);
                    got_received = true;
                    break;
                }
            }
        }
    }

    assert!(got_received, "Socket should still be receiving packets after an unreachable target send");
}

