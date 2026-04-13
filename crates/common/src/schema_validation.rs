// OpenAPI v3 Schema Validation for Custom Resources
//
// This module provides JSON Schema validation for custom resources based on
// OpenAPI v3 schemas defined in CustomResourceDefinitions.

use crate::error::Error;
use crate::resources::crd::JSONSchemaProps;
use serde_json::Value;

/// Validator validates JSON values against JSONSchemaProps
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate a JSON value against a schema
    pub fn validate(schema: &JSONSchemaProps, value: &Value) -> Result<(), Error> {
        Self::validate_with_path(schema, value, "")
    }

    /// Validate with a path for error reporting
    fn validate_with_path(
        schema: &JSONSchemaProps,
        value: &Value,
        path: &str,
    ) -> Result<(), Error> {
        // Validate type
        if let Some(ref type_) = schema.type_ {
            Self::validate_type(type_, value, path)?;
        }

        // Validate based on value type
        match value {
            Value::Object(obj) => Self::validate_object(schema, obj, path)?,
            Value::Array(arr) => Self::validate_array(schema, arr, path)?,
            Value::String(s) => Self::validate_string(schema, s, path)?,
            Value::Number(n) => Self::validate_number(schema, n, path)?,
            Value::Bool(_) => {} // Booleans don't have additional constraints
            Value::Null => {
                if let Some(false) = schema.nullable {
                    return Err(Error::InvalidResource(format!(
                        "Field at {} cannot be null",
                        path
                    )));
                }
            }
        }

        // Validate enum
        if let Some(ref enum_values) = schema.enum_ {
            if !enum_values.contains(value) {
                return Err(Error::InvalidResource(format!(
                    "Field at {} must be one of {:?}",
                    path, enum_values
                )));
            }
        }

        // Validate oneOf
        if let Some(ref one_of) = schema.one_of {
            let matches: Vec<_> = one_of
                .iter()
                .filter(|s| Self::validate_with_path(s, value, path).is_ok())
                .collect();

            if matches.len() != 1 {
                return Err(Error::InvalidResource(format!(
                    "Field at {} must match exactly one schema (matched {})",
                    path,
                    matches.len()
                )));
            }
        }

        // Validate anyOf
        if let Some(ref any_of) = schema.any_of {
            let matches = any_of
                .iter()
                .any(|s| Self::validate_with_path(s, value, path).is_ok());

            if !matches {
                return Err(Error::InvalidResource(format!(
                    "Field at {} must match at least one schema",
                    path
                )));
            }
        }

        // Validate allOf
        if let Some(ref all_of) = schema.all_of {
            for sub_schema in all_of {
                Self::validate_with_path(sub_schema, value, path)?;
            }
        }

        // Validate not
        if let Some(ref not) = schema.not {
            if Self::validate_with_path(not, value, path).is_ok() {
                return Err(Error::InvalidResource(format!(
                    "Field at {} must not match the schema",
                    path
                )));
            }
        }

        Ok(())
    }

    fn validate_type(type_: &str, value: &Value, path: &str) -> Result<(), Error> {
        let matches = match (type_, value) {
            ("object", Value::Object(_)) => true,
            ("array", Value::Array(_)) => true,
            ("string", Value::String(_)) => true,
            ("number", Value::Number(_)) => true,
            ("integer", Value::Number(n)) => n.is_i64() || n.is_u64(),
            ("boolean", Value::Bool(_)) => true,
            ("null", Value::Null) => true,
            _ => false,
        };

        if !matches {
            return Err(Error::InvalidResource(format!(
                "Field at {} must be of type {}, got {:?}",
                path,
                type_,
                Self::value_type(value)
            )));
        }

        Ok(())
    }

    fn validate_object(
        schema: &JSONSchemaProps,
        obj: &serde_json::Map<String, Value>,
        path: &str,
    ) -> Result<(), Error> {
        // Validate required fields
        if let Some(ref required) = schema.required {
            for field in required {
                if !obj.contains_key(field) {
                    return Err(Error::InvalidResource(format!(
                        "Required field '{}' missing at {}",
                        field, path
                    )));
                }
            }
        }

        // Validate min/max properties
        if let Some(min) = schema.min_properties {
            if (obj.len() as i64) < min {
                return Err(Error::InvalidResource(format!(
                    "Object at {} must have at least {} properties",
                    path, min
                )));
            }
        }

        if let Some(max) = schema.max_properties {
            if (obj.len() as i64) > max {
                return Err(Error::InvalidResource(format!(
                    "Object at {} must have at most {} properties",
                    path, max
                )));
            }
        }

        // K8s embedded resource meta fields are always allowed:
        // apiVersion, kind, metadata. See: pruning/algorithm.go:77
        let is_embedded = schema.x_kubernetes_embedded_resource == Some(true);

        // Validate properties
        if let Some(ref properties) = schema.properties {
            for (key, value) in obj {
                if let Some(prop_schema) = properties.get(key) {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    Self::validate_with_path(prop_schema, value, &new_path)?;
                } else if is_embedded && (key == "apiVersion" || key == "kind" || key == "metadata")
                {
                    // Embedded resource meta fields are implicitly allowed
                    continue;
                } else if let Some(ref additional) = schema.additional_properties {
                    // Validate against additionalProperties schema
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    Self::validate_additional_properties(additional, value, &new_path)?;
                } else if schema.x_kubernetes_preserve_unknown_fields != Some(true) {
                    // Unknown field and additionalProperties not set
                    return Err(Error::InvalidResource(format!(
                        "Unknown field '{}' at {}",
                        key, path
                    )));
                }
            }
        } else if let Some(ref additional) = schema.additional_properties {
            // No specific properties defined, validate all against additionalProperties
            for (key, value) in obj {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                Self::validate_additional_properties(additional, value, &new_path)?;
            }
        }

        Ok(())
    }

    fn validate_additional_properties(
        additional: &crate::resources::crd::JSONSchemaPropsOrBool,
        value: &Value,
        path: &str,
    ) -> Result<(), Error> {
        use crate::resources::crd::JSONSchemaPropsOrBool;

        match additional {
            JSONSchemaPropsOrBool::Schema(schema) => {
                Self::validate_with_path(schema, value, path)?;
            }
            JSONSchemaPropsOrBool::Bool(false) => {
                return Err(Error::InvalidResource(format!(
                    "Additional property at {} not allowed",
                    path
                )));
            }
            JSONSchemaPropsOrBool::Bool(true) => {
                // Any additional properties are allowed
            }
        }

        Ok(())
    }

    fn validate_array(schema: &JSONSchemaProps, arr: &[Value], path: &str) -> Result<(), Error> {
        // Validate min/max items
        if let Some(min) = schema.min_items {
            if (arr.len() as i64) < min {
                return Err(Error::InvalidResource(format!(
                    "Array at {} must have at least {} items",
                    path, min
                )));
            }
        }

        if let Some(max) = schema.max_items {
            if (arr.len() as i64) > max {
                return Err(Error::InvalidResource(format!(
                    "Array at {} must have at most {} items",
                    path, max
                )));
            }
        }

        // Validate unique items
        if let Some(true) = schema.unique_items {
            let mut seen = Vec::new();
            for item in arr {
                if seen.contains(item) {
                    return Err(Error::InvalidResource(format!(
                        "Array at {} must have unique items",
                        path
                    )));
                }
                seen.push(item.clone());
            }
        }

        // Validate items schema
        if let Some(ref items) = schema.items {
            use crate::resources::crd::JSONSchemaPropsOrArray;

            match items.as_ref() {
                JSONSchemaPropsOrArray::Schema(item_schema) => {
                    // All items must match this schema
                    for (i, item) in arr.iter().enumerate() {
                        let new_path = format!("{}[{}]", path, i);
                        Self::validate_with_path(item_schema, item, &new_path)?;
                    }
                }
                JSONSchemaPropsOrArray::Schemas(schemas) => {
                    // Each item matches the corresponding schema (tuple validation)
                    for (i, item) in arr.iter().enumerate() {
                        if i < schemas.len() {
                            let new_path = format!("{}[{}]", path, i);
                            Self::validate_with_path(&schemas[i], item, &new_path)?;
                        } else if let Some(ref additional) = schema.additional_items {
                            // Additional items beyond schema array
                            let new_path = format!("{}[{}]", path, i);
                            Self::validate_additional_items(additional, item, &new_path)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_additional_items(
        additional: &crate::resources::crd::JSONSchemaPropsOrBool,
        value: &Value,
        path: &str,
    ) -> Result<(), Error> {
        use crate::resources::crd::JSONSchemaPropsOrBool;

        match additional {
            JSONSchemaPropsOrBool::Schema(schema) => {
                Self::validate_with_path(schema, value, path)?;
            }
            JSONSchemaPropsOrBool::Bool(false) => {
                return Err(Error::InvalidResource(format!(
                    "Additional item at {} not allowed",
                    path
                )));
            }
            JSONSchemaPropsOrBool::Bool(true) => {
                // Any additional items are allowed
            }
        }

        Ok(())
    }

    fn validate_string(schema: &JSONSchemaProps, s: &str, path: &str) -> Result<(), Error> {
        // Validate min/max length
        if let Some(min) = schema.min_length {
            if (s.len() as i64) < min {
                return Err(Error::InvalidResource(format!(
                    "String at {} must be at least {} characters",
                    path, min
                )));
            }
        }

        if let Some(max) = schema.max_length {
            if (s.len() as i64) > max {
                return Err(Error::InvalidResource(format!(
                    "String at {} must be at most {} characters",
                    path, max
                )));
            }
        }

        // Validate pattern
        if let Some(ref pattern) = schema.pattern {
            let re = regex::Regex::new(pattern)
                .map_err(|e| Error::InvalidResource(format!("Invalid regex pattern: {}", e)))?;

            if !re.is_match(s) {
                return Err(Error::InvalidResource(format!(
                    "String at {} must match pattern '{}'",
                    path, pattern
                )));
            }
        }

        // Validate format
        if let Some(ref format) = schema.format {
            Self::validate_format(format, s, path)?;
        }

        Ok(())
    }

    fn validate_number(
        schema: &JSONSchemaProps,
        n: &serde_json::Number,
        path: &str,
    ) -> Result<(), Error> {
        let value = n.as_f64().unwrap_or(0.0);

        // Validate minimum
        if let Some(min) = schema.minimum {
            let exclusive = schema.exclusive_minimum.unwrap_or(false);
            if exclusive {
                if value <= min {
                    return Err(Error::InvalidResource(format!(
                        "Number at {} must be greater than {}",
                        path, min
                    )));
                }
            } else if value < min {
                return Err(Error::InvalidResource(format!(
                    "Number at {} must be at least {}",
                    path, min
                )));
            }
        }

        // Validate maximum
        if let Some(max) = schema.maximum {
            let exclusive = schema.exclusive_maximum.unwrap_or(false);
            if exclusive {
                if value >= max {
                    return Err(Error::InvalidResource(format!(
                        "Number at {} must be less than {}",
                        path, max
                    )));
                }
            } else if value > max {
                return Err(Error::InvalidResource(format!(
                    "Number at {} must be at most {}",
                    path, max
                )));
            }
        }

        Ok(())
    }

    fn validate_format(format: &str, value: &str, path: &str) -> Result<(), Error> {
        match format {
            "date-time" => {
                // Simplified date-time validation
                if !value.contains('T') || !value.contains(':') {
                    return Err(Error::InvalidResource(format!(
                        "String at {} must be a valid date-time",
                        path
                    )));
                }
            }
            "email" => {
                if !value.contains('@') || !value.contains('.') {
                    return Err(Error::InvalidResource(format!(
                        "String at {} must be a valid email",
                        path
                    )));
                }
            }
            "uri" | "url" => {
                if !value.starts_with("http://") && !value.starts_with("https://") {
                    return Err(Error::InvalidResource(format!(
                        "String at {} must be a valid URL",
                        path
                    )));
                }
            }
            "uuid" => {
                // Simplified UUID validation
                if value.len() != 36 || value.chars().filter(|c| *c == '-').count() != 4 {
                    return Err(Error::InvalidResource(format!(
                        "String at {} must be a valid UUID",
                        path
                    )));
                }
            }
            _ => {
                // Unknown formats are ignored per JSON Schema spec
            }
        }

        Ok(())
    }

    fn value_type(value: &Value) -> &str {
        match value {
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Null => "null",
        }
    }

    /// Apply default values from a JSONSchemaProps to a JSON value.
    ///
    /// Walks the schema recursively and for each property that has a `default`
    /// value, inserts that default into `value` if the field is missing.
    /// Handles nested objects recursively.
    pub fn apply_defaults(schema: &JSONSchemaProps, value: &mut Value) {
        Self::apply_defaults_recursive(schema, value);
    }

    fn apply_defaults_recursive(schema: &JSONSchemaProps, value: &mut Value) {
        // If the value is an object, walk its properties and apply defaults
        if let Value::Object(ref mut map) = value {
            if let Some(ref properties) = schema.properties {
                for (key, prop_schema) in properties {
                    if map.contains_key(key) {
                        // Property exists -- recurse into it for nested defaults
                        if let Some(val) = map.get_mut(key) {
                            Self::apply_defaults_recursive(prop_schema, val);
                        }
                    } else if let Some(ref default_val) = prop_schema.default {
                        // Property missing -- apply default
                        map.insert(key.clone(), default_val.clone());
                    }
                }
            }
        }

        // If the value is an array, apply defaults to each item using the items schema
        if let Value::Array(ref mut arr) = value {
            if let Some(ref items) = schema.items {
                use crate::resources::crd::JSONSchemaPropsOrArray;
                match items.as_ref() {
                    JSONSchemaPropsOrArray::Schema(item_schema) => {
                        for item in arr.iter_mut() {
                            Self::apply_defaults_recursive(item_schema, item);
                        }
                    }
                    JSONSchemaPropsOrArray::Schemas(schemas) => {
                        for (i, item) in arr.iter_mut().enumerate() {
                            if i < schemas.len() {
                                Self::apply_defaults_recursive(&schemas[i], item);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_validate_type_object() {
        let schema = JSONSchemaProps {
            type_: Some("object".to_string()),
            ..Default::default()
        };

        let valid = json!({"key": "value"});
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let invalid = json!("string");
        assert!(SchemaValidator::validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_validate_required_fields() {
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            JSONSchemaProps {
                type_: Some("string".to_string()),
                ..Default::default()
            },
        );

        let schema = JSONSchemaProps {
            type_: Some("object".to_string()),
            required: Some(vec!["name".to_string()]),
            properties: Some(properties),
            ..Default::default()
        };

        let valid = json!({"name": "test"});
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let invalid = json!({});
        assert!(SchemaValidator::validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_validate_string_length() {
        let schema = JSONSchemaProps {
            type_: Some("string".to_string()),
            min_length: Some(3),
            max_length: Some(10),
            ..Default::default()
        };

        let valid = json!("hello");
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let too_short = json!("hi");
        assert!(SchemaValidator::validate(&schema, &too_short).is_err());

        let too_long = json!("hello world!");
        assert!(SchemaValidator::validate(&schema, &too_long).is_err());
    }

    #[test]
    fn test_validate_number_range() {
        let schema = JSONSchemaProps {
            type_: Some("number".to_string()),
            minimum: Some(0.0),
            maximum: Some(100.0),
            ..Default::default()
        };

        let valid = json!(50);
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let too_small = json!(-1);
        assert!(SchemaValidator::validate(&schema, &too_small).is_err());

        let too_large = json!(101);
        assert!(SchemaValidator::validate(&schema, &too_large).is_err());
    }

    #[test]
    fn test_validate_array_items() {
        let schema = JSONSchemaProps {
            type_: Some("array".to_string()),
            items: Some(Box::new(
                crate::resources::crd::JSONSchemaPropsOrArray::Schema(JSONSchemaProps {
                    type_: Some("string".to_string()),
                    ..Default::default()
                }),
            )),
            ..Default::default()
        };

        let valid = json!(["a", "b", "c"]);
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let invalid = json!(["a", 1, "c"]);
        assert!(SchemaValidator::validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_validate_enum() {
        let schema = JSONSchemaProps {
            type_: Some("string".to_string()),
            enum_: Some(vec![json!("red"), json!("green"), json!("blue")]),
            ..Default::default()
        };

        let valid = json!("red");
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let invalid = json!("yellow");
        assert!(SchemaValidator::validate(&schema, &invalid).is_err());
    }

    #[test]
    fn test_validate_pattern() {
        let schema = JSONSchemaProps {
            type_: Some("string".to_string()),
            pattern: Some("^[a-z]+$".to_string()),
            ..Default::default()
        };

        let valid = json!("hello");
        assert!(SchemaValidator::validate(&schema, &valid).is_ok());

        let invalid = json!("Hello123");
        assert!(SchemaValidator::validate(&schema, &invalid).is_err());
    }
}
