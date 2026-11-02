/*
 * Personal and Educational Free License (PEF License / PEFL)
 * Version 1.0 - 2026-05-26
 * Copyright (c) 2026 Rubens Rainelli
 *
 * Contact Details:
 *   Email: rubens@rainelli.it
 *   GitHub: https://github.com/RubensRainelli/
 *   Tel: (+39) 342 811 5882
 *
 * Personal and educational free use permitted.
 * Corporate, business or enterprise use prohibited without written authorization and fee.
 * The full license can be found in the LICENSE file.
 */

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

#[allow(dead_code)]
impl Json {
    pub fn obj(entries: Vec<(&str, Json)>) -> Json {
        let mut map = BTreeMap::new();
        for (k, v) in entries {
            map.insert(k.to_string(), v);
        }
        Json::Object(map)
    }

    pub fn str<S: Into<String>>(s: S) -> Json {
        Json::Str(s.into())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_num(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(Json::Number(n)) => Some(*n),
            _ => None,
        }
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_arr(&self, key: &str) -> Option<&Vec<Json>> {
        self.get(key).and_then(|v| v.as_array())
    }

    pub fn stringify(&self) -> String {
        self.to_json_string()
    }

    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write_to(&mut out);
        out
    }

    fn write_to(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            Json::Str(s) => {
                out.push('"');
                for ch in s.chars() {
                    match ch {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) < 0x20 => {
                            out.push_str(&format!("\\u{:04x}", c as u32));
                        }
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            Json::Array(arr) => {
                out.push('[');
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_to(out);
                }
                out.push(']');
            }
            Json::Object(map) => {
                out.push('{');
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    Json::Str(k.clone()).write_to(out);
                    out.push(':');
                    v.write_to(out);
                }
                out.push('}');
            }
        }
    }

    pub fn parse(input: &str) -> Result<Json, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut pos = 0;
        let val = parse_value(&chars, &mut pos)?;
        Ok(val)
    }

    pub fn parse_opt(input: &str) -> Option<Json> {
        Self::parse(input).ok()
    }
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    skip_ws(chars, pos);
    if *pos >= chars.len() {
        return Err("Unexpected EOF".into());
    }
    match chars[*pos] {
        '"' => parse_string(chars, pos).map(Json::Str),
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        't' | 'f' => parse_bool(chars, pos),
        'n' => parse_null(chars, pos),
        '-' | '0'..='9' => parse_number(chars, pos),
        c => Err(format!("Unexpected char '{}' at {}", c, *pos)),
    }
}

fn parse_string(chars: &[char], pos: &mut usize) -> Result<String, String> {
    *pos += 1;
    let mut s = String::new();
    while *pos < chars.len() {
        let ch = chars[*pos];
        *pos += 1;
        if ch == '"' {
            return Ok(s);
        } else if ch == '\\' {
            if *pos >= chars.len() {
                return Err("Unterminated escape".into());
            }
            let esc = chars[*pos];
            *pos += 1;
            match esc {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'b' => s.push('\u{0008}'),
                'f' => s.push('\u{000c}'),
                'n' => s.push('\n'),
                'r' => s.push('\r'),
                't' => s.push('\t'),
                'u' => {
                    if *pos + 4 <= chars.len() {
                        let hex: String = chars[*pos..*pos + 4].iter().collect();
                        *pos += 4;
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(code) {
                                s.push(c);
                            }
                        }
                    }
                }
                other => s.push(other),
            }
        } else {
            s.push(ch);
        }
    }
    Err("Unterminated string".into())
}

fn parse_object(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    *pos += 1;
    let mut map = BTreeMap::new();
    skip_ws(chars, pos);
    if *pos < chars.len() && chars[*pos] == '}' {
        *pos += 1;
        return Ok(Json::Object(map));
    }
    loop {
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '"' {
            return Err("Expected string key".into());
        }
        let key = parse_string(chars, pos)?;
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != ':' {
            return Err("Expected ':'".into());
        }
        *pos += 1;
        let val = parse_value(chars, pos)?;
        map.insert(key, val);
        skip_ws(chars, pos);
        if *pos < chars.len() && chars[*pos] == ',' {
            *pos += 1;
            skip_ws(chars, pos);
            if *pos < chars.len() && chars[*pos] == '}' {
                *pos += 1;
                break;
            }
        } else if *pos < chars.len() && chars[*pos] == '}' {
            *pos += 1;
            break;
        } else {
            return Err("Expected ',' or '}'".into());
        }
    }
    Ok(Json::Object(map))
}

fn parse_array(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    *pos += 1;
    let mut arr = Vec::new();
    skip_ws(chars, pos);
    if *pos < chars.len() && chars[*pos] == ']' {
        *pos += 1;
        return Ok(Json::Array(arr));
    }
    loop {
        let val = parse_value(chars, pos)?;
        arr.push(val);
        skip_ws(chars, pos);
        if *pos < chars.len() && chars[*pos] == ',' {
            *pos += 1;
            skip_ws(chars, pos);
            if *pos < chars.len() && chars[*pos] == ']' {
                *pos += 1;
                break;
            }
        } else if *pos < chars.len() && chars[*pos] == ']' {
            *pos += 1;
            break;
        } else {
            return Err("Expected ',' or ']'".into());
        }
    }
    Ok(Json::Array(arr))
}

fn parse_bool(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    if chars[*pos..].starts_with(&['t', 'r', 'u', 'e']) {
        *pos += 4;
        Ok(Json::Bool(true))
    } else if chars[*pos..].starts_with(&['f', 'a', 'l', 's', 'e']) {
        *pos += 5;
        Ok(Json::Bool(false))
    } else {
        Err("Invalid boolean".into())
    }
}

fn parse_null(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    if chars[*pos..].starts_with(&['n', 'u', 'l', 'l']) {
        *pos += 4;
        Ok(Json::Null)
    } else {
        Err("Invalid null".into())
    }
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<Json, String> {
    let start = *pos;
    while *pos < chars.len()
        && (chars[*pos].is_ascii_digit()
            || chars[*pos] == '.'
            || chars[*pos] == '-'
            || chars[*pos] == '+'
            || chars[*pos] == 'e'
            || chars[*pos] == 'E')
    {
        *pos += 1;
    }
    let s: String = chars[start..*pos].iter().collect();
    s.parse::<f64>()
        .map(Json::Number)
        .map_err(|_| format!("Invalid number '{}'", s))
}
