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

use crate::json::Json;
use std::fs;
use std::path::PathBuf;

const EMBEDDED_EN: &str = include_str!("../locales/en.json");
const EMBEDDED_IT: &str = include_str!("../locales/it.json");

pub fn resolve_locale(lang_code: Option<&str>) -> String {
    let raw = match lang_code {
        Some(s) if !s.trim().is_empty() => s.trim().to_ascii_lowercase(),
        _ => return "en".to_string(),
    };
    let short = raw
        .split(|c| c == '-' || c == '_')
        .next()
        .unwrap_or("en")
        .to_string();

    if short == "it" || short == "en" {
        return short;
    }

    // Check if a contributor added `locales/<short>.json`
    let candidate = PathBuf::from("locales").join(format!("{}.json", short));
    if candidate.exists() {
        return short;
    }

    "en".to_string()
}

pub fn t(lang_code: Option<&str>, key_path: &str, params: &[(&str, &str)]) -> String {
    let locale = resolve_locale(lang_code);

    let catalog_str = match locale.as_str() {
        "it" => EMBEDDED_IT.to_string(),
        "en" => EMBEDDED_EN.to_string(),
        other => {
            let p = PathBuf::from("locales").join(format!("{}.json", other));
            fs::read_to_string(&p).unwrap_or_else(|_| EMBEDDED_EN.to_string())
        }
    };

    let mut resolved = lookup_json_path(&catalog_str, key_path)
        .or_else(|| lookup_json_path(EMBEDDED_EN, key_path))
        .unwrap_or_else(|| key_path.to_string());

    for (k, v) in params {
        let placeholder = format!("{{{}}}", k);
        resolved = resolved.replace(&placeholder, v);
    }
    resolved
}

fn lookup_json_path(raw_json: &str, key_path: &str) -> Option<String> {
    let parsed = Json::parse_opt(raw_json)?;
    let mut cur = &parsed;
    for part in key_path.split('.') {
        cur = cur.get(part)?;
    }
    cur.as_str().map(|s| s.to_string())
}
