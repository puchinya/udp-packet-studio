use std::net::SocketAddr;
use chrono::Local;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadType {
    Text,
    Hex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InspectorProtocol {
    Raw,
    TextAscii,
    EchonetLite,
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
    pub target: String,
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
    pub ip: String,
    pub port: String,
    #[serde(skip, default = "default_socket_addr")]
    pub address: SocketAddr,
    #[serde(skip)]
    pub address_str: String,
    pub data: Vec<u8>,
    #[serde(skip)]
    pub preview_str: String,
}

impl LogEntry {
    pub fn new(
        timestamp: chrono::DateTime<Local>,
        direction: LogDirection,
        address: SocketAddr,
        data: Vec<u8>,
    ) -> Self {
        let address_str = address.to_string();
        let (ip, port) = if direction == LogDirection::SystemInfo || direction == LogDirection::SystemError {
            ("-".to_string(), "-".to_string())
        } else {
            (address.ip().to_string(), address.port().to_string())
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
                if preview.len() > 80 {
                    format!("{}...", &preview[..77])
                } else {
                    preview
                }
            }
        };

        Self {
            timestamp,
            direction,
            ip,
            port,
            address,
            address_str,
            data,
            preview_str,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Collections,
    Sender,
    LogViewer,
    Inspector,
    Multicast,
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
    },
}
