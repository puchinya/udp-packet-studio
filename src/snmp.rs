#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpValue {
    Integer(i64),
    OctetString(Vec<u8>),
    ObjectId(String),
    Null,
    IpAddress(std::net::Ipv4Addr),
    Counter(u32),
    Gauge(u32),
    TimeTicks(u32),
    Unsupported(u8, Vec<u8>),
}

impl SnmpValue {
    pub fn to_string_repr(&self) -> String {
        match self {
            SnmpValue::Integer(v) => v.to_string(),
            SnmpValue::OctetString(bytes) => {
                if let Ok(s) = String::from_utf8(bytes.clone()) {
                    if s.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                        return s;
                    }
                }
                bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" ")
            }
            SnmpValue::ObjectId(oid) => oid.clone(),
            SnmpValue::Null => "Null".to_string(),
            SnmpValue::IpAddress(ip) => ip.to_string(),
            SnmpValue::Counter(v) => format!("Counter: {}", v),
            SnmpValue::Gauge(v) => format!("Gauge: {}", v),
            SnmpValue::TimeTicks(v) => format!("TimeTicks: {} ({}s)", v, *v as f64 / 100.0),
            SnmpValue::Unsupported(tag, bytes) => {
                format!("Unsupported(0x{:02X}): {}", tag, bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<String>>().join(" "))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpMessage {
    pub version: u8,
    pub community: String,
    pub pdu_type: u8,
    pub request_id: i32,
    pub error_status: i32,
    pub error_index: i32,
    pub varbinds: Vec<(String, SnmpValue)>,
}

pub fn pdu_type_name(pdu_type: u8) -> &'static str {
    match pdu_type {
        0xa0 => "GetRequest",
        0xa1 => "GetNextRequest",
        0xa2 => "Response",
        0xa3 => "SetRequest",
        0xa4 => "Trap (v1)",
        0xa5 => "GetBulkRequest",
        0xa6 => "InformRequest",
        0xa7 => "Trap (v2)",
        0xa8 => "Report",
        _ => "Unknown PDU",
    }
}

fn decode_vlq(bytes: &[u8], offset: &mut usize) -> Result<u32, String> {
    let mut val: u32 = 0;
    loop {
        if *offset >= bytes.len() {
            return Err("Unexpected EOF decoding VLQ".to_string());
        }
        let b = bytes[*offset];
        *offset += 1;
        val = val.checked_shl(7)
            .ok_or_else(|| "VLQ overflow".to_string())?
            | (b & 0x7F) as u32;
        if (b & 0x80) == 0 {
            break;
        }
    }
    Ok(val)
}

fn encode_vlq(mut val: u32) -> Vec<u8> {
    if val == 0 {
        return vec![0];
    }
    let mut bytes = Vec::new();
    while val > 0 {
        let mut b = (val & 0x7F) as u8;
        val >>= 7;
        if !bytes.is_empty() {
            b |= 0x80;
        }
        bytes.push(b);
    }
    bytes.reverse();
    bytes
}

fn decode_length(bytes: &[u8], offset: &mut usize) -> Result<usize, String> {
    if *offset >= bytes.len() {
        return Err("Unexpected EOF decoding length".to_string());
    }
    let b = bytes[*offset];
    *offset += 1;
    if (b & 0x80) == 0 {
        Ok(b as usize)
    } else {
        let num_octets = (b & 0x7F) as usize;
        if num_octets == 0 || num_octets > 4 {
            return Err(format!("Unsupported indefinite or too long length: {}", num_octets));
        }
        let mut len = 0usize;
        for _ in 0..num_octets {
            if *offset >= bytes.len() {
                return Err("Unexpected EOF decoding multi-byte length".to_string());
            }
            len = (len << 8) | (bytes[*offset] as usize);
            *offset += 1;
        }
        Ok(len)
    }
}

fn encode_length(len: usize) -> Vec<u8> {
    if len < 128 {
        vec![len as u8]
    } else {
        let mut bytes = Vec::new();
        let mut temp = len;
        while temp > 0 {
            bytes.push((temp & 0xFF) as u8);
            temp >>= 8;
        }
        bytes.reverse();
        let mut res = vec![(0x80 | bytes.len()) as u8];
        res.extend(bytes);
        res
    }
}

fn decode_oid(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("Empty OID bytes".to_string());
    }
    let first_byte = bytes[0];
    let first = first_byte / 40;
    let second = first_byte % 40;
    let mut res = format!("{}.{}", first, second);
    
    let mut offset = 1;
    while offset < bytes.len() {
        let val = decode_vlq(bytes, &mut offset)?;
        res.push_str(&format!(".{}", val));
    }
    Ok(res)
}

