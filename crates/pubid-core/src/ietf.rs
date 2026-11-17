//! The ietf flavor: RFC, the BCP/STD/FYI sub-series, Internet-Drafts.
//!
//! Three flat families keyed by the leading token: RFC (incl. the
//! zero-padded space-less `RFC0001` spelling rfc-index.xml emits — the
//! builder strips the pad), the BCP/STD/FYI sub-series (series derived
//! from the class, never stored — `_type` already carries it), and
//! Internet-Drafts (slug including `draft-` in `number`, optional
//! trailing two-digit `version`). URNs: `urn:ietf:rfc:<n>` /
//! `urn:ietf:{bcp|std|fyi}:<n>` / `urn:ietf:id:<slug>[:<version>]`.

use serde_json::Value;

use crate::artifact::ietf_artifact;
use crate::bound::{bound_str, hash_str, optional_bound_str, optional_hash_str, unpad};
use crate::error::Result;

/// The IETF identifier families, one per `_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IetfKind {
    /// `RFC <n>`.
    Rfc,
    /// `BCP <n>`.
    Bcp,
    /// `STD <n>`.
    Std,
    /// `FYI <n>`.
    Fyi,
    /// A draft slug (`draft-...[-NN]`).
    InternetDraft,
}

impl IetfKind {
    /// The `_type` tag suffix (`rfc`, `bcp`, `std`, `fyi`,
    /// `internet-draft`).
    pub fn tag(self) -> &'static str {
        match self {
            IetfKind::Rfc => "rfc",
            IetfKind::Bcp => "bcp",
            IetfKind::Std => "std",
            IetfKind::Fyi => "fyi",
            IetfKind::InternetDraft => "internet-draft",
        }
    }

    /// The printed series token (`RFC`, `BCP`, `STD`, `FYI`; drafts
    /// print their slug bare).
    fn token(self) -> &'static str {
        match self {
            IetfKind::Rfc => "RFC",
            IetfKind::Bcp => "BCP",
            IetfKind::Std => "STD",
            IetfKind::Fyi => "FYI",
            IetfKind::InternetDraft => "",
        }
    }

    /// The kind for a `_type` tag suffix.
    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "rfc" => Some(IetfKind::Rfc),
            "bcp" => Some(IetfKind::Bcp),
            "std" => Some(IetfKind::Std),
            "fyi" => Some(IetfKind::Fyi),
            "internet-draft" => Some(IetfKind::InternetDraft),
            _ => None,
        }
    }
}

/// The ietf identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IetfIdentifier {
    /// The family (carried by `_type`, never a separate attribute).
    pub kind: IetfKind,
    /// The number, unpadded (`RFC0001` binds `1`); drafts carry the
    /// full slug including the `draft-` prefix.
    pub number: String,
    /// The draft's trailing two-digit version.
    pub version: Option<String>,
}

