//! The ansi flavor: ANSI standards and copublished editions.
//!
//! The artifact binds `publisher`, `number_with_part` (the whole
//! `802.3-2012` face), `copublishers[].copublisher`, and the
//! colon-form `date`; the flavor layer splits the dash-suffix into the
//! wire `part` (the reference's part-year face: `ANSI 802.3-2012`
//! carries `number: 802.3`, `part: '2012'`), and the colon date into
//! `year` (`ANSI/ISO 9899:1990`). The human joins the authorities with
//! slashes; the URN joins them with dashes and keeps the dash-lead
//! part segment (`urn:ansi:802.3:-2012`).

use serde_json::Value;

use crate::artifact::ansi_artifact;
use crate::bound::{bound_str, check_type, optional_bound_str, optional_hash_str};
use crate::error::Result;

/// The ansi identifier (the flavor has one wire class).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiIdentifier {
    /// The publishing body (`ANSI`).
    pub publisher: String,
    /// The copublishing bodies, in print order (`ASME`, `IEC`, ...).
    pub copublishers: Vec<String>,
    /// The document number (`802.3`, `C135.14`).
    pub number: String,
    /// The dash-suffix part (`2012` in `802.3-2012`).
    pub part: Option<String>,
    /// The colon-form date (`1990` in `9899:1990`).
    pub year: Option<String>,
}

impl AnsiIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:ansi:standard";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and split the
    /// number face exactly like the reference's wire hash.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ansi_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let number_with_part = bound_str(&bound, "number_with_part")?;
        // The trailing all-digit dash segment is the part face.
        let (number, part) = match number_with_part.rsplit_once('-') {
            Some((left, right)) if !right.is_empty() && right.chars().all(|c| c.is_ascii_digit()) => {
                (left, Some(right.to_string()))
            }
            _ => (number_with_part, None),
        };
        let copublishers = match bound.get("copublishers") {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|item| item.get("copublisher").and_then(Value::as_str))
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };

        Ok(Self {
            publisher: bound_str(&bound, "publisher")?.to_string(),
            copublishers,
            number: number.to_string(),
            part,
            year: optional_bound_str(&bound, "date").map(str::to_string),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record;
    /// the publisher/copublishers carry nested `body` objects).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        let publisher = hash
            .get("publisher")
            .and_then(|publisher| publisher.get("body"))
            .and_then(Value::as_str)
            .ok_or(crate::error::Error::MissingAttribute {
                capture: "publisher.body",
            })?;
        let copublishers = match hash.get("copublishers") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.get("body")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .ok_or(crate::error::Error::InvalidHash {
                            detail: "copublishers entry has no body".to_string(),
                        })
                })
                .collect::<Result<Vec<_>>>()?,
            Some(other) => {
                return Err(crate::error::Error::InvalidHash {
                    detail: format!("copublishers is {other:?}"),
                })
            }
        };
        Ok(Self {
            publisher: publisher.to_string(),
            copublishers,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            part: optional_hash_str(hash, "part").map(str::to_string),
            year: optional_hash_str(hash, "year").map(str::to_string),
        })
    }

    /// The canonical hash: `_type`, number, part, the nested
    /// publisher/copublisher bodies, then the year (absent optionals
    /// omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(part) = &self.part {
            object.insert("part".into(), part.clone().into());
        }
        object.insert(
            "publisher".into(),
            serde_json::json!({ "body": self.publisher }),
        );
        if !self.copublishers.is_empty() {
            object.insert(
                "copublishers".into(),
                Value::Array(
                    self.copublishers
                        .iter()
                        .map(|body| serde_json::json!({ "body": body }))
                        .collect(),
                ),
            );
        }
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human: authorities joined with slashes, then the
    /// number, dash-part, colon-year.
    pub fn to_human(&self) -> String {
        let mut authorities = vec![self.publisher.clone()];
        authorities.extend(self.copublishers.iter().cloned());
        let mut out = authorities.join("/");
        out.push(' ');
        out.push_str(&self.number);
        if let Some(part) = &self.part {
            out.push('-');
            out.push_str(part);
        }
        if let Some(year) = &self.year {
            out.push(':');
            out.push_str(year);
        }
        out
    }

    /// The URN: `urn:<pub>[-<copub>]*:<number>[:-<part>][:<year>]` —
    /// the part segment keeps its dash-lead, downcased authorities.
    pub fn to_urn(&self) -> String {
        let mut authorities = vec![self.publisher.to_lowercase()];
        authorities.extend(self.copublishers.iter().map(|body| body.to_lowercase()));
        let mut out = format!("urn:{}:{}", authorities.join("-"), self.number);
        if let Some(part) = &self.part {
            out.push_str(&format!(":-{part}"));
        }
        if let Some(year) = &self.year {
            out.push_str(&format!(":{year}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::AnsiIdentifier;
    use serde_json::json;

    #[test]
    fn splits_the_dash_suffix_into_the_part_face() {
        let id = AnsiIdentifier::parse("ANSI 802.3-2012").expect("parse");
        assert_eq!(id.number, "802.3");
        assert_eq!(id.part.as_deref(), Some("2012"));
        assert_eq!(id.publisher, "ANSI");
        assert!(id.copublishers.is_empty());
        assert_eq!(id.to_human(), "ANSI 802.3-2012");
        // The part segment keeps its dash-lead in the URN.
        assert_eq!(id.to_urn(), "urn:ansi:802.3:-2012");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ansi:standard",
                "number": "802.3",
                "part": "2012",
                "publisher": {"body": "ANSI"},
            })
        );
    }

    #[test]
    fn copublishers_join_with_slashes_in_the_human_and_dashes_in_the_urn() {
        let id = AnsiIdentifier::parse("ANSI/IEC 60601-1").expect("parse");
        assert_eq!(id.copublishers, vec!["IEC".to_string()]);
        assert_eq!(id.to_human(), "ANSI/IEC 60601-1");
        assert_eq!(id.to_urn(), "urn:ansi-iec:60601:-1");
    }

    #[test]
    fn the_colon_date_becomes_the_year() {
        let id = AnsiIdentifier::parse("ANSI/ISO 9899:1990").expect("parse");
        assert_eq!(id.year.as_deref(), Some("1990"));
        assert!(id.part.is_none());
        assert_eq!(id.to_human(), "ANSI/ISO 9899:1990");
        assert_eq!(id.to_urn(), "urn:ansi-iso:9899:1990");
    }

    #[test]
    fn an_undated_number_stays_whole() {
        let id = AnsiIdentifier::parse("ANSI/ASME B16.5").expect("parse");
        assert_eq!(id.number, "B16.5");
        assert_eq!(id.to_human(), "ANSI/ASME B16.5");
        assert_eq!(id.to_urn(), "urn:ansi-asme:B16.5");
    }

    #[test]
    fn round_trips_through_the_hash() {
        let id = AnsiIdentifier::parse("ANSI C135.14-1979").expect("parse");
        let rebuilt = AnsiIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt, id);
        assert_eq!(rebuilt.to_hash(), id.to_hash());
    }
}
