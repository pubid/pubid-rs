//! The isbn flavor: ISBN-10/13 per ISO 2108.
//!
//! The artifact carries the grammar and the `body` binding; the flavor
//! layer validates the ISO 2108 length and check digit and splits
//! `raw`/`hyphenated`, mirroring the pubid-ts isbn builder (Ruby Builder
//! parity). The Ruby flavor has no urn_generator, so `to_urn` resolves
//! to the base template (`urn:isbn`).

use serde_json::Value;

use crate::artifact::isbn_artifact;
use crate::bound::{bound_str, check_type, hash_str, optional_hash_str};
use crate::error::{Error, Result};

/// The isbn identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsbnIdentifier {
    /// The digits without hyphens, e.g. `9780306406157`.
    pub raw: String,
    /// The hyphenated input spelling, present only when the input
    /// carried hyphens.
    pub hyphenated: Option<String>,
}

impl IsbnIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:isbn:book";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact, then apply the
    /// builder-layer validation: 10 or 13 digits and a valid ISO 2108
    /// check digit (surfaces failures as parse errors).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = isbn_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        let body = bound_str(&bound, "body")?;
        let raw: String = body.chars().filter(|c| *c != '-').collect();
        if !valid_length(&raw) {
            return Err(Error::InvalidHash {
                detail: format!("ISBN must be 10 or 13 digits (got {})", raw.chars().count()),
            });
        }
        if !valid_check_digit(&raw) {
            return Err(Error::InvalidHash {
                detail: format!("ISBN check digit invalid for {raw}"),
            });
        }
        let hyphenated = body.contains('-').then(|| body.to_string());
        Ok(Self { raw, hyphenated })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        Ok(Self {
            raw: hash_str(hash, "raw")?.to_string(),
            hyphenated: optional_hash_str(hash, "hyphenated").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("raw".into(), self.raw.clone().into());
        if let Some(hyphenated) = &self.hyphenated {
            object.insert("hyphenated".into(), hyphenated.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        match &self.hyphenated {
            Some(hyphenated) => format!("ISBN {hyphenated}"),
            None => format!("ISBN {}", self.raw),
        }
    }

    /// The URN (base template: no attribute hits a slot).
    pub fn to_urn(&self) -> String {
        "urn:isbn".to_string()
    }
}

/// The ISO 2108 length: 10 characters (`\d{9}[\dX]`) or 13 digits.
fn valid_length(raw: &str) -> bool {
    let digits: Vec<char> = raw.chars().collect();
    if digits.len() == 10 {
        digits[..9].iter().all(|c| c.is_ascii_digit())
            && (digits[9].is_ascii_digit() || digits[9] == 'X')
    } else if digits.len() == 13 {
        digits.iter().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

/// The ISO 2108 check digit: ISBN-10 weights 10..1 mod 11 (10 renders
/// as "X"); ISBN-13 alternating weights 1/3 mod 10.
fn valid_check_digit(raw: &str) -> bool {
    let digits: Vec<char> = raw.to_uppercase().chars().collect();
    match digits.len() {
        10 => {
            let sum: u32 = digits[..9]
                .iter()
                .enumerate()
                .map(|(index, c)| c.to_digit(10).unwrap_or(0) * (10 - index as u32))
                .sum();
            let computed = (11 - sum % 11) % 11;
            let expected = if computed == 10 {
                'X'
            } else {
                char::from(computed as u8 + b'0')
            };
            expected == digits[9]
        }
        13 => {
            let sum: u32 = digits[..12]
                .iter()
                .enumerate()
                .map(|(index, c)| {
                    let weight = if index % 2 == 0 { 1 } else { 3 };
                    c.to_digit(10).unwrap_or(0) * weight
                })
                .sum();
            let computed = (10 - sum % 10) % 10;
            char::from(computed as u8 + b'0') == digits[12]
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::IsbnIdentifier;
    use serde_json::json;

    #[test]
    fn parses_hyphenated_and_bare_forms() {
        let hyphenated = IsbnIdentifier::parse("ISBN 978-0-306-40615-7").expect("parse");
        assert_eq!(hyphenated.raw, "9780306406157");
        assert_eq!(hyphenated.hyphenated.as_deref(), Some("978-0-306-40615-7"));
        assert_eq!(hyphenated.to_human(), "ISBN 978-0-306-40615-7");

        let bare = IsbnIdentifier::parse("9780306406157").expect("parse");
        assert_eq!(bare.hyphenated, None);
        assert_eq!(
            bare.to_hash(),
            json!({"_type": "pubid:isbn:book", "raw": "9780306406157"})
        );
    }

    #[test]
    fn rejects_bad_check_digit_and_length() {
        assert!(IsbnIdentifier::parse("ISBN 978-3-16-148410-1").is_err());
        assert!(IsbnIdentifier::parse("ISBN 12345").is_err());
        assert!(IsbnIdentifier::parse("ISBN 12345678901234567890").is_err());
    }

    #[test]
    fn isbn10_x_check_digit() {
        let id = IsbnIdentifier::parse("0-306-40615-2").expect("parse");
        assert_eq!(id.raw, "0306406152");
    }
}
