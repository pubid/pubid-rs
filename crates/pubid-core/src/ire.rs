//! The ire flavor: Institute of Radio Engineers identifiers
//! (1912-1963, the pre-IEEE series).
//!
//! The artifact binds `publisher` (`IRE` / `IEEE-IRE`), `type`
//! (`Trans.` / `Standard` / `Std`), `number` (the document code), and
//! `year`/`full_year`; the flavor layer rebuilds the year to a full
//! 4-digit string (2-digit forms in 12..63 gain the 1900s, the builder
//! rule) and splits the code through the shared IEEE code split (the
//! CodeNumber columns: number / prefix / parts / separator). The human
//! prints the short 20th-century year FIRST, then publisher, type
//! word, code.

use serde_json::Value;

use crate::artifact::ire_artifact;
use crate::bound::{optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};
use crate::ieee_code::IeeeCode;

/// The ire identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IreIdentifier {
    /// The publisher face (`IRE`, or the captured `IEEE-IRE`);
    /// defaults to `IRE` like the Ruby attribute.
    pub publisher: String,
    /// The spelled type word (`Trans.` / `Standard`), when present.
    pub type_word: Option<String>,
    /// The bare-digit number of the split code.
    pub number: String,
    /// The letter prefix of the split code, when the code carries one.
    pub prefix: Option<String>,
    /// The dotted/dashed parts after the number.
    pub parts: Vec<String>,
    /// The separator the source printed before the parts.
    pub separator: Option<String>,
    /// The full 4-digit publication year.
    pub year: Option<String>,
}

