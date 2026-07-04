use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQuestion {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecord {
    pub name: String,
    pub rtype: u16,
    pub rclass: u16,
    pub ttl: u32,
    pub data: Vec<u8>,
    pub parsed_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsDetails {
    pub transaction_id: u16,
    pub flags: u16,
    pub qr: bool,         // false: Query, true: Response
    pub opcode: u8,       // 4 bits
    pub aa: bool,         // Authoritative Answer
    pub tc: bool,         // Truncated
    pub rd: bool,         // Recursion Desired
    pub ra: bool,         // Recursion Available
    pub rcode: u8,        // Response Code (4 bits)
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsRecord>,
    pub authorities: Vec<DnsRecord>,
    pub additionals: Vec<DnsRecord>,
}

pub fn qtype_name(qtype: u16) -> &'static str {
    match qtype {
        1 => "A",
        2 => "NS",
        5 => "CNAME",
        6 => "SOA",
        12 => "PTR",
        15 => "MX",
        16 => "TXT",
        28 => "AAAA",
        33 => "SRV",
        255 => "ANY",
        _ => "UNKNOWN",
    }
}

pub fn qclass_name(qclass: u16) -> String {
    let base_class = qclass & 0x7FFF;
    let unicast_response = (qclass & 0x8000) != 0;
    
    let class_str = match base_class {
        1 => "IN",
        3 => "CH",
        4 => "HS",
        255 => "ANY",
        _ => "UNKNOWN",
    };
    
    if unicast_response {
        format!("{} (unicast-response)", class_str)
    } else {
        class_str.to_string()
    }
}

// Internal helper to read a domain name with compression support
fn parse_domain_name(bytes: &[u8], start_offset: &mut usize) -> Result<String, String> {
    let mut offset = *start_offset;
    let mut name = String::new();
    let mut jumped = false;
    let mut jump_offset = 0;
    let mut loop_count = 0;
    
    let limit = bytes.len();
    
    loop {
        if loop_count > 20 {
            return Err("Too many pointer jumps in domain name (potential loop)".to_string());
        }
        
        if offset >= limit {
            return Err("Unexpected end of DNS packet during domain name parsing".to_string());
        }
        
        let len = bytes[offset];
        
        // Check for compression (highest two bits are 11)
        if (len & 0xC0) == 0xC0 {
            if offset + 1 >= limit {
                return Err("Truncated compression pointer".to_string());
            }
            
            let pointer = (((len & 0x3F) as usize) << 8) | (bytes[offset + 1] as usize);
            if pointer >= limit {
                return Err(format!("Compression pointer points out of bounds: {}", pointer));
            }
            
            if !jumped {
                jump_offset = offset + 2;
                jumped = true;
            }
            
            offset = pointer;
            loop_count += 1;
            continue;
        }
        
        offset += 1;
        if len == 0 {
            break;
        }
        
        if offset + (len as usize) > limit {
            return Err("Truncated label in domain name".to_string());
        }
        
        let label = String::from_utf8_lossy(&bytes[offset..offset + (len as usize)]);
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&label);
        
        offset += len as usize;
    }
    
    if jumped {
        *start_offset = jump_offset;
    } else {
        *start_offset = offset;
    }
    
    if name.is_empty() {
        Ok(".".to_string())
    } else {
        Ok(name)
    }
}

