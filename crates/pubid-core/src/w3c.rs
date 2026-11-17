//! The w3c flavor: `W3C [TYPE-]slug[-date]`, one kind per W3C maturity
//! level.
//!
//! The artifact splits only the maturity token (`type`, optional) from
//! the rest; the flavor layer applies the fixed-width trailing-date
//! split (8/6/4 digits — slugs like `url-1` or `DSIG-label-971024.html`
//! never mis-split) and picks the kind. `date` is a plain string (never
//! a date component, exactly like Ruby). URN:
//! `urn:w3c:[<token-lowercase>:]<number>[:<date>]`.

use serde_json::Value;

use crate::artifact::w3c_artifact;
use crate::bound::{bound_str, hash_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// The eleven W3C maturity kinds, one per `_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W3cKind {
    Standard,
    Note,
    DraftNote,
    WorkingDraft,
    CandidateRecommendation,
    CandidateRecommendationDraft,
    Recommendation,
    ProposedRecommendation,
    ProposedEditedRecommendation,
    SupersededRecommendation,
    ObsoleteRecommendation,
}

impl W3cKind {
    /// The `_type` tag suffix (kebab-case, after `pubid:w3c:`).
    pub fn tag(self) -> &'static str {
        match self {
            W3cKind::Standard => "standard",
            W3cKind::Note => "note",
            W3cKind::DraftNote => "draft-note",
            W3cKind::WorkingDraft => "working-draft",
            W3cKind::CandidateRecommendation => "candidate-recommendation",
            W3cKind::CandidateRecommendationDraft => "candidate-recommendation-draft",
            W3cKind::Recommendation => "recommendation",
            W3cKind::ProposedRecommendation => "proposed-recommendation",
            W3cKind::ProposedEditedRecommendation => "proposed-edited-recommendation",
            W3cKind::SupersededRecommendation => "superseded-recommendation",
            W3cKind::ObsoleteRecommendation => "obsolete-recommendation",
        }
    }

    /// The printed token; `standard` prints bare.
    fn token(self) -> Option<&'static str> {
        match self {
            W3cKind::Standard => None,
            W3cKind::Note => Some("NOTE"),
            W3cKind::DraftNote => Some("DNOTE"),
            W3cKind::WorkingDraft => Some("WD"),
            W3cKind::CandidateRecommendation => Some("CR"),
            W3cKind::CandidateRecommendationDraft => Some("CRD"),
            W3cKind::Recommendation => Some("REC"),
            W3cKind::ProposedRecommendation => Some("PR"),
            W3cKind::ProposedEditedRecommendation => Some("PER"),
            W3cKind::SupersededRecommendation => Some("SPSD"),
            W3cKind::ObsoleteRecommendation => Some("OBSL"),
        }
    }

    /// The kind for a printed token (DNOTE before NOTE, CRD before CR,
    /// PER before PR — shared prefixes are all distinct tokens here, so
    /// exact-match lookup is total).
    fn from_token(token: &str) -> Option<Self> {
        W3C_KINDS
            .iter()
            .copied()
            .find(|kind| kind.token() == Some(token))
    }

    /// The kind for a `_type` tag suffix.
    fn from_tag(tag: &str) -> Option<Self> {
        W3C_KINDS.iter().copied().find(|kind| kind.tag() == tag)
    }
}

/// Every kind, in tag order.
const W3C_KINDS: &[W3cKind] = &[
    W3cKind::Standard,
    W3cKind::Note,
    W3cKind::DraftNote,
    W3cKind::WorkingDraft,
    W3cKind::CandidateRecommendation,
    W3cKind::CandidateRecommendationDraft,
    W3cKind::Recommendation,
    W3cKind::ProposedRecommendation,
    W3cKind::ProposedEditedRecommendation,
    W3cKind::SupersededRecommendation,
    W3cKind::ObsoleteRecommendation,
];

/// The w3c identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W3cIdentifier {
    /// The maturity kind (carried by `_type`, never an attribute).
    pub kind: W3cKind,
    /// The slug (everything between the type token and the date).
    pub number: String,
    /// The trailing fixed-width date (`\d{8}` / `\d{6}` / `\d{4}`).
    pub date: Option<String>,
}

