//! The gb flavor: Chinese national standards (GB / GB/T / GB/Z and
//! the sector faces).
//!
//! The artifact binds the whole `publisher_code` (`GB/T`, `GBn`,
//! `HB/Z`, `T/GZAEPI`); the flavor layer splits the trailing `/T` /
//! `/Z` mandate off (a code that ends in any other slash token —
//! `T/GZAEPI` — stays whole as the publisher). The all-parts form
//! `GB/T 5606 (all parts)` binds the same captures as the bare
//! standard, so the marker is detected on the input spelling
//! (ingestion-side, like `Pubid::Api`'s MPMP normalization) and the
//! member standard hangs off the wire `identifiers` array.

use serde_json::Value;

use crate::artifact::gb_artifact;
use crate::bound::{bound_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// The member standard of the gb flavor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GbStandard {
    /// The publisher face (`GB`, `GBn`, `HB`, `JB`, `T/GZAEPI`).
    pub publisher: String,
    /// The trailing mandate (`T` in `GB/T`, `Z` in `HB/Z`).
    pub mandate: Option<String>,
    /// The document number.
    pub number: String,
    /// The dotted part (`1` in `5606.1`).
    pub part: Option<String>,
    /// The dash-form year (`2004`).
    pub year: Option<String>,
}

impl GbStandard {
    /// The member type tag (the nested `identifiers` entries).
    pub const TYPE: &'static str = "pubid:gb:standard";

    fn from_bound(bound: &Value) -> Result<Self> {
        let (publisher, mandate) = split_publisher_code(bound_str(bound, "publisher_code")?);
        Ok(Self {
            publisher: publisher.to_string(),
            mandate: mandate.map(str::to_string),
            number: bound_str(bound, "number")?.to_string(),
            part: optional_bound_str(bound, "part").map(str::to_string),
            year: optional_bound_str(bound, "year").map(str::to_string),
        })
    }

    fn from_hash(hash: &Value) -> Result<Self> {
        Ok(Self {
            publisher: crate::bound::hash_str(hash, "publisher")?.to_string(),
            mandate: optional_hash_str(hash, "mandate").map(str::to_string),
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            part: optional_hash_str(hash, "part").map(str::to_string),
            year: optional_hash_str(hash, "year").map(str::to_string),
        })
    }

    fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("publisher".into(), self.publisher.clone().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(mandate) = &self.mandate {
            object.insert("mandate".into(), mandate.clone().into());
        }
        if let Some(part) = &self.part {
            object.insert("part".into(), part.clone().into());
        }
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        Value::Object(object)
    }

    /// The member human: `GB/T 5606.1-2004`.
    fn to_human(&self) -> String {
        let mut out = self.publisher.clone();
        if let Some(mandate) = &self.mandate {
            out.push('/');
            out.push_str(mandate);
        }
        out.push(' ');
        out.push_str(&self.number);
        if let Some(part) = &self.part {
            out.push('.');
            out.push_str(part);
        }
        if let Some(year) = &self.year {
            out.push('-');
            out.push_str(year);
        }
        out
    }

    /// The member URN: `urn:gb:<pub>:<number>[:-<part>][:<year>]`.
    fn to_urn(&self) -> String {
        let mut out = format!("urn:gb:{}:{}", self.publisher.to_lowercase(), self.number);
        if let Some(part) = &self.part {
            out.push_str(&format!(":-{part}"));
        }
        if let Some(year) = &self.year {
            out.push_str(&format!(":{year}"));
        }
        out
    }
}

/// The gb identifier: a standard, or the all-parts wrapper over one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GbIdentifier {
    /// The concrete class (drives `_type` and the render shape).
    pub kind: GbKind,
    /// The member standard (the wire `identifiers` entry; the corpus
    /// all-parts rows carry exactly one).
    pub standard: GbStandard,
}

/// What concrete gb class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GbKind {
    AllParts,
    Standard,
}

use GbKind::*;

impl GbKind {
    /// The shared `_type` prefix every gb tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:gb:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            AllParts => "pubid:gb:all-parts",
            Standard => "pubid:gb:standard",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [AllParts, Standard]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }
}

