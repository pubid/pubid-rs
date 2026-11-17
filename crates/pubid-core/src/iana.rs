//! The iana flavor: registry slugs.
//!
//! The artifact binds the top-level slug as `registry` and the optional
//! sub-slug as `sub_registry`; the flavor layer renames `registry` to
//! the `number` attribute (the pubid-ts iana builder's cast). The URN
//! is the Ruby flavor's own generator: `urn:iana:<slug>[:<sub-slug>]`.

use serde_json::Value;

use crate::artifact::iana_artifact;
use crate::bound::{bound_str, check_type, hash_str, optional_bound_str, optional_hash_str};
use crate::error::Result;

/// The iana identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IanaIdentifier {
    /// The top-level registry slug (the bound `registry` capture).
    pub number: String,
    /// The optional sub-registry slug (the wire name is `sub_registry`).
    pub sub_registry: Option<String>,
}

impl IanaIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:iana:registry";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact (bound hash).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = iana_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Ok(Self {
            number: bound_str(&bound, "registry")?.to_string(),
            sub_registry: optional_bound_str(&bound, "sub_registry").map(str::to_string),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        Ok(Self {
            number: hash_str(hash, "number")?.to_string(),
            sub_registry: optional_hash_str(hash, "sub_registry").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(sub_registry) = &self.sub_registry {
            object.insert("sub_registry".into(), sub_registry.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        match &self.sub_registry {
            Some(sub_registry) => format!("IANA {}/{}", self.number, sub_registry),
            None => format!("IANA {}", self.number),
        }
    }

    /// The URN: `urn:iana:<slug>[:<sub-slug>]`.
    pub fn to_urn(&self) -> String {
        match &self.sub_registry {
            Some(sub_registry) => format!("urn:iana:{}:{}", self.number, sub_registry),
            None => format!("urn:iana:{}", self.number),
        }
    }
}
