use std::net::SocketAddr;
use chrono::Local;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadType {
    Text,
    Hex,
    EchonetLite,
    Syslog,
    Snmp,
    Dns,
    Coap,
}

pub fn default_payload_type() -> PayloadType {
    PayloadType::Hex
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InspectorProtocol {
    Raw,
    TextAscii,
    EchonetLite,
    Syslog,
    Snmp,
    Dns,
    Coap,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnmpValueType {
    Integer,
    OctetString,
    ObjectId,
    Null,
    IpAddress,
    Counter32,
    Gauge32,
    TimeTicks,
}

impl Default for SnmpValueType {
    fn default() -> Self {
        Self::Null
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnmpVarBindState {
    pub oid: String,
    pub value_type: SnmpValueType,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogExportFormat {
    Csv,
    Json,
    Pcap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketDefinition {
    pub id: String,
    pub name: String,
    pub target_ip: String,
    pub target_port: String,
    #[serde(default = "default_payload_type")]
    pub payload_type: PayloadType,
    pub payload: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogDirection {
    Sent,
    Received,
    SystemInfo,
    SystemError,
}

fn default_socket_addr() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<Local>,
    pub direction: LogDirection,
    #[serde(skip, default = "default_socket_addr")]
    pub address: SocketAddr,
    #[serde(skip)]
    pub address_str: String,
    pub data: Vec<u8>,
    #[serde(skip)]
    pub preview_str: String,
    pub local_ip: Option<String>,
    pub local_port: Option<String>,
    pub src_ip: String,
    pub src_port: String,
    pub dest_ip: String,
    pub dest_port: String,
}

impl LogEntry {
    pub fn new(
        timestamp: chrono::DateTime<Local>,
        direction: LogDirection,
        address: SocketAddr,
        data: Vec<u8>,
    ) -> Self {
        Self::new_with_local(timestamp, direction, address, None, data)
    }

    pub fn new_with_local(
        timestamp: chrono::DateTime<Local>,
        direction: LogDirection,
        address: SocketAddr,
        local_addr: Option<SocketAddr>,
        data: Vec<u8>,
    ) -> Self {
        let address_str = address.to_string();
        let (local_ip, local_port) = match local_addr {
            Some(addr) => (Some(addr.ip().to_string()), Some(addr.port().to_string())),
            None => (None, None),
        };
        
        let is_system = direction == LogDirection::SystemInfo || direction == LogDirection::SystemError;
        
        let src_ip = if is_system {
            "-".to_string()
        } else if direction == LogDirection::Sent {
            local_ip.clone().unwrap_or_else(|| "0.0.0.0".to_string())
        } else {
            address.ip().to_string()
        };

        let src_port = if is_system {
            "-".to_string()
        } else if direction == LogDirection::Sent {
            local_port.clone().unwrap_or_else(|| "0".to_string())
        } else {
            address.port().to_string()
        };

        let dest_ip = if is_system {
            "-".to_string()
        } else if direction == LogDirection::Sent {
            address.ip().to_string()
        } else {
            local_ip.clone().unwrap_or_else(|| "0.0.0.0".to_string())
        };

        let dest_port = if is_system {
            "-".to_string()
        } else if direction == LogDirection::Sent {
            address.port().to_string()
        } else {
            local_port.clone().unwrap_or_else(|| "0".to_string())
        };

        let preview_str = match direction {
            LogDirection::Sent | LogDirection::Received => {
                let hex_str = data.iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<String>>()
                    .join(" ");
                if hex_str.len() > 80 {
                    format!("{}...", &hex_str[..77])
                } else {
                    hex_str
                }
            }
            LogDirection::SystemInfo | LogDirection::SystemError => {
                let payload_preview = String::from_utf8_lossy(&data);
                let preview = payload_preview.replace('\n', " ");
                if preview.chars().count() > 80 {
                    format!("{}...", preview.chars().take(77).collect::<String>())
                } else {
                    preview
                }
            }
        };

        Self {
            timestamp,
            direction,
            address,
            address_str,
            data,
            preview_str,
            local_ip,
            local_port,
            src_ip,
            src_port,
            dest_ip,
            dest_port,
        }
    }

    pub fn get_preview(&self, max_bytes: usize) -> String {
        match self.direction {
            LogDirection::Sent | LogDirection::Received => {
                if self.data.is_empty() {
                    return String::new();
                }
                let limit = std::cmp::min(self.data.len(), max_bytes);
                let hex_str = self.data[..limit].iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<String>>()
                    .join(" ");
                if self.data.len() > max_bytes {
                    if hex_str.is_empty() {
                        "...".to_string()
                    } else {
                        format!("{}...", hex_str)
                    }
                } else {
                    hex_str
                }
            }
            LogDirection::SystemInfo | LogDirection::SystemError => {
                let payload_preview = String::from_utf8_lossy(&self.data);
                let preview = payload_preview.replace('\n', " ");
                let char_limit = max_bytes.max(80);
                if preview.chars().count() > char_limit {
                    format!("{}...", preview.chars().take(char_limit.saturating_sub(3)).collect::<String>())
                } else {
                    preview
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElBuilderProperty {
    pub epc: String,
    pub edt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocketConfig {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: String,
}

#[derive(Debug, Clone)]
pub struct ActiveSocketState {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: String,
    pub is_listening: bool,
    pub bound_addr: Option<String>,
    pub error: Option<String>,
    pub bind_time: Option<chrono::DateTime<chrono::Local>>,
    pub multicast_groups: Vec<MulticastGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub requests: Vec<PacketDefinition>,
    pub is_expanded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MulticastGroup {
    pub multi_addr: String,
    pub interface_addr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    Collections,
    Sender,
    LogViewer,
    Inspector,
    Multicast,
    Sockets,
}

// Helper utility: parsing Hex sequences like "48 65 6c 6c 6f"
pub fn parse_hex_to_bytes(hex_str: &str) -> Result<Vec<u8>, String> {
    let clean: String = hex_str
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    if clean.len() % 2 != 0 {
        return Err("Hex string must have an even number of hex digits (excluding spaces)".to_string());
    }
    let mut bytes = Vec::with_capacity(clean.len() / 2);
    for i in (0..clean.len()).step_by(2) {
        let hex_byte = &clean[i..i+2];
        match u8::from_str_radix(hex_byte, 16) {
            Ok(b) => bytes.push(b),
            Err(e) => return Err(format!("Invalid hex pair '{}': {}", hex_byte, e)),
        }
    }
    Ok(bytes)
}

pub fn parse_echonet_lite(bytes: &[u8]) -> Option<(String, String, String, usize, Vec<ElBuilderProperty>)> {
    if bytes.len() < 12 {
        return None;
    }
    let ehd1 = bytes[0];
    let ehd2 = bytes[1];
    if ehd1 != 0x10 || ehd2 != 0x81 {
        return None;
    }
    let tid = format!("{:02X}{:02X}", bytes[2], bytes[3]);
    let seoj = format!("{:02X}{:02X}{:02X}", bytes[4], bytes[5], bytes[6]);
    let deoj = format!("{:02X}{:02X}{:02X}", bytes[7], bytes[8], bytes[9]);
    let esv_byte = bytes[10];
    let opc = bytes[11] as usize;
    
    let esv_preset = match esv_byte {
        0x62 => 0,
        0x61 => 1,
        0x60 => 2,
        0x63 => 3,
        0x73 => 4,
        0x7A => 5,
        0x6E => 6,
        _ => 0,
    };
    
    let mut properties = Vec::new();
    let mut offset = 12;
    for _ in 0..opc {
        if offset + 2 > bytes.len() {
            return None;
        }
        let epc = format!("{:02X}", bytes[offset]);
        let pdc = bytes[offset + 1] as usize;
        offset += 2;
        if offset + pdc > bytes.len() {
            return None;
        }
        let edt = bytes[offset..offset+pdc].iter().map(|b| format!("{:02X}", b)).collect::<String>();
        offset += pdc;
        properties.push(ElBuilderProperty { epc, edt });
    }
    
    Some((tid, seoj, deoj, esv_preset, properties))
}

pub fn generate_echonet_lite(
    tid: &str,
    seoj: &str,
    deoj: &str,
    esv_preset: usize,
    properties: &[ElBuilderProperty],
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    bytes.push(0x10);
    bytes.push(0x81);
    
    let tid_clean: String = tid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if tid_clean.len() != 4 {
        return Err("TID must be 4 hex characters".to_string());
    }
    let tid_bytes = parse_hex_to_bytes(&tid_clean)?;
    bytes.extend_from_slice(&tid_bytes);
    
    let seoj_clean: String = seoj.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if seoj_clean.len() != 6 {
        return Err("SEOJ must be 6 hex characters".to_string());
    }
    let seoj_bytes = parse_hex_to_bytes(&seoj_clean)?;
    bytes.extend_from_slice(&seoj_bytes);
    
    let deoj_clean: String = deoj.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if deoj_clean.len() != 6 {
        return Err("DEOJ must be 6 hex characters".to_string());
    }
    let deoj_bytes = parse_hex_to_bytes(&deoj_clean)?;
    bytes.extend_from_slice(&deoj_bytes);
    
    let esv = match esv_preset {
        0 => 0x62,
        1 => 0x61,
        2 => 0x60,
        3 => 0x63,
        4 => 0x73,
        5 => 0x7A,
        6 => 0x6E,
        _ => 0x62,
    };
    bytes.push(esv);
    
    let is_get = esv == 0x62 || esv == 0x63;
    
    if properties.is_empty() {
        return Err("At least one property must be specified".to_string());
    }
    
    bytes.push(properties.len() as u8);
    
    for prop in properties {
        let epc_clean: String = prop.epc.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if epc_clean.len() != 2 {
            return Err("EPC must be 2 hex characters".to_string());
        }
        let epc_byte = parse_hex_to_bytes(&epc_clean)?[0];
        bytes.push(epc_byte);
        
        if is_get {
            bytes.push(0x00);
        } else {
            let edt_clean: String = prop.edt.chars().filter(|c| c.is_ascii_hexdigit()).collect();
            if edt_clean.is_empty() {
                return Err("EDT cannot be empty".to_string());
            }
            if edt_clean.len() % 2 != 0 {
                return Err("EDT must have an even number of characters".to_string());
            }
            let edt_bytes = parse_hex_to_bytes(&edt_clean)?;
            bytes.push(edt_bytes.len() as u8);
            bytes.extend_from_slice(&edt_bytes);
        }
    }
    
    Ok(bytes)
}

pub fn validate_payload(payload: &str, payload_type: PayloadType) -> Result<Vec<u8>, String> {
    match payload_type {
        PayloadType::Text | PayloadType::Syslog => {
            Ok(payload.as_bytes().to_vec())
        }
        PayloadType::Hex | PayloadType::EchonetLite | PayloadType::Snmp | PayloadType::Dns | PayloadType::Coap => {
            let has_invalid_chars = payload.chars().any(|c| {
                !c.is_ascii_hexdigit()
                    && !c.is_whitespace()
                    && c != ':'
                    && c != '-'
                    && c != ','
            });
            if has_invalid_chars {
                return Err("Contains invalid characters (only hex digits, spaces, and delimiters :, -, are allowed).".to_string());
            }
            match parse_hex_to_bytes(payload) {
                Ok(bytes) => {
                    Ok(bytes)
                }
                Err(e) => {
                    if e.contains("must have an even number") {
                        Err("Hex string must have an even number of hex digits (excluding spaces).".to_string())
                    } else {
                        Err(format!("Invalid hex pair: {}", e))
                    }
                }
            }
        }
    }
}

// Helper utility: generate pseudo-UUIDs based on timestamp
pub fn generate_id() -> String {
    use std::time::SystemTime;
    let n = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("pkt_{}", n)
}

// Wireshark style Hex Dump visualizer
pub fn to_hex_dump(bytes: &[u8]) -> String {
    let mut result = String::new();
    let chunk_size = 16;
    for (i, chunk) in bytes.chunks(chunk_size).enumerate() {
        let offset = i * chunk_size;
        result.push_str(&format!("{:04x}:  ", offset));
        
        // Render hex representation
        for (j, byte) in chunk.iter().enumerate() {
            result.push_str(&format!("{:02x} ", byte));
            if j == 7 {
                result.push(' ');
            }
        }
        
        // Pad for uneven rows
        if chunk.len() < chunk_size {
            let padding = chunk_size - chunk.len();
            for j in 0..padding {
                result.push_str("   ");
                if chunk.len() + j == 7 {
                    result.push(' ');
                }
            }
        }
        
        result.push_str(" |");
        
        // Render ASCII graphic values
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                result.push(*byte as char);
            } else {
                result.push('.');
            }
        }
        result.push_str("|\n");
    }
    result
}

#[derive(Debug, Clone)]
pub enum LoggerCommand {
    Log(LogEntry),
    Configure {
        enabled: bool,
        dir: String,
        format: LogExportFormat,
        listener_addr: String,
        bind_time: Option<chrono::DateTime<chrono::Local>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AboutTab {
    Info,
    ThirdParty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingsTab {
    General,
    LogDisplay,
    LogSaving,
    Protocols,
    Others,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    System,
    Light,
    Dark,
}

impl Default for AppTheme {
    fn default() -> Self {
        AppTheme::System
    }
}

fn default_dns_port() -> String {
    "53,5353".to_string()
}

fn default_coap_port() -> String {
    "5683".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolConfig {
    pub echonet_lite_port: String,
    pub snmp_agent_port: String,
    pub snmp_trap_port: String,
    pub syslog_port: String,
    #[serde(default = "default_dns_port")]
    pub dns_port: String,
    #[serde(default = "default_coap_port")]
    pub coap_port: String,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            echonet_lite_port: "3610".to_string(),
            snmp_agent_port: "161".to_string(),
            snmp_trap_port: "162".to_string(),
            syslog_port: "514".to_string(),
            dns_port: "53,5353".to_string(),
            coap_port: "5683".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresetPortItem {
    pub protocol: String,
    pub port: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingFormatChange {
    pub request_id: Option<String>,
    pub from_type: PayloadType,
    pub to_type: PayloadType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatChangeResult {
    Immediate { new_payload: String },
    Pending(PendingFormatChange),
}



