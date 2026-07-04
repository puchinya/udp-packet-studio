use crate::types::LogEntry;
use regex::RegexBuilder;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    And,
    Or,
    LParen,
    RParen,
    Field(String),               // port, srcport, dstport, ip, payload
    Slice(usize, Option<usize>), // [start:length]
    OpEqual,                     // ==
    OpMatches,                   // matches
    Value(String),               // hex value, IP pattern, number, or regex pattern (without quotes)
}

#[derive(Debug, Clone)]
pub enum FilterOp {
    Port(u16),
    SrcPort(u16),
    DstPort(u16),
    Ip(String), // IP pattern with wildcard *
    Payload(Vec<u8>),
    PayloadSlice {
        start: usize,
        length: Option<usize>,
        expected: Vec<u8>,
    },
    PayloadMatches(String), // regex pattern
}

#[derive(Debug, Clone)]
pub enum FilterExpr {
    Op(FilterOp),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
}

/// Tokenize the input string.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        if c == '(' {
            tokens.push(Token::LParen);
            chars.next();
            continue;
        }
        if c == ')' {
            tokens.push(Token::RParen);
            chars.next();
            continue;
        }

        if c == '=' {
            chars.next();
            if chars.peek() == Some(&'=') {
                chars.next();
            }
            tokens.push(Token::OpEqual);
            continue;
        }

        if c.is_alphanumeric() || c == '[' || c == '"' || c == '*' || c == '.' || c == ':' || c == '-' {
            if c == '"' {
                chars.next(); // Skip opening quote
                let mut val = String::new();
                let mut escaped = false;
                let mut closed = false;
                while let Some(nc) = chars.next() {
                    if escaped {
                        val.push(nc);
                        escaped = false;
                    } else if nc == '\\' {
                        escaped = true;
                    } else if nc == '"' {
                        closed = true;
                        break;
                    } else {
                        val.push(nc);
                    }
                }
                if !closed {
                    return Err("Unclosed double quote".to_string());
                }
                tokens.push(Token::Value(val));
                continue;
            }

            let mut word = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_alphanumeric() || nc == '_' || nc == '.' || nc == '*' || nc == ':' || nc == '-' {
                    word.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }

            let lower_word = word.to_lowercase();
            if lower_word == "and" || word == "&&" {
                tokens.push(Token::And);
            } else if lower_word == "or" || word == "||" {
                tokens.push(Token::Or);
            } else if lower_word == "matches" {
                tokens.push(Token::OpMatches);
            } else if lower_word == "port" || lower_word == "srcport" || lower_word == "dstport" || lower_word == "ip" || lower_word == "payload" {
                tokens.push(Token::Field(lower_word));
                if chars.peek() == Some(&'[') {
                    chars.next(); // Skip '['
                    let mut slice_str = String::new();
                    let mut closed = false;
                    while let Some(nc) = chars.next() {
                        if nc == ']' {
                            closed = true;
                            break;
                        }
                        slice_str.push(nc);
                    }
                    if !closed {
                        return Err("Unclosed slice bracket '['".to_string());
                    }
                    let parts: Vec<&str> = slice_str.split(':').collect();
                    if parts.is_empty() || parts.len() > 2 {
                        return Err(format!("Invalid slice syntax: [{}]", slice_str));
                    }
                    let start = parts[0].trim().parse::<usize>()
                        .map_err(|_| format!("Invalid slice start index: {}", parts[0]))?;
                    let length = if parts.len() == 2 {
                        let len_str = parts[1].trim();
                        if len_str.is_empty() {
                            None
                        } else {
                            let len_val = len_str.parse::<usize>()
                                .map_err(|_| format!("Invalid slice length: {}", len_str))?;
                            Some(len_val)
                        }
                    } else {
                        Some(1)
                    };
                    tokens.push(Token::Slice(start, length));
                }
            } else {
                tokens.push(Token::Value(word));
            }
            continue;
        }

        return Err(format!("Unexpected character: '{}'", c));
    }

    Ok(tokens)
}