impl W3cIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// The polymorphic type tag for this identifier's kind.
    pub fn type_tag(&self) -> String {
        format!("pubid:w3c:{}", self.kind.tag())
    }

    /// Parse an input through the vendored artifact, then split the
    /// trailing date (the builder-side half of the port).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = w3c_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        let kind = match optional_bound_str(&bound, "type") {
            Some(token) => W3cKind::from_token(token).ok_or_else(|| Error::InvalidHash {
                detail: format!("W3C: unknown type token {token:?}"),
            })?,
            None => W3cKind::Standard,
        };
        let (number, date) = split_date(bound_str(&bound, "rest")?);
        Ok(Self {
            kind,
            number: number.to_string(),
            date: date.map(str::to_string),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let tag = hash
            .get("_type")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::InvalidHash {
                detail: "missing _type".to_string(),
            })?
            .strip_prefix("pubid:w3c:")
            .ok_or_else(|| Error::InvalidHash {
                detail: "_type is not a w3c tag".to_string(),
            })?;
        let kind = W3cKind::from_tag(tag).ok_or_else(|| Error::InvalidHash {
            detail: format!("unknown w3c _type {tag:?}"),
        })?;
        Ok(Self {
            kind,
            number: hash_str(hash, "number")?.to_string(),
            date: optional_hash_str(hash, "date").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(date) = &self.date {
            object.insert("date".into(), date.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        let mut body = String::new();
        if let Some(token) = self.kind.token() {
            body.push_str(token);
            body.push('-');
        }
        body.push_str(&self.number);
        if let Some(date) = &self.date {
            body.push('-');
            body.push_str(date);
        }
        format!("W3C {body}")
    }

    /// The URN: `urn:w3c:[<token-lowercase>:]<number>[:<date>]`.
    pub fn to_urn(&self) -> String {
        let mut parts = vec!["urn".to_string(), "w3c".to_string()];
        if let Some(token) = self.kind.token() {
            parts.push(token.to_lowercase());
        }
        parts.push(self.number.clone());
        if let Some(date) = &self.date {
            parts.push(date.clone());
        }
        parts.join(":")
    }
}

/// The trailing-date split: `(.+)-(\d{8}|\d{6}|\d{4})$` — the greedy
/// lead means the RIGHTMOST dash is tried first, and at each dash the
/// widths in 8/6/4 order (mirrors the regex backtracking, so a slug
/// like `url-1` never mis-splits but `html40-19980424` does).
fn split_date(rest: &str) -> (&str, Option<&str>) {
    let widths = [8usize, 6, 4];
    let bytes = rest.as_bytes();
    // Index 0 is excluded: the regex lead `(.+)` needs at least one
    // character before the dash.
    for index in (1..bytes.len()).rev() {
        if bytes[index] == b'-' {
            for width in widths {
                if is_fixed_digits(&rest[index + 1..], width) {
                    return (&rest[..index], Some(&rest[index + 1..]));
                }
            }
        }
    }
    (rest, None)
}

/// Exactly `width` ASCII digits.
fn is_fixed_digits(text: &str, width: usize) -> bool {
    text.len() == width && text.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{split_date, W3cIdentifier, W3cKind};
    use serde_json::json;

    #[test]
    fn splits_dates_at_the_fixed_widths() {
        assert_eq!(split_date("html40-19980424"), ("html40", Some("19980424")));
        assert_eq!(split_date("TPRC-970930"), ("TPRC", Some("970930")));
        // 4 digits are a legacy MMDD date, so this splits (Ruby parity).
        assert_eq!(split_date("x-1234"), ("x", Some("1234")));
        // Shorter runs (and digit runs ending in non-digits) never split.
        assert_eq!(split_date("url-1"), ("url-1", None));
        assert_eq!(
            split_date("DSIG-label-971024.html"),
            ("DSIG-label-971024.html", None)
        );
        assert_eq!(split_date("07-WebData"), ("07-WebData", None));
        // Greedy-lead backtracking: the rightmost dash wins even when a
        // lefter dash carries a wider digit run.
        assert_eq!(split_date("x-123456-1234"), ("x-123456", Some("1234")));
    }

    #[test]
    fn parses_bare_and_typed_spellings() {
        let standard = W3cIdentifier::parse("W3C 07-WebData").expect("parse");
        assert_eq!(standard.kind, W3cKind::Standard);
        assert_eq!(standard.number, "07-WebData");
        assert_eq!(
            standard.to_hash(),
            json!({"_type": "pubid:w3c:standard", "number": "07-WebData"})
        );

        let note = W3cIdentifier::parse("W3C NOTE-TPRC-970930").expect("parse");
        assert_eq!(note.kind, W3cKind::Note);
        assert_eq!(note.number, "TPRC");
        assert_eq!(note.date.as_deref(), Some("970930"));
        assert_eq!(note.to_human(), "W3C NOTE-TPRC-970930");
        assert_eq!(note.to_urn(), "urn:w3c:note:TPRC:970930");

        let rec = W3cIdentifier::parse("W3C REC-html40-19980424").expect("parse");
        assert_eq!(rec.kind, W3cKind::Recommendation);
        assert_eq!(rec.to_urn(), "urn:w3c:rec:html40:19980424");
    }

    #[test]
    fn every_kind_round_trips_through_the_hash() {
        for kind in super::W3C_KINDS {
            let id = W3cIdentifier {
                kind: *kind,
                number: "slug".to_string(),
                date: Some("20250101".to_string()),
            };
            let rebuilt = W3cIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id);
        }
    }
}
