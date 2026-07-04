#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoapOption {
    pub number: u16,
    pub name: String,
    pub value: Vec<u8>,
    pub value_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoapDetails {
    pub version: u8,
    pub mtype: u8, // Message Type (0: CON, 1: NON, 2: ACK, 3: RST)
    pub tkl: u8,   // Token Length
    pub code: u8,  // Request Method or Response Code
    pub message_id: u16,
    pub token: Vec<u8>,
    pub options: Vec<CoapOption>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CoapOptionState {
    pub number: String,
    pub value: String,
}

pub fn option_name(num: u16) -> &'static str {
    match num {
        1 => "If-Match",
        3 => "Uri-Host",
        4 => "ETag",
        5 => "If-None-Match",
        7 => "Uri-Port",
        8 => "Location-Path",
        11 => "Uri-Path",
        12 => "Content-Format",
        14 => "Max-Age",
        15 => "Uri-Query",
        17 => "Accept",
        20 => "Location-Query",
        35 => "Proxy-Uri",
        39 => "Proxy-Scheme",
        60 => "Size1",
        _ => "Unknown",
    }
}

pub fn code_name(code: u8) -> String {
    if code == 0 {
        return "0.00 Empty".to_string();
    }
    
    let class = code >> 5;
    let detail = code & 0x1F;
    
    let name = match class {
        0 => match detail {
            1 => "GET",
            2 => "POST",
            3 => "PUT",
            4 => "DELETE",
            _ => "Unknown Request",
        },
        2 => match detail {
            1 => "Created",
            2 => "Deleted",
            3 => "Valid",
            4 => "Changed",
            5 => "Content",
            _ => "Success",
        },
        4 => match detail {
            0 => "Bad Request",
            1 => "Unauthorized",
            2 => "Bad Option",
            3 => "Forbidden",
            4 => "Not Found",
            5 => "Method Not Allowed",
            6 => "Not Acceptable",
            12 => "Precondition Failed",
            13 => "Request Entity Too Large",
            15 => "Unsupported Content-Format",
            _ => "Client Error",
        },
        5 => match detail {
            0 => "Internal Server Error",
            1 => "Not Implemented",
            2 => "Bad Gateway",
            3 => "Service Unavailable",
            4 => "Gateway Timeout",
            5 => "Proxying Not Supported",
            _ => "Server Error",
        },
        _ => "Unknown Code",
    };
    
    format!("{}.{:02} {}", class, detail, name)
}

pub fn parse_coap(bytes: &[u8]) -> Result<CoapDetails, String> {
    if bytes.len() < 4 {
        return Err("CoAP packet too short (less than 4 bytes header)".to_string());
    }
    
    let version = (bytes[0] & 0xC0) >> 6;
    let mtype = (bytes[0] & 0x30) >> 4;
    let tkl = bytes[0] & 0x0F;
    let code = bytes[1];
    let message_id = u16::from_be_bytes([bytes[2], bytes[3]]);
    
    let mut offset = 4;
    
    if offset + (tkl as usize) > bytes.len() {
        return Err(format!("Token length {} exceeds packet size", tkl));
    }
    
    let token = bytes[offset..offset + (tkl as usize)].to_vec();
    offset += tkl as usize;
    
    let mut options = Vec::new();
    let mut current_option_number = 0u16;
    
    while offset < bytes.len() {
        let first_byte = bytes[offset];
        if first_byte == 0xFF {
            // Payload Marker
            offset += 1;
            break;
        }
        
        let delta_nibble = (first_byte & 0xF0) >> 4;
        let length_nibble = first_byte & 0x0F;
        offset += 1;
        
        // Parse Option Delta Extended
        let delta = match delta_nibble {
            13 => {
                if offset >= bytes.len() {
                    return Err("Truncated Option Delta (13 extended byte missing)".to_string());
                }
                let val = bytes[offset] as u16 + 13;
                offset += 1;
                val
            }
            14 => {
                if offset + 1 >= bytes.len() {
                    return Err("Truncated Option Delta (14 extended bytes missing)".to_string());
                }
                let val = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) + 269;
                offset += 2;
                val
            }
            15 => {
                return Err("Invalid Option Delta (nibble value 15 is reserved)".to_string());
            }
            d => d as u16,
        };
        
        current_option_number += delta;
        
        // Parse Option Length Extended
        let length = match length_nibble {
            13 => {
                if offset >= bytes.len() {
                    return Err("Truncated Option Length (13 extended byte missing)".to_string());
                }
                let val = bytes[offset] as usize + 13;
                offset += 1;
                val
            }
            14 => {
                if offset + 1 >= bytes.len() {
                    return Err("Truncated Option Length (14 extended bytes missing)".to_string());
                }
                let val = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize + 269;
                offset += 2;
                val
            }
            15 => {
                return Err("Invalid Option Length (nibble value 15 is reserved)".to_string());
            }
            l => l as usize,
        };
        
        if offset + length > bytes.len() {
            return Err(format!("Option value truncated (expected {} bytes, remaining {})", length, bytes.len() - offset));
        }
        
        let opt_val = bytes[offset..offset + length].to_vec();
        offset += length;
        
        // Parse option value as string or integer based on option type
        let opt_name = option_name(current_option_number);
        let value_str = match current_option_number {
            3 | 8 | 11 | 15 | 20 | 35 | 39 => {
                // String formats: Uri-Host, Location-Path, Uri-Path, Uri-Query, Location-Query, Proxy-Uri, Proxy-Scheme
                String::from_utf8(opt_val.clone()).unwrap_or_else(|_| {
                    opt_val.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ")
                })
            }
            7 | 12 | 14 | 17 | 60 => {
                // Uint formats: Uri-Port, Content-Format, Max-Age, Accept, Size1
                let mut val = 0u32;
                for &b in &opt_val {
                    val = (val << 8) | (b as u32);
                }
                val.to_string()
            }
            _ => {
                // Opaque binary formats or unknown options
                opt_val.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ")
            }
        };
        
        options.push(CoapOption {
            number: current_option_number,
            name: opt_name.to_string(),
            value: opt_val,
            value_str,
        });
    }
    
    let payload = if offset < bytes.len() {
        bytes[offset..].to_vec()
    } else {
        Vec::new()
    };
    
    Ok(CoapDetails {
        version,
        mtype,
        tkl,
        code,
        message_id,
        token,
        options,
        payload,
    })
}