/// Helper to parse a hex string into bytes.
fn parse_hex_data(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();

    if clean.len() % 2 != 0 {
        return Err("Hex string must have an even number of digits".to_string());
    }

    let mut bytes = Vec::new();
    for i in (0..clean.len()).step_by(2) {
        let b = u8::from_str_radix(&clean[i..i+2], 16)
            .map_err(|_| "Invalid hex character".to_string())?;
        bytes.push(b);
    }
    Ok(bytes)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn parse(&mut self) -> Result<FilterExpr, String> {
        let expr = self.parse_or()?;
        if let Some(tok) = self.peek() {
            return Err(format!("Unexpected token at end of expression: {:?}", tok));
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<FilterExpr, String> {
        let mut expr = self.parse_and()?;
        while let Some(Token::Or) = self.peek() {
            self.next();
            let right = self.parse_and()?;
            expr = FilterExpr::Or(Box::new(expr), Box::new(right));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, String> {
        let mut expr = self.parse_primary()?;
        while let Some(Token::And) = self.peek() {
            self.next();
            let right = self.parse_primary()?;
            expr = FilterExpr::And(Box::new(expr), Box::new(right));
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<FilterExpr, String> {
        match self.peek() {
            Some(Token::LParen) => {
                self.next();
                let expr = self.parse_or()?;
                if self.next() != Some(Token::RParen) {
                    return Err("Expected ')'".to_string());
                }
                Ok(expr)
            }
            _ => {
                let comp = self.parse_comparison()?;
                Ok(FilterExpr::Op(comp))
            }
        }
    }

    fn parse_comparison(&mut self) -> Result<FilterOp, String> {
        let field_tok = self.next();
        let field = match field_tok {
            Some(Token::Field(f)) => f,
            other => return Err(format!("Expected field (port, ip, payload etc.), found {:?}", other)),
        };

        let slice = match self.peek() {
            Some(Token::Slice(start, len)) => {
                let s = (*start, *len);
                self.next();
                Some(s)
            }
            _ => None,
        };

        let op_tok = self.next();
        let val_tok = self.next();

        let val = match val_tok {
            Some(Token::Value(v)) => v,
            other => return Err(format!("Expected value, found {:?}", other)),
        };

        match field.as_str() {
            "port" | "srcport" | "dstport" => {
                if slice.is_some() {
                    return Err(format!("Field '{}' does not support slicing", field));
                }
                if op_tok != Some(Token::OpEqual) {
                    return Err(format!("Operator for '{}' must be '=='", field));
                }
                let port_val = val.parse::<u16>().map_err(|_| format!("Invalid port number: {}", val))?;
                match field.as_str() {
                    "port" => Ok(FilterOp::Port(port_val)),
                    "srcport" => Ok(FilterOp::SrcPort(port_val)),
                    "dstport" => Ok(FilterOp::DstPort(port_val)),
                    _ => unreachable!(),
                }
            }
            "ip" => {
                if slice.is_some() {
                    return Err("Field 'ip' does not support slicing".to_string());
                }
                if op_tok != Some(Token::OpEqual) {
                    return Err("Operator for 'ip' must be '=='".to_string());
                }
                Ok(FilterOp::Ip(val))
            }
            "payload" => {
                if let Some((start, length)) = slice {
                    if op_tok != Some(Token::OpEqual) {
                        return Err("Operator for sliced 'payload' must be '=='".to_string());
                    }
                    let expected = parse_hex_data(&val)?;
                    Ok(FilterOp::PayloadSlice { start, length, expected })
                } else {
                    match op_tok {
                        Some(Token::OpEqual) => {
                            let expected = parse_hex_data(&val)?;
                            Ok(FilterOp::Payload(expected))
                        }
                        Some(Token::OpMatches) => {
                            // Validate regex compiles (case-insensitive flag enabled)
                            if let Err(e) = RegexBuilder::new(&val).case_insensitive(true).build() {
                                return Err(format!("Invalid regular expression: {}", e));
                            }
                            Ok(FilterOp::PayloadMatches(val))
                        }
                        other => return Err(format!("Invalid operator for 'payload': {:?}", other)),
                    }
                }
            }
            _ => Err(format!("Unknown field: {}", field)),
        }
    }
}

/// Parses the complete filter expression.
pub fn parse_filter(input: &str) -> Result<FilterExpr, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Empty filter expression".to_string());
    }

    if let Some(first_char) = trimmed.chars().next() {
        if first_char.is_ascii_digit() {
            // Implicit IP filter if expression starts with a digit
            return Ok(FilterExpr::Op(FilterOp::Ip(trimmed.to_string())));
        }
    }

    let tokens = tokenize(trimmed)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn match_ip_pattern(ip: &str, pattern: &str) -> bool {
    let mut re_pattern = String::from("^");
    for c in pattern.chars() {
        match c {
            '*' => re_pattern.push_str(".*"),
            '.' => re_pattern.push_str(r"\."),
            _ => re_pattern.push(c),
        }
    }
    re_pattern.push('$');

    if let Ok(re) = RegexBuilder::new(&re_pattern).case_insensitive(true).build() {
        re.is_match(ip)
    } else {
        ip == pattern
    }
}

impl FilterExpr {
    pub fn eval(&self, entry: &LogEntry) -> bool {
        match self {
            FilterExpr::Op(op) => op.eval(entry),
            FilterExpr::And(left, right) => left.eval(entry) && right.eval(entry),
            FilterExpr::Or(left, right) => left.eval(entry) || right.eval(entry),
        }
    }
}

impl FilterOp {
    pub fn eval(&self, entry: &LogEntry) -> bool {
        match self {
            FilterOp::Port(p) => {
                if let Ok(src) = entry.src_port.parse::<u16>() {
                    if src == *p {
                        return true;
                    }
                }
                if let Ok(dst) = entry.dest_port.parse::<u16>() {
                    if dst == *p {
                        return true;
                    }
                }
                false
            }
            FilterOp::SrcPort(p) => {
                if let Ok(src) = entry.src_port.parse::<u16>() {
                    src == *p
                } else {
                    false
                }
            }
            FilterOp::DstPort(p) => {
                if let Ok(dst) = entry.dest_port.parse::<u16>() {
                    dst == *p
                } else {
                    false
                }
            }
            FilterOp::Ip(pattern) => {
                match_ip_pattern(&entry.src_ip, pattern) || match_ip_pattern(&entry.dest_ip, pattern)
            }
            FilterOp::Payload(expected) => {
                entry.data == *expected
            }
            FilterOp::PayloadSlice { start, length, expected } => {
                let len = entry.data.len();
                if *start >= len {
                    return false;
                }
                let end = match length {
                    Some(l) => std::cmp::min(start + l, len),
                    None => len,
                };
                if start >= &end {
                    return false;
                }
                let slice = &entry.data[*start..end];
                slice == expected
            }
            FilterOp::PayloadMatches(pattern) => {
                if let Ok(re) = RegexBuilder::new(pattern).case_insensitive(true).build() {
                    let s = String::from_utf8_lossy(&entry.data);
                    re.is_match(&s)
                } else {
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LogEntry, LogDirection};
    use std::net::SocketAddr;
    use chrono::Local;

    fn create_test_entry(src_ip: &str, src_port: u16, dest_ip: &str, dest_port: u16, data: Vec<u8>) -> LogEntry {
        let addr = format!("{}:{}", src_ip, src_port).parse::<SocketAddr>().unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 0)));
        let local_addr = format!("{}:{}", dest_ip, dest_port).parse::<SocketAddr>().ok();
        LogEntry::new_with_local(Local::now(), LogDirection::Received, addr, local_addr, data)
    }

    #[test]
    fn test_implicit_ip() {
        let entry1 = create_test_entry("192.168.1.5", 8000, "10.0.0.1", 9000, vec![]);
        let filter = parse_filter("192.168.1.*").unwrap();
        assert!(filter.eval(&entry1));

        let filter_exact = parse_filter("192.168.1.5").unwrap();
        assert!(filter_exact.eval(&entry1));

        let filter_no_match = parse_filter("192.168.2.*").unwrap();
        assert!(!filter_no_match.eval(&entry1));
    }

    #[test]
    fn test_ports() {
        let entry = create_test_entry("192.168.1.5", 8000, "10.0.0.1", 9000, vec![]);
        
        let filter_port = parse_filter("port == 8000").unwrap();
        assert!(filter_port.eval(&entry));

        let filter_port_dst = parse_filter("port == 9000").unwrap();
        assert!(filter_port_dst.eval(&entry));

        let filter_srcport = parse_filter("srcport == 8000").unwrap();
        assert!(filter_srcport.eval(&entry));

        let filter_srcport_fail = parse_filter("srcport == 9000").unwrap();
        assert!(!filter_srcport_fail.eval(&entry));

        let filter_dstport = parse_filter("dstport == 9000").unwrap();
        assert!(filter_dstport.eval(&entry));
    }

    #[test]
    fn test_payload() {
        let entry = create_test_entry("192.168.1.5", 8000, "10.0.0.1", 9000, vec![0x11, 0x22, 0x33, 0x44]);

        let filter_exact1 = parse_filter("payload == 11223344").unwrap();
        assert!(filter_exact1.eval(&entry));

        let filter_exact2 = parse_filter("payload == 11:22:33:44").unwrap();
        assert!(filter_exact2.eval(&entry));

        let filter_slice = parse_filter("payload[1:2] == 2233").unwrap();
        assert!(filter_slice.eval(&entry));

        let filter_slice_oob = parse_filter("payload[10:2] == 22").unwrap();
        assert!(!filter_slice_oob.eval(&entry));
    }

    #[test]
    fn test_regex_matches() {
        let entry = create_test_entry("192.168.1.5", 8000, "10.0.0.1", 9000, b"Hello World!".to_vec());

        let filter_regex = parse_filter("payload matches \"hello\"").unwrap();
        assert!(filter_regex.eval(&entry)); // Case-insensitive should match "Hello"

        let filter_regex_exact = parse_filter("payload matches \"^Hello.*!$\"").unwrap();
        assert!(filter_regex_exact.eval(&entry));
    }

    #[test]
    fn test_logical() {
        let entry = create_test_entry("192.168.1.5", 8000, "10.0.0.1", 9000, vec![0x11]);

        let filter_and = parse_filter("srcport == 8000 and dstport == 9000").unwrap();
        assert!(filter_and.eval(&entry));

        let filter_or = parse_filter("srcport == 9999 or dstport == 9000").unwrap();
        assert!(filter_or.eval(&entry));

        let filter_complex = parse_filter("(srcport == 8000 and dstport == 9999) or payload == 11").unwrap();
        assert!(filter_complex.eval(&entry));
    }
}