impl GbIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact; the all-parts
    /// marker is read off the input spelling (the grammar drops it).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = gb_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        let standard = GbStandard::from_bound(&bound)?;
        let kind = if input.contains("(all parts)") {
            AllParts
        } else {
            Standard
        };
        Ok(Self { kind, standard })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record;
    /// the all-parts wrapper carries the nested `identifiers` array).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(GbKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown gb _type {:?}", hash.get("_type")),
            })?;
        let standard = match kind {
            Standard => GbStandard::from_hash(hash)?,
            AllParts => {
                let members = hash.get("identifiers").and_then(Value::as_array).ok_or(
                    Error::InvalidHash {
                        detail: "all-parts has no identifiers array".to_string(),
                    },
                )?;
                let first = members.first().ok_or(Error::InvalidHash {
                    detail: "all-parts identifiers is empty".to_string(),
                })?;
                GbStandard::from_hash(first)?
            }
        };
        Ok(Self { kind, standard })
    }

    /// The canonical hash: the standard's attributes in place, or the
    /// nested `identifiers` array under the all-parts tag.
    pub fn to_hash(&self) -> Value {
        match self.kind {
            Standard => self.standard.to_hash(),
            AllParts => serde_json::json!({
                "_type": AllParts.type_tag(),
                "identifiers": [self.standard.to_hash()],
            }),
        }
    }

    /// The canonical human; the all-parts wrapper appends the marker.
    pub fn to_human(&self) -> String {
        match self.kind {
            Standard => self.standard.to_human(),
            AllParts => format!("{} (all parts)", self.standard.to_human()),
        }
    }

    /// The URN (the wrapper renders its member's URN).
    pub fn to_urn(&self) -> String {
        self.standard.to_urn()
    }
}

/// Split the captured `publisher_code`: a trailing `/T` or `/Z` is the
/// mandate; any other slash token stays whole (`T/GZAEPI`).
fn split_publisher_code(code: &str) -> (&str, Option<&str>) {
    match code.rsplit_once('/') {
        Some((publisher, mandate @ ("T" | "Z"))) => (publisher, Some(mandate)),
        _ => (code, None),
    }
}

#[cfg(test)]
mod tests {
    use super::GbIdentifier;
    use crate::GbKind;
    use serde_json::json;

    #[test]
    fn the_plain_standard_renders_the_year_form() {
        let id = GbIdentifier::parse("GB 1234-2010").expect("parse");
        assert_eq!(id.kind, GbKind::Standard);
        assert_eq!(id.standard.publisher, "GB");
        assert_eq!(id.to_human(), "GB 1234-2010");
        assert_eq!(id.to_urn(), "urn:gb:gb:1234:2010");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:gb:standard",
                "publisher": "GB",
                "number": "1234",
                "year": "2010",
            })
        );
    }

    #[test]
    fn the_mandate_splits_off_the_publisher_code() {
        let id = GbIdentifier::parse("GB/T 5606.1-2004").expect("parse");
        assert_eq!(id.standard.mandate.as_deref(), Some("T"));
        assert_eq!(id.standard.part.as_deref(), Some("1"));
        assert_eq!(id.to_human(), "GB/T 5606.1-2004");
        assert_eq!(id.to_urn(), "urn:gb:gb:5606:-1:2004");

        let z = GbIdentifier::parse("HB/Z 12-2010").expect("parse");
        assert_eq!(z.standard.publisher, "HB");
        assert_eq!(z.standard.mandate.as_deref(), Some("Z"));
        assert_eq!(z.to_human(), "HB/Z 12-2010");
    }

    #[test]
    fn a_group_standard_code_stays_whole() {
        let id = GbIdentifier::parse("T/GZAEPI 001-2018").expect("parse");
        assert_eq!(id.standard.publisher, "T/GZAEPI");
        assert!(id.standard.mandate.is_none());
        assert_eq!(id.to_human(), "T/GZAEPI 001-2018");
        assert_eq!(id.to_urn(), "urn:gb:t/gzaepi:001:2018");
    }

    #[test]
    fn the_all_parts_wrapper_wraps_the_member_standard() {
        let id = GbIdentifier::parse("GB/T 5606 (all parts)").expect("parse");
        assert_eq!(id.kind, GbKind::AllParts);
        assert_eq!(id.to_human(), "GB/T 5606 (all parts)");
        assert_eq!(id.to_urn(), "urn:gb:gb:5606");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:gb:all-parts",
                "identifiers": [{
                    "_type": "pubid:gb:standard",
                    "publisher": "GB",
                    "number": "5606",
                    "mandate": "T",
                }],
            })
        );
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in ["GBn/T 123-1990", "GB/T 5606 (all parts)"] {
            let id = GbIdentifier::parse(spelling).expect(spelling);
            let rebuilt = GbIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id, "{spelling}");
            assert_eq!(rebuilt.to_hash(), id.to_hash());
        }
    }
}
