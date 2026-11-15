//! The doi flavor: DOIs per ISO 26324.
//!
//! The artifact carries the grammar and the `prefix`/`suffix`
//! bindings; the render and URN sides live here, mirroring the
//! pubid-ts doi flavor: the Ruby gem has no doi urn_generator, so
//! `to_urn` resolves to the base template (`urn:doi`), and the
//! canonical human is `doi:{prefix}/{suffix}` with a lowercase scheme.

use serde_json::Value;

use crate::artifact::doi_artifact;
use crate::error::{Error, Result};

/// The doi identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoiIdentifier {
    /// The DOI registrar prefix, e.g. `10.1000`.
    pub prefix: String,
    /// The DOI suffix, e.g. `182`.
    pub suffix: String,
}

impl DoiIdentifier {
    /// The polymorphic type tag carried first in the canonical hash.
    pub const TYPE: &'static str = "pubid:doi:resource";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact (bound hash).
    ///
    /// The artifact's `prefix` capture binds the digits after the
    /// grammar's literal `10.`; the canonical DOI prefix per ISO 26324
    /// is `10.{digits}`, so the flavor layer casts it (mirrors the
    /// pubid-ts doi builder's `prefix_digits` cast).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = doi_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        let digits = bound
            .get("prefix")
            .and_then(Value::as_str)
            .ok_or(Error::MissingAttribute { capture: "prefix" })?;
        let suffix = bound
            .get("suffix")
            .and_then(Value::as_str)
            .ok_or(Error::MissingAttribute { capture: "suffix" })?;
        Ok(Self {
            prefix: format!("10.{digits}"),
            suffix: suffix.to_string(),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::InvalidHash {
                detail: "missing _type".to_string(),
            })?;
        if kind != Self::TYPE {
            return Err(Error::InvalidHash {
                detail: "_type is not pubid:doi:resource".to_string(),
            });
        }
        let capture = |name: &'static str| -> Result<&str> {
            hash.get(name)
                .and_then(Value::as_str)
                .ok_or(Error::MissingAttribute { capture: name })
        };
        Ok(Self {
            prefix: capture("prefix")?.to_string(),
            suffix: capture("suffix")?.to_string(),
        })
    }

    /// The canonical hash: `_type` first, then the attributes.
    pub fn to_hash(&self) -> Value {
        serde_json::json!({
            "_type": Self::TYPE,
            "prefix": self.prefix,
            "suffix": self.suffix,
        })
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        format!("doi:{}/{}", self.prefix, self.suffix)
    }

    /// The URN (base template: no doi attribute hits a slot).
    pub fn to_urn(&self) -> String {
        "urn:doi".to_string()
    }
}