impl IreIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:ieee:ire";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "ire_identifier";

    /// Parse an input through the vendored artifact and rebuild the
    /// year/code exactly like `Pubid::Ieee::Ire::Builder`.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ire_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let code = IeeeCode::parse(crate::bound::bound_str(&bound, "number")?).ok_or(
            Error::InvalidHash {
                detail: "ire code does not split".to_string(),
            },
        )?;

        // Short years in 12..63 gain the 1900s; full_year overrides.
        let year = match optional_bound_str(&bound, "full_year") {
            Some(full_year) => Some(normalize_year(full_year)),
            None => optional_bound_str(&bound, "year").map(normalize_year),
        };

        Ok(Self {
            publisher: optional_bound_str(&bound, "publisher")
                .unwrap_or("IRE")
                .to_string(),
            type_word: optional_bound_str(&bound, "type").map(str::to_string),
            number: code.number,
            prefix: code.prefix,
            parts: code.parts,
            separator: code.separator,
            year,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        crate::bound::check_type(hash, Self::TYPE)?;
        let parts = match hash.get("parts") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str().map(str::to_string).ok_or(Error::InvalidHash {
                        detail: "ire parts entry is not a string".to_string(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            Some(other) => {
                return Err(Error::InvalidHash {
                    detail: format!("ire parts is {other:?}"),
                })
            }
        };
        Ok(Self {
            publisher: optional_hash_str(hash, "publisher")
                .unwrap_or("IRE")
                .to_string(),
            type_word: optional_hash_str(hash, "type").map(str::to_string),
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            prefix: optional_hash_str(hash, "prefix").map(str::to_string),
            parts,
            separator: optional_hash_str(hash, "separator").map(str::to_string),
            year: optional_hash_str(hash, "year").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the split columns
    /// (the defaults — publisher `IRE`, empty type — never serialize;
    /// `parts` drops when empty).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(prefix) = &self.prefix {
            object.insert("prefix".into(), prefix.clone().into());
        }
        if let Some(separator) = &self.separator {
            object.insert("separator".into(), separator.clone().into());
        }
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        if self.publisher != "IRE" {
            object.insert("publisher".into(), self.publisher.clone().into());
        }
        if let Some(type_word) = &self.type_word {
            object.insert("type".into(), type_word.clone().into());
        }
        if !self.parts.is_empty() {
            object.insert(
                "parts".into(),
                Value::Array(self.parts.iter().map(|part| part.clone().into()).collect()),
            );
        }
        Value::Object(object)
    }

    /// The canonical human representation: short 20th-century year
    /// first, then publisher, type word, code (the code keeps the
    /// separators it was printed with).
    pub fn to_human(&self) -> String {
        let mut parts = Vec::new();
        if let Some(year) = &self.year {
            parts.push(short_year(year));
        }
        parts.push(self.publisher.clone());
        if let Some(type_word) = &self.type_word {
            parts.push(type_word.clone());
        }
        parts.push(self.code().to_s());
        parts.join(" ")
    }

    /// The URN: `urn:ieee:<publisher>[:<type>]:<canonical code>
    /// [:<year>]` — spelling-stable code face, downcased segments.
    pub fn to_urn(&self) -> String {
        let mut parts = vec![
            "urn".to_string(),
            "ieee".to_string(),
            self.publisher.to_lowercase(),
        ];
        if let Some(type_word) = &self.type_word {
            if !type_word.trim().is_empty() && type_word != "Std" {
                parts.push(type_word.to_lowercase().replace(' ', "."));
            }
        }
        parts.push(self.code().canonical_to_s());
        if let Some(year) = &self.year {
            parts.push(year.clone());
        }
        parts.join(":")
    }

    /// The runtime code object rebuilt from the split columns
    /// (lossless: the columns carry prefix, number, parts, separator).
    fn code(&self) -> IeeeCode {
        IeeeCode {
            prefix: self.prefix.clone(),
            number: self.number.clone(),
            parts: self.parts.clone(),
            separator: self.separator.clone(),
            per_part: None,
        }
    }
}

/// The builder's year rule: 2-digit forms in 12..63 gain the 1900s.
fn normalize_year(year: &str) -> String {
    let value: u32 = year.parse().unwrap_or(0);
    if (12..=63).contains(&value) {
        (value + 1900).to_string()
    } else {
        value.to_string()
    }
}

/// The printed short year: 20th-century years drop the century.
fn short_year(year: &str) -> String {
    let value: u32 = year.parse().unwrap_or(0);
    if (1900..=1999).contains(&value) {
        (value - 1900).to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::IreIdentifier;
    use serde_json::json;

    #[test]
    fn year_first_form_converts_the_short_year_and_splits_the_code() {
        let id = IreIdentifier::parse("52 IRE 7.S2").expect("parse");
        assert_eq!(id.year.as_deref(), Some("1952"));
        assert_eq!(id.publisher, "IRE");
        assert_eq!(id.number, "7");
        assert_eq!(id.parts, vec!["S2".to_string()]);
        assert_eq!(id.separator.as_deref(), Some("."));
        assert_eq!(id.to_human(), "52 IRE 7.S2");
        assert_eq!(id.to_urn(), "urn:ieee:ire:7.S2:1952");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ieee:ire",
                "number": "7",
                "year": "1952",
                "parts": ["S2"],
                "separator": ".",
            })
        );
    }

    #[test]
    fn prefix_first_form_defaults_the_publisher() {
        let id = IreIdentifier::parse("IRE 28").expect("parse");
        assert_eq!(id.publisher, "IRE");
        assert_eq!(id.to_human(), "IRE 28");
        assert_eq!(id.to_urn(), "urn:ieee:ire:28");
    }

    #[test]
    fn a_spaced_code_stays_whole_like_the_reference_split() {
        // The reference Components::Code splits on dots and dashes
        // only, so "15 S1" is one bare number ("15 S1"), not parts.
        let id = IreIdentifier::parse("IRE Standard 15 S1").expect("parse");
        assert_eq!(id.type_word.as_deref(), Some("Standard"));
        assert_eq!(id.number, "15 S1");
        assert!(id.parts.is_empty());
        assert_eq!(id.to_human(), "IRE Standard 15 S1");
        // The type word is its own URN segment (the reference's
        // UrnGenerator joins the parts with colons).
        assert_eq!(id.to_urn(), "urn:ieee:ire:standard:15 S1");
    }

    #[test]
    fn round_trips_through_the_hash() {
        let id = IreIdentifier::parse("1930 IRE 20").expect("parse");
        let rebuilt = IreIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt, id);
        assert_eq!(rebuilt.to_hash(), id.to_hash());
    }
}
