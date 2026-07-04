use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyslogDetails {
    pub rfc: String,
    pub priority: u8,
    pub facility: u8,
    pub severity: u8,
    pub timestamp: String,
    pub hostname: String,
    pub app_name: String,
    pub proc_id: String,
    pub msg_id: String,
    pub message: String,
}

pub fn parse_syslog(bytes: &[u8]) -> Option<SyslogDetails> {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim();
    if !s.starts_with('<') {
        return None;
    }
    let close_bracket = s.find('>')?;
    let pri_str = &s[1..close_bracket];
    let priority = pri_str.parse::<u8>().ok()?;
    let facility = priority / 8;
    let severity = priority % 8;
    
    let after_pri = &s[close_bracket + 1..];
    
    // Check for RFC 5424 (version 1 is common)
    // Format: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG
    // Check if it starts with "1 " (version 1)
    if after_pri.starts_with("1 ") {
        let parts: Vec<&str> = after_pri.splitn(7, ' ').collect();
        if parts.len() >= 6 {
            let timestamp = parts[1].to_string();
            let hostname = parts[2].to_string();
            let app_name = parts[3].to_string();
            let proc_id = parts[4].to_string();
            let msg_id = parts[5].to_string();
            
            let rest = parts.get(6).cloned().unwrap_or("");
            let (structured_data, message) = if rest.starts_with('[') {
                if let Some(end_sd) = rest.find(']') {
                    let sd = &rest[0..=end_sd];
                    let msg = &rest[end_sd + 1..];
                    (sd.to_string(), msg.trim().to_string())
                } else {
                    ("-".to_string(), rest.to_string())
                }
            } else if rest.starts_with("- ") {
                ("-".to_string(), rest[2..].to_string())
            } else if rest == "-" {
                ("-".to_string(), String::new())
            } else {
                ("-".to_string(), rest.to_string())
            };
            
            let display_message = if structured_data != "-" {
                format!("SD: {} MSG: {}", structured_data, message)
            } else {
                message
            };
            
            return Some(SyslogDetails {
                rfc: "RFC 5424".to_string(),
                priority,
                facility,
                severity,
                timestamp,
                hostname,
                app_name,
                proc_id,
                msg_id,
                message: display_message,
            });
        }
    }
    
    // RFC 3164 (or legacy)
    // Regex for: Mmm dd hh:mm:ss hostname tag: msg
    // Example: Oct 11 22:14:15 mymachine su: 'su root' failed...
    let re = Regex::new(r"^([A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2})\s+([^\s]+)\s+(.*)$").ok()?;
    if let Some(caps) = re.captures(after_pri) {
        let timestamp = caps.get(1)?.as_str().to_string();
        let hostname = caps.get(2)?.as_str().to_string();
        let rest_msg = caps.get(3)?.as_str().to_string();
        
        let mut app_name = "-".to_string();
        let mut message = rest_msg.clone();
        if let Some(colon_idx) = rest_msg.find(':') {
            let tag = &rest_msg[0..colon_idx];
            if !tag.contains(' ') {
                app_name = tag.to_string();
                message = rest_msg[colon_idx + 1..].trim().to_string();
            }
        }
        
        Some(SyslogDetails {
            rfc: "RFC 3164".to_string(),
            priority,
            facility,
            severity,
            timestamp,
            hostname,
            app_name,
            proc_id: "-".to_string(),
            msg_id: "-".to_string(),
            message,
        })
    } else {
        // Fallback to simple split
        let parts: Vec<&str> = after_pri.splitn(3, ' ').collect();
        let timestamp = parts.first().cloned().unwrap_or("-").to_string();
        let hostname = parts.get(1).cloned().unwrap_or("-").to_string();
        let message = parts.get(2).cloned().unwrap_or(after_pri).to_string();
        Some(SyslogDetails {
            rfc: "RFC 3164 (legacy)".to_string(),
            priority,
            facility,
            severity,
            timestamp,
            hostname,
            app_name: "-".to_string(),
            proc_id: "-".to_string(),
            msg_id: "-".to_string(),
            message,
        })
    }
}

