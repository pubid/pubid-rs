//! Shared bound-hash and canonical-hash readers for the flavor layer.
//!
//! Every flavor parses through its artifact into a bound hash
//! (`parse_and_bind`) and rebuilds from a canonical hash (`from_hash`);
//! these helpers keep the capture reads uniform across flavors, so a
//! missing or non-string capture fails with the same error everywhere.

use serde_json::Value;

use crate::error::{Error, Result};

/// A required string capture from a bound hash.
pub fn bound_str<'a>(bound: &'a Value, capture: &'static str) -> Result<&'a str> {
    bound
        .get(capture)
        .and_then(Value::as_str)
        .ok_or(Error::MissingAttribute { capture })
}

/// An optional string capture from a bound hash (absent or null means
/// the grammar did not match the branch).
pub fn optional_bound_str<'a>(bound: &'a Value, capture: &'static str) -> Option<&'a str> {
    bound.get(capture).and_then(Value::as_str)
}

/// A string capture that may arrive span-wrapped (`{value, line,
/// column, offset, length}` — the bindings runtime's position-tagged
/// leaves) or as a plain string; both unwrap to the captured text.
pub fn capture_str(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Object(map) => map.get("value").and_then(Value::as_str),
        _ => None,
    }
}

/// An optional string capture from a bound hash or a captured node,
/// span-unwrapping like [`capture_str`].
pub fn optional_at<'a>(node: &'a Value, key: &'static str) -> Option<&'a str> {
    node.get(key).and_then(capture_str)
}

/// An optional string member of a captured entry (`parts[].part`,
/// `copublishers[].copublisher`), span-unwrapping like [`capture_str`].
pub fn capture_in<'a>(node: &'a Value, key: &'static str) -> Option<&'a str> {
    node.get(key).and_then(capture_str)
}

/// A required string field from a canonical hash (`from_hash` side).
pub fn hash_str<'a>(hash: &'a Value, field: &'static str) -> Result<&'a str> {
    hash.get(field)
        .and_then(Value::as_str)
        .ok_or(Error::MissingAttribute { capture: field })
}

/// An optional string field from a canonical hash.
pub fn optional_hash_str<'a>(hash: &'a Value, field: &'static str) -> Option<&'a str> {
    hash.get(field).and_then(Value::as_str)
}

/// The `_type` tag check shared by every flavor's `from_hash`.
pub fn check_type(hash: &Value, expected: &str) -> Result<()> {
    let kind = hash
        .get("_type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::InvalidHash {
            detail: "missing _type".to_string(),
        })?;
    if kind != expected {
        return Err(Error::InvalidHash {
            detail: format!("_type is not {expected}"),
        });
    }
    Ok(())
}

/// Strip the rfc-index.xml zero-pad (`"STD0066"` -> `"66"`; `"000"`
/// stays `"0"`), mirroring the flavor semantics.
pub fn unpad(number: &str) -> &str {
    let stripped = number.trim_start_matches('0');
    if stripped.is_empty() {
        "0"
    } else {
        stripped
    }
}

#[cfg(test)]
mod tests {
    use super::unpad;
    use serde_json::json;

    #[test]
    fn unpad_strips_but_keeps_one_digit() {
        assert_eq!(unpad("0066"), "66");
        assert_eq!(unpad("0001"), "1");
        assert_eq!(unpad("000"), "0");
        assert_eq!(unpad("0"), "0");
        assert_eq!(unpad("2616"), "2616");
    }

    #[test]
    fn missing_capture_names_the_field() {
        let err = super::bound_str(&json!({}), "prefix").unwrap_err();
        assert!(err.to_string().contains("prefix"));
    }
}