// Internal helper to parse resource record
fn parse_resource_record(bytes: &[u8], offset: &mut usize) -> Result<DnsRecord, String> {
    let name = parse_domain_name(bytes, offset)?;
    
    if *offset + 10 > bytes.len() {
        return Err("Truncated resource record header".to_string());
    }
    
    let rtype = u16::from_be_bytes([bytes[*offset], bytes[*offset + 1]]);
    let rclass = u16::from_be_bytes([bytes[*offset + 2], bytes[*offset + 3]]);
    let ttl = u32::from_be_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    let rdlength = u16::from_be_bytes([bytes[*offset + 8], bytes[*offset + 9]]) as usize;
    *offset += 10;
    
    if *offset + rdlength > bytes.len() {
        return Err(format!("Truncated resource record data (expected {} bytes)", rdlength));
    }
    
    let data = bytes[*offset..*offset + rdlength].to_vec();
    
    // Parse data based on TYPE
    let parsed_data = match rtype {
        1 => { // A
            if rdlength == 4 {
                let addr = Ipv4Addr::new(data[0], data[1], data[2], data[3]);
                addr.to_string()
            } else {
                format!("Invalid A record length: {}", rdlength)
            }
        }
        28 => { // AAAA
            if rdlength == 16 {
                let mut ipv6_bytes = [0u8; 16];
                ipv6_bytes.copy_from_slice(&data[..16]);
                let addr = Ipv6Addr::from(ipv6_bytes);
                addr.to_string()
            } else {
                format!("Invalid AAAA record length: {}", rdlength)
            }
        }
        2 | 5 | 12 => { // NS, CNAME, PTR
            let mut record_offset = *offset;
            parse_domain_name(bytes, &mut record_offset)
                .unwrap_or_else(|e| format!("Failed to parse name: {}", e))
        }
        16 => { // TXT
            let mut txt_parts = Vec::new();
            let mut txt_offset = 0;
            while txt_offset < rdlength {
                let chunk_len = data[txt_offset] as usize;
                if txt_offset + 1 + chunk_len > rdlength {
                    txt_parts.push(String::from_utf8_lossy(&data[txt_offset + 1..]).into_owned());
                    break;
                }
                txt_parts.push(String::from_utf8_lossy(&data[txt_offset + 1..txt_offset + 1 + chunk_len]).into_owned());
                txt_offset += 1 + chunk_len;
            }
            txt_parts.join(", ")
        }
        33 => { // SRV
            if rdlength >= 6 {
                let priority = u16::from_be_bytes([data[0], data[1]]);
                let weight = u16::from_be_bytes([data[2], data[3]]);
                let port = u16::from_be_bytes([data[4], data[5]]);
                let mut target_offset = *offset + 6;
                let target = parse_domain_name(bytes, &mut target_offset)
                    .unwrap_or_else(|e| format!("Failed to parse target: {}", e));
                format!("Priority: {}, Weight: {}, Port: {}, Target: {}", priority, weight, port, target)
            } else {
                "Invalid SRV record length".to_string()
            }
        }
        _ => {
            // General hex string representation
            data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ")
        }
    };
    
    *offset += rdlength;
    
    Ok(DnsRecord {
        name,
        rtype,
        rclass,
        ttl,
        data,
        parsed_data,
    })
}

pub fn parse_dns(bytes: &[u8]) -> Result<DnsDetails, String> {
    if bytes.len() < 12 {
        return Err("DNS packet too short (less than 12 bytes header)".to_string());
    }
    
    let transaction_id = u16::from_be_bytes([bytes[0], bytes[1]]);
    let flags = u16::from_be_bytes([bytes[2], bytes[3]]);
    
    let qr = (flags & 0x8000) != 0;
    let opcode = ((flags & 0x7800) >> 11) as u8;
    let aa = (flags & 0x0400) != 0;
    let tc = (flags & 0x0200) != 0;
    let rd = (flags & 0x0100) != 0;
    let ra = (flags & 0x0080) != 0;
    let rcode = (flags & 0x000F) as u8;
    
    let qdcount = u16::from_be_bytes([bytes[4], bytes[5]]);
    let ancount = u16::from_be_bytes([bytes[6], bytes[7]]);
    let nscount = u16::from_be_bytes([bytes[8], bytes[9]]);
    let arcount = u16::from_be_bytes([bytes[10], bytes[11]]);
    
    let mut offset = 12;
    
    let mut questions = Vec::new();
    for _ in 0..qdcount {
        let name = parse_domain_name(bytes, &mut offset)?;
        if offset + 4 > bytes.len() {
            return Err("Truncated question section".to_string());
        }
        let qtype = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
        let qclass = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]);
        offset += 4;
        questions.push(DnsQuestion { name, qtype, qclass });
    }
    
    let mut answers = Vec::new();
    for _ in 0..ancount {
        answers.push(parse_resource_record(bytes, &mut offset)?);
    }
    
    let mut authorities = Vec::new();
    for _ in 0..nscount {
        authorities.push(parse_resource_record(bytes, &mut offset)?);
    }
    
    let mut additionals = Vec::new();
    for _ in 0..arcount {
        additionals.push(parse_resource_record(bytes, &mut offset)?);
    }
    
    Ok(DnsDetails {
        transaction_id,
        flags,
        qr,
        opcode,
        aa,
        tc,
        rd,
        ra,
        rcode,
        qdcount,
        ancount,
        nscount,
        arcount,
        questions,
        answers,
        authorities,
        additionals,
    })
}