pub fn build_coap_packet(
    version: u8,
    mtype: u8,
    code: u8,
    message_id: u16,
    token: &[u8],
    options: &mut [(u16, Vec<u8>)],
    payload: &[u8],
) -> Result<Vec<u8>, String> {
    if version > 3 {
        return Err("CoAP version must be 0-3".to_string());
    }
    if mtype > 3 {
        return Err("CoAP Message Type must be 0-3 (CON, NON, ACK, RST)".to_string());
    }
    if token.len() > 8 {
        return Err("CoAP Token length must be 0-8 bytes".to_string());
    }
    
    let mut bytes = Vec::new();
    
    // Header
    bytes.push((version << 6) | (mtype << 4) | (token.len() as u8 & 0x0F));
    bytes.push(code);
    bytes.extend_from_slice(&message_id.to_be_bytes());
    
    // Token
    bytes.extend_from_slice(token);
    
    // Options (Must be sorted by option number)
    options.sort_by_key(|opt| opt.0);
    
    let mut prev_option_number = 0u16;
    for (opt_num, opt_val) in options {
        if *opt_num < prev_option_number {
            return Err("Options logic error (unsorted options)".to_string());
        }
        
        let delta = *opt_num - prev_option_number;
        let length = opt_val.len();
        
        let delta_nibble = if delta < 13 {
            delta as u8
        } else if delta < 269 {
            13
        } else {
            14
        };
        
        let length_nibble = if length < 13 {
            length as u8
        } else if length < 269 {
            13
        } else {
            14
        };
        
        bytes.push((delta_nibble << 4) | length_nibble);
        
        if delta_nibble == 13 {
            bytes.push((delta - 13) as u8);
        } else if delta_nibble == 14 {
            bytes.extend_from_slice(&(delta - 269).to_be_bytes());
        }
        
        if length_nibble == 13 {
            bytes.push((length - 13) as u8);
        } else if length_nibble == 14 {
            bytes.extend_from_slice(&((length - 269) as u16).to_be_bytes());
        }
        
        bytes.extend_from_slice(opt_val);
        prev_option_number = *opt_num;
    }
    
    // Payload
    if !payload.is_empty() {
        bytes.push(0xFF); // Payload Marker
        bytes.extend_from_slice(payload);
    }
    
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_parse_simple_coap() {
        // CON GET (code 0.01), Message ID 0x4321, Token: [0xDE, 0xAD]
        let token = vec![0xDE, 0xAD];
        let mut options = vec![
            (11, b"sensors".to_vec()), // Uri-Path: sensors
            (11, b"temp".to_vec()),    // Uri-Path: temp
        ];
        let payload = b"22.5 C".to_vec();
        
        let packet = build_coap_packet(1, 0, 1, 0x4321, &token, &mut options, &payload).unwrap();
        
        let parsed = parse_coap(&packet).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.mtype, 0); // CON
        assert_eq!(parsed.code, 1); // GET
        assert_eq!(parsed.message_id, 0x4321);
        assert_eq!(parsed.token, token);
        
        assert_eq!(parsed.options.len(), 2);
        assert_eq!(parsed.options[0].number, 11);
        assert_eq!(parsed.options[0].name, "Uri-Path");
        assert_eq!(parsed.options[0].value_str, "sensors");
        
        assert_eq!(parsed.options[1].number, 11);
        assert_eq!(parsed.options[1].value_str, "temp");
        
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_parse_response_with_extended_option() {
        // ACK Content (2.05 = 69), Message ID 0x4321, Token: [0xDE, 0xAD]
        // Content-Format option (12 = 0x0C), value 50 (application/json)
        // Max-Age option (14 = 0x0E), value 60
        let token = vec![0xDE, 0xAD];
        let mut options = vec![
            (12, vec![50]), // Content-Format: application/json
            (14, vec![60]), // Max-Age: 60
        ];
        
        let packet = build_coap_packet(1, 2, 69, 0x4321, &token, &mut options, &[]).unwrap();
        
        let parsed = parse_coap(&packet).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.mtype, 2); // ACK
        assert_eq!(parsed.code, 69); // Content
        assert_eq!(parsed.options.len(), 2);
        assert_eq!(parsed.options[0].number, 12);
        assert_eq!(parsed.options[0].value_str, "50");
        assert_eq!(parsed.options[1].number, 14);
        assert_eq!(parsed.options[1].value_str, "60");
    }
}