pub fn facility_name(facility: u8) -> &'static str {
    match facility {
        0 => "kern",
        1 => "user",
        2 => "mail",
        3 => "daemon",
        4 => "auth",
        5 => "syslog",
        6 => "lpr",
        7 => "news",
        8 => "uucp",
        9 => "cron",
        10 => "authpriv",
        11 => "ftp",
        12 => "ntp",
        13 => "logaudit",
        14 => "logalert",
        15 => "clock",
        16 => "local0",
        17 => "local1",
        18 => "local2",
        19 => "local3",
        20 => "local4",
        21 => "local5",
        22 => "local6",
        23 => "local7",
        _ => "unknown",
    }
}

pub fn severity_name(severity: u8) -> &'static str {
    match severity {
        0 => "Emergency (0)",
        1 => "Alert (1)",
        2 => "Critical (2)",
        3 => "Error (3)",
        4 => "Warning (4)",
        5 => "Notice (5)",
        6 => "Informational (6)",
        7 => "Debug (7)",
        _ => "unknown",
    }
}

pub fn build_syslog_rfc3164(facility: usize, severity: usize, timestamp: &str, hostname: &str, app_name: &str, msg: &str) -> String {
    let pri = facility * 8 + severity;
    let app_part = if app_name.is_empty() || app_name == "-" {
        String::new()
    } else {
        format!("{}: ", app_name)
    };
    format!("<{}>{} {} {}{}", pri, timestamp, hostname, app_part, msg)
}

pub fn build_syslog_rfc5424(facility: usize, severity: usize, timestamp: &str, hostname: &str, app_name: &str, proc_id: &str, msg_id: &str, msg: &str) -> String {
    let pri = facility * 8 + severity;
    let ts = if timestamp.is_empty() { "-" } else { timestamp };
    let host = if hostname.is_empty() { "-" } else { hostname };
    let app = if app_name.is_empty() { "-" } else { app_name };
    let proc = if proc_id.is_empty() { "-" } else { proc_id };
    let msgid = if msg_id.is_empty() { "-" } else { msg_id };
    format!("<{}>1 {} {} {} {} {} - {}", pri, ts, host, app, proc, msgid, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc5424() {
        let raw = b"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - An application event log entry";
        let parsed = parse_syslog(raw).unwrap();
        assert_eq!(parsed.rfc, "RFC 5424");
        assert_eq!(parsed.priority, 165);
        assert_eq!(parsed.facility, 20); // local4
        assert_eq!(parsed.severity, 5); // Notice
        assert_eq!(parsed.timestamp, "2003-10-11T22:14:15.003Z");
        assert_eq!(parsed.hostname, "mymachine.example.com");
        assert_eq!(parsed.app_name, "evntslog");
        assert_eq!(parsed.proc_id, "-");
        assert_eq!(parsed.msg_id, "ID47");
        assert_eq!(parsed.message, "An application event log entry");
    }

    #[test]
    fn test_parse_rfc3164() {
        let raw = b"<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick";
        let parsed = parse_syslog(raw).unwrap();
        assert_eq!(parsed.rfc, "RFC 3164");
        assert_eq!(parsed.priority, 34);
        assert_eq!(parsed.facility, 4); // auth
        assert_eq!(parsed.severity, 2); // Critical
        assert_eq!(parsed.timestamp, "Oct 11 22:14:15");
        assert_eq!(parsed.hostname, "mymachine");
        assert_eq!(parsed.app_name, "su");
        assert_eq!(parsed.message, "'su root' failed for lonvick");
    }

    #[test]
    fn test_build_rfc3164() {
        let built = build_syslog_rfc3164(4, 2, "Oct 11 22:14:15", "mymachine", "su", "'su root' failed for lonvick");
        assert_eq!(built, "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick");
    }

    #[test]
    fn test_build_rfc5424() {
        let built = build_syslog_rfc5424(20, 5, "2003-10-11T22:14:15.003Z", "mymachine.example.com", "evntslog", "-", "ID47", "An application event log entry");
        assert_eq!(built, "<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 - An application event log entry");
    }
}
