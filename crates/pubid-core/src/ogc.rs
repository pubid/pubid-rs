//! The ogc flavor: `<yy>-<nnn>[<revision>]` OGC documents.
//!
//! The artifact binds `year`, `number` and the optional `revision`
//! (the optional "OGC " prefix is consumed and discarded). Strings
//! preserve their zero padding; the revision normalizes to lower case
//! (and drops when empty). The URN is the Ruby flavor's generator:
//! `urn:ogc:<year>:<number>[:<revision>]`.

use serde_json::Value;

use crate::artifact::ogc_artifact;
use crate::bound::{bound_str, check_type, hash_str, optional_bound_str, optional_hash_str};
use crate::error::Result;

/// The ogc identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OgcIdentifier {
    /// The two-digit year, zero padding preserved (`06`).
    pub year: String,
    /// The document number, zero padding preserved (`009`).
    pub number: String,
    /// The lower-cased revision (`a`, `r4`), absent when unspelled.
    pub revision: Option<String>,
}

impl OgcIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:ogc:document";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact (bound hash).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ogc_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        // The revision normalizes to lower case; an empty revision is
        // no revision.
        let revision = optional_bound_str(&bound, "revision")
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty());
        Ok(Self {
            year: bound_str(&bound, "year")?.to_string(),
            number: bound_str(&bound, "number")?.to_string(),
            revision,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        Ok(Self {
            year: hash_str(hash, "year")?.to_string(),
            number: hash_str(hash, "number")?.to_string(),
            revision: optional_hash_str(hash, "revision").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("year".into(), self.year.clone().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(revision) = &self.revision {
            object.insert("revision".into(), revision.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human representation (no publisher token, revision
    /// glued on).
    pub fn to_human(&self) -> String {
        match &self.revision {
            Some(revision) => format!("{}-{}{}", self.year, self.number, revision),
            None => format!("{}-{}", self.year, self.number),
        }
    }

    /// The URN: `urn:ogc:<year>:<number>[:<revision>]`.
    pub fn to_urn(&self) -> String {
        match &self.revision {
            Some(revision) => format!("urn:ogc:{}:{}:{}", self.year, self.number, revision),
            None => format!("urn:ogc:{}:{}", self.year, self.number),
        }
    }
}