impl IetfIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// The polymorphic type tag for this identifier's family.
    pub fn type_tag(&self) -> String {
        format!("pubid:ietf:{}", self.kind.tag())
    }

    /// Parse an input through the vendored artifact (bound hash).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ietf_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        if let Some(draft_rest) = optional_bound_str(&bound, "draft_rest") {
            let full = format!("draft-{draft_rest}");
            let (slug, version) = split_draft_version(&full);
            return Ok(Self {
                kind: IetfKind::InternetDraft,
                number: slug.to_string(),
                version: version.map(str::to_string),
            });
        }
        if let Some(series) = optional_bound_str(&bound, "series") {
            let kind = match series {
                "BCP" => IetfKind::Bcp,
                "STD" => IetfKind::Std,
                "FYI" => IetfKind::Fyi,
                other => {
                    return Err(crate::error::Error::InvalidHash {
                        detail: format!("unknown ietf series {other:?}"),
                    })
                }
            };
            return Ok(Self {
                kind,
                number: unpad(bound_str(&bound, "number")?).to_string(),
                version: None,
            });
        }
        Ok(Self {
            kind: IetfKind::Rfc,
            number: unpad(bound_str(&bound, "number")?).to_string(),
            version: None,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let tag = hash
            .get("_type")
            .and_then(Value::as_str)
            .ok_or_else(|| crate::error::Error::InvalidHash {
                detail: "missing _type".to_string(),
            })?
            .strip_prefix("pubid:ietf:")
            .ok_or_else(|| crate::error::Error::InvalidHash {
                detail: "_type is not an ietf tag".to_string(),
            })?;
        let kind = IetfKind::from_tag(tag).ok_or_else(|| crate::error::Error::InvalidHash {
            detail: format!("unknown ietf _type {tag:?}"),
        })?;
        Ok(Self {
            kind,
            number: hash_str(hash, "number")?.to_string(),
            version: optional_hash_str(hash, "version").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(version) = &self.version {
            object.insert("version".into(), version.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human representation.
    ///
    /// Fails loudly on an empty number rather than rendering `RFC `
    /// (the pubid-ts `requireNumber` guard).
    pub fn to_human(&self) -> String {
        let number = self.require_number();
        match self.kind {
            IetfKind::InternetDraft => match &self.version {
                Some(version) => format!("{number}-{version}"),
                None => number.to_string(),
            },
            kind => format!("{} {number}", kind.token()),
        }
    }

    /// The URN: `urn:ietf:{rfc|bcp|std|fyi}:<n>` or
    /// `urn:ietf:id:<slug>[:<version>]`.
    pub fn to_urn(&self) -> String {
        let number = self.require_number();
        match self.kind {
            IetfKind::InternetDraft => match &self.version {
                Some(version) => format!("urn:ietf:id:{number}:{version}"),
                None => format!("urn:ietf:id:{number}"),
            },
            kind => format!("urn:ietf:{}:{number}", kind.tag()),
        }
    }

    /// Fail loudly on a nil number rather than keying the index under
    /// an empty string.
    fn require_number(&self) -> &str {
        if self.number.is_empty() {
            panic!("ietf identifier has an empty number");
        }
        &self.number
    }
}

/// A draft version is a trailing `-NN` of exactly two digits at the
/// very end: a three-digit topic tail (`…-256`) keeps its
/// char-before-last-two a digit, not `-`, so it stays in the slug.
fn split_draft_version(full: &str) -> (&str, Option<&str>) {
    let bytes = full.as_bytes();
    if full.len() > 3
        && bytes[bytes.len() - 3] == b'-'
        && bytes[bytes.len() - 2..]
            .iter()
            .all(|byte| byte.is_ascii_digit())
    {
        let slug = &full[..full.len() - 3];
        let version = &full[full.len() - 2..];
        return (slug, Some(version));
    }
    (full, None)
}

#[cfg(test)]
mod tests {
    use super::{IetfIdentifier, IetfKind};
    use serde_json::json;

    #[test]
    fn parses_rfc_with_and_without_padding() {
        let spaced = IetfIdentifier::parse("RFC 2616").expect("parse");
        assert_eq!(spaced.kind, IetfKind::Rfc);
        assert_eq!(spaced.number, "2616");
        assert_eq!(
            spaced.to_hash(),
            json!({"_type": "pubid:ietf:rfc", "number": "2616"})
        );

        let padded = IetfIdentifier::parse("RFC0001").expect("parse");
        assert_eq!(padded.number, "1");
    }

    #[test]
    fn parses_subseries_by_leading_token() {
        for (input, kind, token, number) in [
            ("BCP 14", IetfKind::Bcp, "BCP", "14"),
            ("STD0066", IetfKind::Std, "STD", "66"),
            ("FYI 3", IetfKind::Fyi, "FYI", "3"),
        ] {
            let id = IetfIdentifier::parse(input).expect("parse");
            assert_eq!(id.kind, kind, "{input}");
            assert_eq!(id.number, number, "{input}");
            assert_eq!(
                id.to_hash()["_type"],
                format!("pubid:ietf:{}", kind.tag()),
                "{input}"
            );
            // The human is always the normalized spaced spelling.
            assert_eq!(id.to_human(), format!("{token} {number}"), "{input}");
            assert_eq!(
                id.to_urn(),
                format!("urn:ietf:{}:{number}", kind.tag()),
                "{input}"
            );
        }
    }

    #[test]
    fn splits_draft_versions_but_keeps_three_digit_tails() {
        let versioned = IetfIdentifier::parse("draft-ietf-httpbis-messaging-19").expect("parse");
        assert_eq!(versioned.number, "draft-ietf-httpbis-messaging");
        assert_eq!(versioned.version.as_deref(), Some("19"));
        assert_eq!(versioned.to_human(), "draft-ietf-httpbis-messaging-19");
        assert_eq!(
            versioned.to_urn(),
            "urn:ietf:id:draft-ietf-httpbis-messaging:19"
        );

        let tailed = IetfIdentifier::parse("draft-something-256").expect("parse");
        assert_eq!(tailed.number, "draft-something-256");
        assert_eq!(tailed.version, None);
    }
}