fn encode_oid(oid_str: &str) -> Result<Vec<u8>, String> {
    let clean: String = oid_str.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() < 2 {
        return Err("OID must have at least 2 components".to_string());
    }
    let first = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
    let second = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
    if first > 2 || (first < 2 && second >= 40) {
        return Err("Invalid OID first/second components".to_string());
    }
    let mut res = vec![(first * 40 + second) as u8];
    for p in &parts[2..] {
        let val = p.parse::<u32>().map_err(|e| e.to_string())?;
        res.extend(encode_vlq(val));
    }
    Ok(res)
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: u8,
    pub value: Vec<u8>,
}

pub fn decode_element(bytes: &[u8], offset: &mut usize) -> Result<Element, String> {
    if *offset >= bytes.len() {
        return Err("Unexpected EOF decoding tag".to_string());
    }
    let tag = bytes[*offset];
    *offset += 1;
    let len = decode_length(bytes, offset)?;
    if *offset + len > bytes.len() {
        return Err(format!("Element value goes out of bounds: tag=0x{:02X}, len={}, remaining={}", tag, len, bytes.len() - *offset));
    }
    let value = bytes[*offset..*offset + len].to_vec();
    *offset += len;
    Ok(Element { tag, value })
}

fn decode_integer(bytes: &[u8]) -> Result<i64, String> {
    if bytes.is_empty() {
        return Err("Empty integer bytes".to_string());
    }
    if bytes.len() > 8 {
        return Err("Integer too large".to_string());
    }
    let mut val = if (bytes[0] & 0x80) != 0 {
        -1i64
    } else {
        0i64
    };
    for &b in bytes {
        val = (val << 8) | (b as i64);
    }
    Ok(val)
}

fn decode_unsigned(bytes: &[u8]) -> Result<u64, String> {
    if bytes.is_empty() {
        return Err("Empty unsigned bytes".to_string());
    }
    if bytes.len() > 9 || (bytes.len() == 9 && bytes[0] != 0) {
        return Err("Unsigned integer too large".to_string());
    }
    let mut val = 0u64;
    for &b in bytes {
        val = (val << 8) | (b as u64);
    }
    Ok(val)
}

fn encode_integer(val: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut temp = val;
    loop {
        bytes.push((temp & 0xFF) as u8);
        temp >>= 8;
        if (temp == 0 && (bytes.last().unwrap() & 0x80) == 0) || (temp == -1 && (bytes.last().unwrap() & 0x80) != 0) {
            break;
        }
    }
    bytes.reverse();
    bytes
}

fn encode_unsigned(val: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut temp = val;
    loop {
        bytes.push((temp & 0xFF) as u8);
        temp >>= 8;
        if temp == 0 {
            break;
        }
    }
    if (bytes.last().unwrap() & 0x80) != 0 {
        bytes.push(0);
    }
    bytes.reverse();
    bytes
}

