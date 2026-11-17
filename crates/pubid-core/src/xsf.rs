//! The xsf flavor: XEP numbers.
//!
//! "XEP NNNN" plus the two named documents (`README`, `xxxx`) kept
//! verbatim — they reach pubid as real primary docids. The URN is the
//! Ruby flavor's own generator: `urn:xsf:xep:<number>`.

use serde_json::Value;

use crate::artifact::xsf_artifact;
use crate::bound::{bound_str, check_type, hash_str};
use crate::error::Result;

/// The xsf identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsfIdentifier {
    /// The XEP number, zero-padded as spelled (`0001`), or one of the
    /// named documents (`README`, `xxxx`).
    pub number: String,
}

impl XsfIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:xsf:xep";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact (bound hash).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = xsf_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Ok(Self {
            number: bound_str(&bound, "number")?.to_string(),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        Ok(Self {
            number: hash_str(hash, "number")?.to_string(),
        })
    }

    /// The canonical hash.
    pub fn to_hash(&self) -> Value {
        serde_json::json!({
            "_type": Self::TYPE,
            "number": self.number,
        })
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        format!("XEP {}", self.number)
    }

    /// The URN: `urn:xsf:xep:<number>`.
    pub fn to_urn(&self) -> String {
        format!("urn:xsf:xep:{}", self.number)
    }
}