// Convert normal domain representation e.g. "google.com" into DNS wire format e.g. "\x06google\x03com\x00"
pub fn encode_domain_name(name: &str) -> Result<Vec<u8>, String> {
    if name.is_empty() || name == "." {
        return Ok(vec![0]);
    }
    
    let mut bytes = Vec::new();
    for label in name.split('.') {
        if label.is_empty() {
            return Err("Domain name contains empty label".to_string());
        }
        if label.len() > 63 {
            return Err("Label too long (maximum 63 characters)".to_string());
        }
        bytes.push(label.len() as u8);
        bytes.extend_from_slice(label.as_bytes());
    }
    bytes.push(0); // Terminating null byte
    
    if bytes.len() > 255 {
        return Err("Domain name too long (maximum 255 bytes)".to_string());
    }
    
    Ok(bytes)
}

pub fn build_dns_query(
    transaction_id: u16,
    flags: u16,
    qname: &str,
    qtype: u16,
    qclass: u16,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    
    // Header
    bytes.extend_from_slice(&transaction_id.to_be_bytes());
    bytes.extend_from_slice(&flags.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT = 1
    bytes.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT = 0
    bytes.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT = 0
    bytes.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT = 0
    
    // Question Section
    let qname_bytes = encode_domain_name(qname)?;
    bytes.extend_from_slice(&qname_bytes);
    bytes.extend_from_slice(&qtype.to_be_bytes());
    bytes.extend_from_slice(&qclass.to_be_bytes());
    
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_domain_name() {
        let raw = encode_domain_name("google.com").unwrap();
        assert_eq!(raw, vec![6, b'g', b'o', b'o', b'g', b'l', b'e', 3, b'c', b'o', b'm', 0]);
        
        let mut offset = 0;
        let decoded = parse_domain_name(&raw, &mut offset).unwrap();
        assert_eq!(decoded, "google.com");
        assert_eq!(offset, raw.len());
    }

    #[test]
    fn test_build_and_parse_dns_query() {
        // Standard query, recursion desired: transaction ID 0x1234, IN class (0x0001), A record (0x0001)
        let query = build_dns_query(0x1234, 0x0100, "test.local", 1, 1).unwrap();
        
        let parsed = parse_dns(&query).unwrap();
        assert_eq!(parsed.transaction_id, 0x1234);
        assert_eq!(parsed.flags, 0x0100);
        assert_eq!(parsed.qr, false);
        assert_eq!(parsed.rd, true);
        assert_eq!(parsed.qdcount, 1);
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].name, "test.local");
        assert_eq!(parsed.questions[0].qtype, 1); // A
        assert_eq!(parsed.questions[0].qclass, 1); // IN
    }

    #[test]
    fn test_parse_dns_response_with_compression() {
        // Build mock response using manual byte manipulation or decompression verification
        // ID: 0x1234, Flags: 0x8180 (Response, recursion desired, recursion available), QDCOUNT: 1, ANCOUNT: 1
        let mut resp = vec![
            0x12, 0x34, // ID
            0x81, 0x80, // Flags
            0x00, 0x01, // QDCOUNT
            0x00, 0x01, // ANCOUNT
            0x00, 0x00, // NSCOUNT
            0x00, 0x00, // ARCOUNT
        ];
        
        // Question: test.local, Type A, Class IN
        resp.extend_from_slice(&[4, b't', b'e', b's', b't', 5, b'l', b'o', b'c', b'a', b'l', 0]);
        resp.extend_from_slice(&[0x00, 0x01]); // A
        resp.extend_from_slice(&[0x00, 0x01]); // IN
        
        // Answer: Compression pointer to Question Name (offset 12 = 0x000C) -> 0xC00C
        resp.extend_from_slice(&[0xC0, 0x0C]);
        resp.extend_from_slice(&[0x00, 0x01]); // Type A
        resp.extend_from_slice(&[0x00, 0x01]); // Class IN
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL 60
        resp.extend_from_slice(&[0x00, 0x04]); // RDLength 4
        resp.extend_from_slice(&[192, 168, 1, 10]); // RData
        
        let parsed = parse_dns(&resp).unwrap();
        assert_eq!(parsed.transaction_id, 0x1234);
        assert_eq!(parsed.ancount, 1);
        assert_eq!(parsed.answers.len(), 1);
        assert_eq!(parsed.answers[0].name, "test.local");
        assert_eq!(parsed.answers[0].parsed_data, "192.168.1.10");
    }
}