pub fn parse_snmp(bytes: &[u8]) -> Result<SnmpMessage, String> {
    let mut offset = 0;
    let msg_seq = decode_element(bytes, &mut offset)?;
    if msg_seq.tag != 0x30 {
        return Err(format!("Invalid SNMP message header (expected Sequence 0x30, got 0x{:02X})", msg_seq.tag));
    }
    
    let mut inner_offset = 0;
    let version_elem = decode_element(&msg_seq.value, &mut inner_offset)?;
    if version_elem.tag != 0x02 {
        return Err("Invalid SNMP version tag".to_string());
    }
    if version_elem.value.len() != 1 {
        return Err("Invalid SNMP version length".to_string());
    }
    let version_raw = version_elem.value[0];
    if version_raw != 0 && version_raw != 1 {
        return Err(format!("Unsupported SNMP version: {}", version_raw));
    }
    
    let community_elem = decode_element(&msg_seq.value, &mut inner_offset)?;
    if community_elem.tag != 0x04 {
        return Err("Invalid SNMP community tag".to_string());
    }
    let community = String::from_utf8_lossy(&community_elem.value).into_owned();
    
    let pdu_elem = decode_element(&msg_seq.value, &mut inner_offset)?;
    let pdu_type = pdu_elem.tag;
    
    let mut pdu_offset = 0;
    let req_id_elem = decode_element(&pdu_elem.value, &mut pdu_offset)?;
    if req_id_elem.tag != 0x02 {
        return Err("Invalid SNMP Request ID tag".to_string());
    }
    let request_id = decode_integer(&req_id_elem.value)? as i32;
    
    let err_status_elem = decode_element(&pdu_elem.value, &mut pdu_offset)?;
    if err_status_elem.tag != 0x02 {
        return Err("Invalid SNMP Error Status tag".to_string());
    }
    let error_status = decode_integer(&err_status_elem.value)? as i32;
    
    let err_index_elem = decode_element(&pdu_elem.value, &mut pdu_offset)?;
    if err_index_elem.tag != 0x02 {
        return Err("Invalid SNMP Error Index tag".to_string());
    }
    let error_index = decode_integer(&err_index_elem.value)? as i32;
    
    let varbind_list_seq = decode_element(&pdu_elem.value, &mut pdu_offset)?;
    if varbind_list_seq.tag != 0x30 {
        return Err("Invalid SNMP VarBindList header".to_string());
    }
    
    let mut varbind_offset = 0;
    let mut varbinds = Vec::new();
    while varbind_offset < varbind_list_seq.value.len() {
        let varbind_seq = decode_element(&varbind_list_seq.value, &mut varbind_offset)?;
        if varbind_seq.tag != 0x30 {
            return Err("Invalid SNMP VarBind header".to_string());
        }
        let mut single_var_offset = 0;
        let oid_elem = decode_element(&varbind_seq.value, &mut single_var_offset)?;
        if oid_elem.tag != 0x06 {
            return Err("Invalid SNMP VarBind OID tag".to_string());
        }
        let oid = decode_oid(&oid_elem.value)?;
        
        let val_elem = decode_element(&varbind_seq.value, &mut single_var_offset)?;
        let value = match val_elem.tag {
            0x02 => SnmpValue::Integer(decode_integer(&val_elem.value)?),
            0x04 => SnmpValue::OctetString(val_elem.value),
            0x06 => SnmpValue::ObjectId(decode_oid(&val_elem.value)?),
            0x05 => SnmpValue::Null,
            0x40 => {
                if val_elem.value.len() == 4 {
                    SnmpValue::IpAddress(std::net::Ipv4Addr::new(
                        val_elem.value[0], val_elem.value[1], val_elem.value[2], val_elem.value[3]
                    ))
                } else {
                    SnmpValue::Unsupported(0x40, val_elem.value)
                }
            }
            0x41 => SnmpValue::Counter(decode_unsigned(&val_elem.value)? as u32),
            0x42 => SnmpValue::Gauge(decode_unsigned(&val_elem.value)? as u32),
            0x43 => SnmpValue::TimeTicks(decode_unsigned(&val_elem.value)? as u32),
            other => SnmpValue::Unsupported(other, val_elem.value),
        };
        
        varbinds.push((oid, value));
    }
    
    Ok(SnmpMessage {
        version: version_raw,
        community,
        pdu_type,
        request_id,
        error_status,
        error_index,
        varbinds,
    })
}

