//! The amca flavor: AMCA standard / publication / interpretation
//! identifiers.
//!
//! The artifact binds flat scalars (`copublisher`, `code`, `year`,
//! `reaffirmed`, `revision`, `interpretation_code`,
//! `interpretation_year`, `publication_keyword`) plus the raw
//! `interpretation` wrapper of the interpretation branch; the flavor
//! layer reads captures top-level first and falls back into the
//! wrapper (the binder cannot nest the branch's captures), then
//! dispatches the class exactly like `Pubid::Amca::Builder`
//! (publication before interpretation before standard). `code` is the
//! document `number` (the leaves declare `attribute :number`); the
//! default `publisher: "AMCA"` never serializes.

use serde_json::Value;

use crate::artifact::amca_artifact;
use crate::bound::{check_type, hash_str, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete AMCA class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmcaKind {
    Standard,
    Publication,
    Interpretation,
}

impl AmcaKind {
    /// The shared `_type` prefix every amca tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:amca:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            AmcaKind::Standard => "pubid:amca:standard",
            AmcaKind::Publication => "pubid:amca:publication",
            AmcaKind::Interpretation => "pubid:amca:interpretation",
        }
    }

    /// The URN type key (`self.class.type[:key]`, appended last).
    fn urn_key(self) -> &'static str {
        match self {
            AmcaKind::Standard => "standard",
            AmcaKind::Publication => "publication",
            AmcaKind::Interpretation => "interpretation",
        }
    }

    /// The human document title (`self.class.type[:title]`).
    fn title(self) -> Option<&'static str> {
        match self {
            AmcaKind::Standard => Some("Standard"),
            AmcaKind::Publication => Some("Publication"),
            // The interpretation renders its own shape, no title.
            AmcaKind::Interpretation => None,
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        match tag {
            "pubid:amca:standard" => Some(AmcaKind::Standard),
            "pubid:amca:publication" => Some(AmcaKind::Publication),
            "pubid:amca:interpretation" => Some(AmcaKind::Interpretation),
            _ => None,
        }
    }
}

/// The amca identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmcaIdentifier {
    /// The concrete class (drives `_type`, title, and URN key).
    pub kind: AmcaKind,
    /// The copublisher (`AMCA` / `ANSI/AMCA`); always captured.
    pub copublisher: String,
    /// The document number (the grammar's `code`).
    pub number: String,
    /// The bare edition year (`"16"`, `"02"`).
    pub year: Option<String>,
    /// The unbound trailing suffix, when a future binding carries one.
    pub suffix: Option<String>,
    /// The reaffirmation year (`R2008` -> `"2008"`).
    pub reaffirmed: Option<String>,
    /// The publication revision (`Rev. 01-23` -> `"01-23"`).
    pub revision: Option<String>,
    /// The interpretation code (`JW`, `KB`, or a number).
    pub interpretation_code: Option<String>,
}

