//! The un flavor: slash-separated UN document paths.
//!
//! The artifact binds every path segment under `token[].token` (the
//! repeat-of-captures shape); the flavor layer splits the final token
//! off as `number`, derives a parse-time `date` from the last 4-digit
//! PATH token exactly like the Ruby builder, and the URN template
//! reads it: `urn:un:<number>[:<year>]`. The key-value mapping carries
//! only `_type`/`path`/`number`, so `date` never serializes into the
//! canonical hash (a 4-digit NUMBER is not a year).

use serde_json::Value;

use crate::artifact::un_artifact;
use crate::bound::{check_type, hash_str};
use crate::error::{Error, Result};

/// The un identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnIdentifier {
    /// The slash-separated path tokens (never includes the number).
    pub path: Vec<String>,
    /// The final token (may carry a letter prefix, e.g. `R.1068`).
    pub number: String,
}

impl UnIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:un:document";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact (bound hash).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = un_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        let tokens = extract_tokens(&bound);
        if tokens.is_empty() {
            return Err(Error::InvalidHash {
                detail: "UN identifier has no tokens".to_string(),
            });
        }
        let number = tokens[tokens.len() - 1].clone();
        let path = tokens[..tokens.len() - 1].to_vec();
        Ok(Self { path, number })
    }

    /// The 4-digit path token driving the URN year slot, if any (a
    /// parse-time attribute; it never serializes into the hash).
    pub fn date_year(&self) -> Option<&str> {
        self.path
            .iter()
            .rev()
            .find(|token| token.len() == 4 && token.chars().all(|c| c.is_ascii_digit()))
            .map(String::as_str)
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        let path = match hash.get("path") {
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str().map(str::to_string).ok_or(Error::InvalidHash {
                        detail: "un path entry is not a string".to_string(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            _ => {
                return Err(Error::InvalidHash {
                    detail: "un hash has no path array".to_string(),
                })
            }
        };
        Ok(Self {
            path,
            number: hash_str(hash, "number")?.to_string(),
        })
    }

    /// The canonical hash: `_type` first, then `path` (always an
    /// array) and `number` — the `date` never serializes.
    pub fn to_hash(&self) -> Value {
        serde_json::json!({
            "_type": Self::TYPE,
            "path": self.path,
            "number": self.number,
        })
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        let mut parts = self.path.clone();
        parts.push(self.number.clone());
        parts.join("/")
    }

    /// The URN: `urn:un:<number>[:<year>]` — the year-only render of
    /// the derived date when a path token is a 4-digit year.
    pub fn to_urn(&self) -> String {
        match self.date_year() {
            Some(year) => format!("urn:un:{}:{}", self.number, year),
            None => format!("urn:un:{}", self.number),
        }
    }
}

/// Walk the bound `token[].token` repeats into plain strings (the
/// repeated capture lands as an array of one-key objects; a scalar
/// shape is tolerated the same way the Ruby builder walks its tree).
fn extract_tokens(bound: &Value) -> Vec<String> {
    fn flatten(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Array(items) => items.iter().for_each(|item| flatten(item, out)),
            Value::Object(map) => {
                if let Some(inner) = map.get("token") {
                    flatten(inner, out);
                }
            }
            Value::String(text) => out.push(text.clone()),
            _ => {}
        }
    }
    let mut tokens = Vec::new();
    match bound.get("token") {
        Some(value) => flatten(value, &mut tokens),
        None => {
            // A scalar bound hash (single segment) is not in the
            // corpus, but the walk mirrors the Ruby extract_tokens.
            if let Some(text) = bound.as_str() {
                tokens.push(text.to_string());
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::UnIdentifier;
    use serde_json::json;

    #[test]
    fn splits_number_from_path_and_derives_the_urn_year() {
        let resolved = UnIdentifier::parse("TRADE/CEFACT/2004/32").expect("parse");
        assert_eq!(resolved.path, vec!["TRADE", "CEFACT", "2004"]);
        assert_eq!(resolved.number, "32");
        assert_eq!(resolved.to_human(), "TRADE/CEFACT/2004/32");
        assert_eq!(resolved.to_urn(), "urn:un:32:2004");
        assert_eq!(
            resolved.to_hash()["path"],
            json!(["TRADE", "CEFACT", "2004"])
        );
    }

    #[test]
    fn a_four_digit_number_is_not_a_year() {
        let id = UnIdentifier::parse("TRADE/WP.4/1068").expect("parse");
        assert_eq!(id.to_urn(), "urn:un:1068");
        let lettered = UnIdentifier::parse("TRADE/WP.4/R.1068").expect("parse");
        assert_eq!(lettered.number, "R.1068");
        assert_eq!(lettered.to_urn(), "urn:un:R.1068");
    }

    #[test]
    fn the_year_prefix_is_consumed_and_never_serialized() {
        let id = UnIdentifier::parse("UN A/RES/78/1").expect("parse");
        assert_eq!(id.path, vec!["A", "RES", "78"]);
        assert_eq!(id.to_urn(), "urn:un:1");
        let rebuilt = UnIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt.to_hash(), id.to_hash());
        assert_eq!(rebuilt.to_human(), id.to_human());
    }
}