pub fn build_snmp(
    version: u8,
    community: &str,
    pdu_type: u8,
    request_id: i32,
    error_status: i32,
    error_index: i32,
    varbinds: &[crate::types::SnmpVarBindState],
) -> Result<Vec<u8>, String> {
    let mut varbind_list_payload = Vec::new();
    for vb in varbinds {
        let mut vb_payload = Vec::new();
        // OID
        let oid_bytes = encode_oid(&vb.oid)?;
        vb_payload.push(0x06);
        vb_payload.extend(encode_length(oid_bytes.len()));
        vb_payload.extend(oid_bytes);
        
        // Value
        let val_bytes = match vb.value_type {
            crate::types::SnmpValueType::Integer => {
                let v = vb.value.trim().parse::<i64>().map_err(|_| "Invalid integer value")?;
                let mut p = Vec::new();
                p.push(0x02);
                let enc = encode_integer(v);
                p.extend(encode_length(enc.len()));
                p.extend(enc);
                p
            }
            crate::types::SnmpValueType::OctetString => {
                let bytes = vb.value.as_bytes().to_vec();
                let mut p = Vec::new();
                p.push(0x04);
                p.extend(encode_length(bytes.len()));
                p.extend(bytes);
                p
            }
            crate::types::SnmpValueType::ObjectId => {
                let enc = encode_oid(&vb.value)?;
                let mut p = Vec::new();
                p.push(0x06);
                p.extend(encode_length(enc.len()));
                p.extend(enc);
                p
            }
            crate::types::SnmpValueType::Null => {
                vec![0x05, 0x00]
            }
            crate::types::SnmpValueType::IpAddress => {
                let ip: std::net::Ipv4Addr = vb.value.trim().parse().map_err(|_| "Invalid IPv4 Address")?;
                let mut p = Vec::new();
                p.push(0x40);
                p.extend(encode_length(4));
                p.extend(&ip.octets());
                p
            }
            crate::types::SnmpValueType::Counter32 => {
                let v = vb.value.trim().parse::<u64>().map_err(|_| "Invalid counter value")?;
                let mut p = Vec::new();
                p.push(0x41);
                let enc = encode_unsigned(v);
                p.extend(encode_length(enc.len()));
                p.extend(enc);
                p
            }
            crate::types::SnmpValueType::Gauge32 => {
                let v = vb.value.trim().parse::<u64>().map_err(|_| "Invalid gauge value")?;
                let mut p = Vec::new();
                p.push(0x42);
                let enc = encode_unsigned(v);
                p.extend(encode_length(enc.len()));
                p.extend(enc);
                p
            }
            crate::types::SnmpValueType::TimeTicks => {
                let v = vb.value.trim().parse::<u64>().map_err(|_| "Invalid TimeTicks value")?;
                let mut p = Vec::new();
                p.push(0x43);
                let enc = encode_unsigned(v);
                p.extend(encode_length(enc.len()));
                p.extend(enc);
                p
            }
        };
        vb_payload.extend(val_bytes);
        
        let mut vb_seq = Vec::new();
        vb_seq.push(0x30);
        vb_seq.extend(encode_length(vb_payload.len()));
        vb_seq.extend(vb_payload);
        
        varbind_list_payload.extend(vb_seq);
    }
    
    let mut varbind_list = Vec::new();
    varbind_list.push(0x30);
    varbind_list.extend(encode_length(varbind_list_payload.len()));
    varbind_list.extend(varbind_list_payload);
    
    let mut pdu_payload = Vec::new();
    pdu_payload.push(0x02);
    let req_enc = encode_integer(request_id as i64);
    pdu_payload.extend(encode_length(req_enc.len()));
    pdu_payload.extend(req_enc);
    
    pdu_payload.push(0x02);
    let err_status_enc = encode_integer(error_status as i64);
    pdu_payload.extend(encode_length(err_status_enc.len()));
    pdu_payload.extend(err_status_enc);
    
    pdu_payload.push(0x02);
    let err_index_enc = encode_integer(error_index as i64);
    pdu_payload.extend(encode_length(err_index_enc.len()));
    pdu_payload.extend(err_index_enc);
    
    pdu_payload.extend(varbind_list);
    
    let mut pdu = Vec::new();
    pdu.push(pdu_type);
    pdu.extend(encode_length(pdu_payload.len()));
    pdu.extend(pdu_payload);
    
    let mut msg_payload = Vec::new();
    msg_payload.push(0x02);
    msg_payload.push(0x01);
    msg_payload.push(version);
    
    msg_payload.push(0x04);
    msg_payload.extend(encode_length(community.len()));
    msg_payload.extend(community.as_bytes());
    
    msg_payload.extend(pdu);
    
    let mut msg = Vec::new();
    msg.push(0x30);
    msg.extend(encode_length(msg_payload.len()));
    msg.extend(msg_payload);
    
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlq() {
        assert_eq!(encode_vlq(0), vec![0]);
        assert_eq!(encode_vlq(127), vec![127]);
        assert_eq!(encode_vlq(128), vec![0x81, 0x00]);
        
        let mut offset = 0;
        assert_eq!(decode_vlq(&[127], &mut offset).unwrap(), 127);
        
        offset = 0;
        assert_eq!(decode_vlq(&[0x81, 0x00], &mut offset).unwrap(), 128);
    }

    #[test]
    fn test_oid() {
        let oid = "1.3.6.1.2.1.1.1.0";
        let enc = encode_oid(oid).unwrap();
        let dec = decode_oid(&enc).unwrap();
        assert_eq!(dec, oid);
    }

    #[test]
    fn test_integer() {
        let enc1 = encode_integer(10);
        assert_eq!(decode_integer(&enc1).unwrap(), 10);

        let enc2 = encode_integer(-10);
        assert_eq!(decode_integer(&enc2).unwrap(), -10);
    }

    #[test]
    fn test_snmp_build_and_parse() {
        let varbinds = vec![
            crate::types::SnmpVarBindState {
                oid: "1.3.6.1.2.1.1.1.0".to_string(),
                value_type: crate::types::SnmpValueType::Null,
                value: "".to_string(),
            }
        ];
        // Build GetRequest (0xa0)
        let bytes = build_snmp(1, "public", 0xa0, 12345, 0, 0, &varbinds).unwrap();
        
        let parsed = parse_snmp(&bytes).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.community, "public");
        assert_eq!(parsed.pdu_type, 0xa0);
        assert_eq!(parsed.request_id, 12345);
        assert_eq!(parsed.error_status, 0);
        assert_eq!(parsed.error_index, 0);
        assert_eq!(parsed.varbinds.len(), 1);
        assert_eq!(parsed.varbinds[0].0, "1.3.6.1.2.1.1.1.0");
        assert_eq!(parsed.varbinds[0].1, SnmpValue::Null);
    }
}
