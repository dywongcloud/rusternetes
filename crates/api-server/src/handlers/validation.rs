//! Resource name validation utilities
//!
//! Implements Kubernetes-compatible name validation rules:
//! - DNS subdomain names (RFC 1123): lowercase alphanumeric, '-', '.', max 253 chars
//! - DNS label names: lowercase alphanumeric, '-', max 63 chars

use rusternetes_common::Error;
use std::collections::{HashMap, HashSet};

/// Find duplicate JSON keys at the top level of a JSON object string.
/// Returns the first duplicate key found, or None.
fn find_duplicate_json_key(json_str: &str) -> Option<String> {
    // Simple approach: use serde_json streaming to detect duplicate keys
    // Parse as a generic JSON value and check for duplicate keys at each level
    let trimmed = json_str.trim();
    if !trimmed.starts_with('{') { return None; }

    // Use a simple key extraction — find all "key": patterns at the top level
    let mut seen = HashSet::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut key_start: Option<usize> = None;
    let chars: Vec<char> = trimmed.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if escape { escape = false; i += 1; continue; }
        if ch == '\\' && in_string { escape = true; i += 1; continue; }

        if ch == '"' {
            if !in_string {
                in_string = true;
                if depth == 1 && key_start.is_none() {
                    key_start = Some(i + 1);
                }
            } else {
                in_string = false;
                if depth == 1 {
                    if let Some(start) = key_start {
                        // Check if this is a key (followed by ':')
                        let end = i;
                        let key: String = chars[start..end].iter().collect();
                        // Look ahead for ':'
                        let mut j = i + 1;
                        while j < chars.len() && chars[j].is_whitespace() { j += 1; }
                        if j < chars.len() && chars[j] == ':' {
                            if !seen.insert(key.clone()) {
                                return Some(key);
                            }
                        }
                        key_start = None;
                    }
                }
            }
        } else if !in_string {
            if ch == '{' || ch == '[' { depth += 1; }
            if ch == '}' || ch == ']' { depth -= 1; }
            if ch == ',' && depth == 1 { key_start = None; }
        }
        i += 1;
    }
    None
}

/// Recursively find fields in `original` that are not present in `canonical`.
/// Returns a list of dotted field paths for unknown fields.
fn find_unknown_fields_recursive(
    original: &serde_json::Value,
    canonical: &serde_json::Value,
    prefix: &str,
    unknown: &mut Vec<String>,
) {
    match (original, canonical) {
        (serde_json::Value::Object(orig_map), serde_json::Value::Object(canon_map)) => {
            for (key, orig_val) in orig_map {
                let field_path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                if let Some(canon_val) = canon_map.get(key) {
                    // Recurse into nested objects
                    find_unknown_fields_recursive(orig_val, canon_val, &field_path, unknown);
                } else {
                    unknown.push(field_path);
                }
            }
        }
        (serde_json::Value::Array(orig_arr), serde_json::Value::Array(canon_arr)) => {
            // For arrays, check element-by-element if both have the same length
            for (i, (orig_elem, canon_elem)) in
                orig_arr.iter().zip(canon_arr.iter()).enumerate()
            {
                let field_path = format!("{}[{}]", prefix, i);
                find_unknown_fields_recursive(orig_elem, canon_elem, &field_path, unknown);
            }
        }
        _ => {
            // Scalar values — nothing to check
        }
    }
}

