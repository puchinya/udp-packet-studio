use std::net::SocketAddr;
use std::sync::mpsc::channel;
use chrono::Local;
use udp_packet_studio::UdpStudioState;
use udp_packet_studio::types::{LogEntry, LogDirection, PayloadType, LogExportFormat, InspectorProtocol};
use udp_packet_studio::udp_worker::UdpWorker;
use udp_packet_studio::views::collections::{YamlCollection, YamlRequest};
use udp_packet_studio::views::log_viewer::write_pcap_helper;

fn make_test_state() -> UdpStudioState {
    let (tx, rx) = channel();
    let worker = UdpWorker::spawn(tx, egui::Context::default());
    let (tx_logger, _) = channel();
    UdpStudioState {
        collections: Vec::new(),
        selected_request_id: None,
        composer_selected_collection_idx: 0,
        composer_target: String::new(),
        composer_payload_type: PayloadType::Text,
        composer_payload: String::new(),
        composer_name: String::new(),
        logs: Vec::new(),
        selected_log_idx: None,
        filter_text: String::new(),
        auto_scroll: true,
        log_export_format: LogExportFormat::Csv,
        filtered_indices: Vec::new(),
        listener_addr: String::new(),
        is_listening: false,
        bound_addr: None,
        listener_error: None,
        udp_worker: worker,
        rx_event: rx,
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
                target: "127.0.0.1:3610".to_string(),
                payload_type: PayloadType::Hex,
                payload: "10 81 00 01 05".to_string(),
            },
            YamlRequest {
                name: "Plain Text".to_string(),
                target: "192.168.1.100:9000".to_string(),
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
    state.el_deoj_preset = 0;
    state.el_esv_preset = 0;
    state.el_epc_preset = 0;

    let result = state.generate_echonet_lite_hex().unwrap();
    assert_eq!(result, "10 81 00 0A 05 FF 01 01 30 01 62 01 80 00");
}

#[test]
fn test_generate_echonet_lite_hex_set() {
    let mut state = make_test_state();
    state.el_tid = "1234".to_string();
    state.el_seoj = "05FF01".to_string();
    state.el_deoj_preset = 0;
    state.el_esv_preset = 1;
    state.el_epc_preset = 0;
    state.el_edt = "30".to_string();

    let result = state.generate_echonet_lite_hex().unwrap();
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
    csv_content.push_str("No,Timestamp,Direction,IP,Port,Length,DataHex,DataText\n");
    for (idx, entry) in logs.iter().enumerate() {
        let time_str = entry.timestamp.format("%Y-%m-%d %H:%M:%S.%3f").to_string();
        let dir_str = match entry.direction {
            LogDirection::Sent => "SENT",
            LogDirection::Received => "RECV",
            LogDirection::SystemInfo => "INFO",
            LogDirection::SystemError => "ERROR",
        };
        let len_str = entry.data.len().to_string();
        let hex_str = entry.data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ");
        let plain_str = String::from_utf8_lossy(&entry.data).replace('\n', " ").replace('"', "\"\"");
        csv_content.push_str(&format!("{},\"{}\",\"{}\",\"{}\",\"{}\",{},\"{}\",\"{}\"\n", 
            idx + 1, time_str, dir_str, entry.ip, entry.port, len_str, hex_str, plain_str));
    }

    assert!(csv_content.contains("1,\"2026-06-13 12:00:00.000\",\"SENT\",\"127.0.0.1\",\"9000\",5,\"48 65 6C 6C 6F\",\"Hello\""));
    assert!(csv_content.contains("2,\"2026-06-13 12:00:00.000\",\"RECV\",\"192.168.1.50\",\"5000\",4,\"10 81 00 01\""));
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