impl AmcaIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and dispatch the
    /// class (publication -> interpretation -> standard).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = amca_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let capture = |name: &str| -> Option<String> {
            optional_capture(&bound, name)
                .map(str::to_string)
                .or_else(|| {
                    bound
                        .get("interpretation")
                        .and_then(|wrapper| optional_capture(wrapper, name).map(str::to_string))
                })
        };

        let kind = if bound.get("publication_keyword").is_some() || capture("revision").is_some() {
            AmcaKind::Publication
        } else if bound.get("interpretation").is_some()
            || bound.get("interpretation_code").is_some()
            || bound.get("interpretation_year").is_some()
        {
            AmcaKind::Interpretation
        } else {
            AmcaKind::Standard
        };

        Ok(Self {
            kind,
            copublisher: capture("copublisher").ok_or(Error::MissingAttribute {
                capture: "copublisher",
            })?,
            number: capture("code").ok_or(Error::MissingAttribute { capture: "code" })?,
            year: capture("year").or(capture("interpretation_year")),
            suffix: None,
            reaffirmed: capture("reaffirmed"),
            revision: capture("revision"),
            interpretation_code: capture("interpretation_code"),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(AmcaKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown amca _type {:?}", hash.get("_type")),
            })?;
        let _ = check_type(hash, kind.type_tag());
        Ok(Self {
            kind,
            copublisher: hash_str(hash, "copublisher")?.to_string(),
            number: hash_str(hash, "number")?.to_string(),
            year: optional_hash_str(hash, "year").map(str::to_string),
            suffix: optional_hash_str(hash, "suffix").map(str::to_string),
            reaffirmed: optional_hash_str(hash, "reaffirmed").map(str::to_string),
            revision: optional_hash_str(hash, "revision").map(str::to_string),
            interpretation_code: optional_hash_str(hash, "interpretation_code").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the mapped attributes
    /// (the default publisher never serializes; absent optionals drop).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("copublisher".into(), self.copublisher.clone().into());
        object.insert("number".into(), self.number.clone().into());
        for (name, value) in [
            ("year", &self.year),
            ("suffix", &self.suffix),
            ("reaffirmed", &self.reaffirmed),
            ("revision", &self.revision),
            ("interpretation_code", &self.interpretation_code),
        ] {
            if let Some(value) = value {
                object.insert(name.into(), value.clone().into());
            }
        }
        Value::Object(object)
    }

    /// The canonical human representation (three render shapes, per
    /// `Pubid::Amca::Renderer`).
    pub fn to_human(&self) -> String {
        match self.kind {
            AmcaKind::Interpretation => {
                let mut result = format!("{} {}", self.copublisher, self.number);
                if let Some(code) = &self.interpretation_code {
                    result.push_str(&format!(" {code} Interp"));
                } else if let Some(year) = &self.year {
                    // The Ruby renderer prints an EN DASH before a
                    // year-only interpretation.
                    result.push_str(&format!(" \u{2013} {year}"));
                } else {
                    result.push_str(" Interp");
                }
                if let Some(suffix) = &self.suffix {
                    result.push_str(&format!(" {suffix}"));
                }
                result
            }
            kind => {
                let mut result = [
                    self.copublisher.as_str(),
                    kind.title().unwrap_or(""),
                    &self.number,
                ]
                .iter()
                .filter(|part| !part.is_empty())
                .map(|part| part.to_string())
                .collect::<Vec<_>>()
                .join(" ");
                if let Some(year) = &self.year {
                    result.push_str(&format!("-{year}"));
                }
                if let Some(revision) = &self.revision {
                    result.push_str(&format!(" (Rev. {revision})"));
                }
                if let Some(reaffirmed) = &self.reaffirmed {
                    result.push_str(&format!(" (R{reaffirmed})"));
                }
                result
            }
        }
    }

    /// The URN: `urn:amca:<number>[:<year>][:rev.<revision>]
    /// [:reaff.<year>]:copub.<copublisher>:<type key>` (the generator's
    /// segment order; the interpretation code rides between year and
    /// revision as `interp.<code>`).
    pub fn to_urn(&self) -> String {
        let mut parts = vec!["urn".to_string(), "amca".to_string(), self.number.clone()];
        if let Some(year) = &self.year {
            parts.push(year.clone());
        }
        if let Some(suffix) = &self.suffix {
            parts.push(suffix.to_lowercase());
        }
        if let Some(code) = &self.interpretation_code {
            parts.push(format!("interp.{}", code.to_lowercase()));
        }
        if let Some(revision) = &self.revision {
            parts.push(format!("rev.{revision}"));
        }
        if let Some(reaffirmed) = &self.reaffirmed {
            parts.push(format!("reaff.{reaffirmed}"));
        }
        parts.push(format!("copub.{}", self.copublisher.to_lowercase()));
        parts.push(self.kind.urn_key().to_string());
        parts.join(":")
    }
}

/// Read an optional string capture from a bound node (the wrapper may
/// be a nested object or, for a single capture, a bare string).
fn optional_capture<'a>(bound: &'a Value, name: &str) -> Option<&'a str> {
    bound.get(name).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::AmcaIdentifier;

    #[test]
    fn standard_renders_title_year_and_reaffirmation() {
        let id = AmcaIdentifier::parse("AMCA Standard 803-02 (R2008)").expect("parse");
        assert_eq!(id.kind, super::AmcaKind::Standard);
        assert_eq!(id.number, "803");
        assert_eq!(id.year.as_deref(), Some("02"));
        assert_eq!(id.reaffirmed.as_deref(), Some("2008"));
        assert_eq!(id.to_human(), "AMCA Standard 803-02 (R2008)");
        assert_eq!(
            id.to_urn(),
            "urn:amca:803:02:reaff.2008:copub.amca:standard"
        );
    }

    #[test]
    fn publication_carries_the_revision_before_the_reaffirmation() {
        let id = AmcaIdentifier::parse("AMCA Publication 211-22 (Rev. 01-23)").expect("parse");
        assert_eq!(id.kind, super::AmcaKind::Publication);
        assert_eq!(id.revision.as_deref(), Some("01-23"));
        assert_eq!(id.to_human(), "AMCA Publication 211-22 (Rev. 01-23)");
        assert_eq!(
            id.to_urn(),
            "urn:amca:211:22:rev.01-23:copub.amca:publication"
        );
    }

    #[test]
    fn interpretation_with_code_downcases_into_the_urn() {
        let id = AmcaIdentifier::parse("AMCA 204 AW Interp").expect("parse");
        assert_eq!(id.kind, super::AmcaKind::Interpretation);
        assert_eq!(id.interpretation_code.as_deref(), Some("AW"));
        assert_eq!(id.to_human(), "AMCA 204 AW Interp");
        assert_eq!(
            id.to_urn(),
            "urn:amca:204:interp.aw:copub.amca:interpretation"
        );
    }

    #[test]
    fn ansi_copublisher_joins_the_human_and_the_urn() {
        let id = AmcaIdentifier::parse("ANSI/AMCA Standard 204-20").expect("parse");
        assert_eq!(id.copublisher, "ANSI/AMCA");
        assert_eq!(id.to_human(), "ANSI/AMCA Standard 204-20");
        assert_eq!(id.to_urn(), "urn:amca:204:20:copub.ansi/amca:standard");
    }

    #[test]
    fn round_trips_through_the_hash() {
        let id = AmcaIdentifier::parse("AMCA Publication 1011-03 (R2010)").expect("parse");
        let rebuilt = AmcaIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt, id);
        assert_eq!(rebuilt.to_hash(), id.to_hash());
    }
}