/// When `fieldValidation=Strict` is set, validate that the request body does not
/// contain unknown fields by comparing the original JSON against a re-serialized
/// version of the parsed struct.
pub fn validate_strict_fields(
    params: &HashMap<String, String>,
    original_body: &[u8],
    parsed_resource: &impl serde::Serialize,
) -> Result<(), Error> {
    if params.get("fieldValidation").map(|v| v.as_str()) != Some("Strict") {
        return Ok(());
    }

    // Check for duplicate keys in the JSON body before parsing
    // serde_json silently takes the last value for duplicates, so we must detect manually
    if let Ok(body_str) = std::str::from_utf8(original_body) {
        if let Some(dup_field) = find_duplicate_json_key(body_str) {
            return Err(Error::InvalidResource(format!(
                "strict decoding error: json: duplicate field \"{}\"", dup_field
            )));
        }
    }

    // Parse original as generic JSON
    let original: serde_json::Value = serde_json::from_slice(original_body)
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("duplicate field") {
                if let Some(field) = msg.split('`').nth(1) {
                    return Error::InvalidResource(format!(
                        "strict decoding error: json: unknown field \"{}\"", field
                    ));
                }
            }
            Error::InvalidResource(msg)
        })?;

    // Re-serialize the parsed struct to get canonical JSON
    let canonical = serde_json::to_value(parsed_resource)
        .map_err(|e| Error::Internal(e.to_string()))?;

    // Find unknown fields recursively
    let mut unknown = Vec::new();
    find_unknown_fields_recursive(&original, &canonical, "", &mut unknown);

    if !unknown.is_empty() {
        return Err(Error::InvalidResource(format!(
            "strict decoding error: unknown field \"{}\"",
            unknown[0]
        )));
    }

    Ok(())
}

/// Validate that a resource name is a valid DNS subdomain name (RFC 1123).
///
/// Rules:
/// - Must be non-empty
/// - Must be <= 253 characters
/// - Must consist of lowercase alphanumeric characters, '-' or '.'
/// - Must start and end with an alphanumeric character
///
/// This is the standard validation for most Kubernetes resource names.
pub fn validate_resource_name(name: &str) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::InvalidResource(
            "name must be non-empty".to_string(),
        ));
    }

    if name.len() > 253 {
        return Err(Error::InvalidResource(format!(
            "name '{}' is too long: must be no more than 253 characters",
            name
        )));
    }

    // Check each character
    for (i, c) in name.chars().enumerate() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' && c != '.' {
            return Err(Error::InvalidResource(format!(
                "name '{}' is invalid: a lowercase RFC 1123 subdomain must consist of lower case alphanumeric characters, '-' or '.', and must start and end with an alphanumeric character (e.g. 'example.com', regex used for validation is '[a-z0-9]([-a-z0-9]*[a-z0-9])?(\\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*')",
                name
            )));
        }
    }

    // Must start and end with alphanumeric
    let first = name.chars().next().unwrap();
    let last = name.chars().last().unwrap();

    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(Error::InvalidResource(format!(
            "name '{}' is invalid: must start with an alphanumeric character",
            name
        )));
    }

    if !last.is_ascii_lowercase() && !last.is_ascii_digit() {
        return Err(Error::InvalidResource(format!(
            "name '{}' is invalid: must end with an alphanumeric character",
            name
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_resource_name("my-config").is_ok());
        assert!(validate_resource_name("test-123").is_ok());
        assert!(validate_resource_name("a").is_ok());
        assert!(validate_resource_name("my.config.map").is_ok());
        assert!(validate_resource_name("123").is_ok());
    }

    #[test]
    fn test_invalid_names() {
        // Empty
        assert!(validate_resource_name("").is_err());

        // Uppercase
        assert!(validate_resource_name("MyConfig").is_err());

        // Starts with dash
        assert!(validate_resource_name("-my-config").is_err());

        // Ends with dash
        assert!(validate_resource_name("my-config-").is_err());

        // Contains underscore
        assert!(validate_resource_name("my_config").is_err());

        // Contains space
        assert!(validate_resource_name("my config").is_err());

        // Too long (254 chars)
        let long_name = "a".repeat(254);
        assert!(validate_resource_name(&long_name).is_err());

        // Max length is OK (253 chars)
        let max_name = "a".repeat(253);
        assert!(validate_resource_name(&max_name).is_ok());
    }
}
