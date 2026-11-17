//! The oasis flavor: OASIS standard slugs.
//!
//! The grammar strips the "OASIS " prefix and captures the slug
//! verbatim as `original`; the flavor layer decomposes it
//! order-independently by classifying WHOLE dash-fragments (version,
//! stage, part — stage tokens are case-sensitive, exactly the
//! pubid-ts/ruby builder rules). `original` alone drives the human
//! form; the URN percent-encodes only the space and stray `]` of
//! malformed records.

use serde_json::Value;

use crate::artifact::oasis_artifact;
use crate::bound::{bound_str, check_type, hash_str, optional_hash_str};
use crate::error::Result;

/// The oasis identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OasisIdentifier {
    /// The slug verbatim (everything after the "OASIS " prefix).
    pub original: String,
    /// The lead fragments before the first classifiable one.
    pub number: Option<String>,
    /// The first `v?N(.N)+` fragment.
    pub version: Option<String>,
    /// The first stage-token fragment (`CSPRD`/`CSD`/`COS`/`CS`/`WD`/
    /// `OS`/`PS`/`PRD`/`CD`/`Errata` + optional digits).
    pub stage: Option<String>,
    /// The first part fragment (`Part`/`Pt`/`part`/`P` + digits).
    pub part: Option<String>,
    /// The remaining unclassifiable fragments after the lead.
    pub label: Option<String>,
}

impl OasisIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:oasis:standard";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact, then decompose
    /// the slug (the builder-side half of the port).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = oasis_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Ok(decompose(bound_str(&bound, "original")?))
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        Ok(Self {
            original: hash_str(hash, "original")?.to_string(),
            number: optional_hash_str(hash, "number").map(str::to_string),
            version: optional_hash_str(hash, "version").map(str::to_string),
            stage: optional_hash_str(hash, "stage").map(str::to_string),
            part: optional_hash_str(hash, "part").map(str::to_string),
            label: optional_hash_str(hash, "label").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("original".into(), self.original.clone().into());
        for (name, value) in [
            ("number", &self.number),
            ("version", &self.version),
            ("stage", &self.stage),
            ("part", &self.part),
            ("label", &self.label),
        ] {
            if let Some(value) = value {
                object.insert(name.into(), value.clone().into());
            }
        }
        Value::Object(object)
    }

    /// The canonical human representation (`original` alone drives it).
    pub fn to_human(&self) -> String {
        format!("OASIS {}", self.original)
    }

    /// The URN: `urn:oasis:<slug>` with space/`]` percent-encoded.
    pub fn to_urn(&self) -> String {
        let slug: String = self
            .original
            .chars()
            .flat_map(|c| match c {
                ' ' => vec!['%', '2', '0'],
                ']' => vec!['%', '5', 'D'],
                other => vec![other],
            })
            .collect();
        format!("urn:oasis:{slug}")
    }
}

/// What kind of dash-fragment a slug piece is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FragmentKind {
    Version,
    Stage,
    Part,
}

/// `v?N(.N)+` — bare integers deliberately are NOT versions.
fn is_version(fragment: &str) -> bool {
    let digits = fragment.strip_prefix(['v', 'V']).unwrap_or(fragment);
    let groups: Vec<&str> = digits.split('.').collect();
    groups.len() >= 2
        && groups
            .iter()
            .all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_digit()))
}

/// The stage tokens, CASE-SENSITIVE, each with an optional digit tail.
const STAGE_TOKENS: &[&str] = &[
    "CSPRD", "CSD", "COS", "CS", "WD", "OS", "PS", "PRD", "CD", "Errata",
];

fn is_stage(fragment: &str) -> bool {
    STAGE_TOKENS.iter().any(|token| {
        fragment
            .strip_prefix(token)
            .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit()))
    })
}

/// The part spellings incl. bare `Pt`/`P` (longer prefixes first).
const PART_TOKENS: &[&str] = &["Part", "Pt", "part", "P"];

fn is_part(fragment: &str) -> bool {
    PART_TOKENS.iter().any(|token| {
        fragment
            .strip_prefix(token)
            .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit()))
    })
}

fn classify(fragment: &str) -> Option<FragmentKind> {
    if is_version(fragment) {
        Some(FragmentKind::Version)
    } else if is_stage(fragment) {
        Some(FragmentKind::Stage)
    } else if is_part(fragment) {
        Some(FragmentKind::Part)
    } else {
        None
    }
}

/// The order-independent decomposition: whole dash-fragments classified
/// one by one; the lead (before the first classifiable) is the `number`,
/// the first fragment of each kind wins its attribute, and the remaining
/// unclassifiable fragments join as the `label`.
fn decompose(original: &str) -> OasisIdentifier {
    let fragments: Vec<&str> = original.split('-').collect();
    let classified: Vec<Option<FragmentKind>> = fragments.iter().map(|f| classify(f)).collect();

    let lead_end = classified
        .iter()
        .position(|kind| kind.is_some())
        .unwrap_or(fragments.len());
    let lead = &fragments[..lead_end];
    let rest = &fragments[lead_end..];
    let rest_kinds = &classified[lead_end..];

    let number = join(lead);
    let first_of = |wanted: FragmentKind| -> Option<String> {
        rest_kinds
            .iter()
            .zip(rest.iter())
            .find(|(kind, _)| **kind == Some(wanted))
            .map(|(_, fragment)| (*fragment).to_string())
    };
    let unclassified: Vec<&str> = rest_kinds
        .iter()
        .zip(rest.iter())
        .filter(|(kind, _)| kind.is_none())
        .map(|(_, fragment)| *fragment)
        .collect();
    let label = join(&unclassified);

    OasisIdentifier {
        original: original.to_string(),
        number,
        version: first_of(FragmentKind::Version),
        stage: first_of(FragmentKind::Stage),
        part: first_of(FragmentKind::Part),
        label,
    }
}

/// Join fragments with `-`; `None` when there is nothing to join.
fn join(fragments: &[&str]) -> Option<String> {
    if fragments.is_empty() {
        None
    } else {
        Some(fragments.join("-"))
    }
}

#[cfg(test)]
mod tests {
    use super::OasisIdentifier;
    use serde_json::json;

    #[test]
    fn decomposes_lead_version_stage_part() {
        let id =
            OasisIdentifier::parse("OASIS AkomaNtosoCore-v1.0-Pt2-Specifications").expect("parse");
        assert_eq!(id.original, "AkomaNtosoCore-v1.0-Pt2-Specifications");
        assert_eq!(id.number.as_deref(), Some("AkomaNtosoCore"));
        assert_eq!(id.version.as_deref(), Some("v1.0"));
        assert_eq!(id.part.as_deref(), Some("Pt2"));
        assert_eq!(id.label.as_deref(), Some("Specifications"));
        assert_eq!(id.stage, None);
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:oasis:standard",
                "original": "AkomaNtosoCore-v1.0-Pt2-Specifications",
                "number": "AkomaNtosoCore",
                "version": "v1.0",
                "part": "Pt2",
                "label": "Specifications",
            })
        );
    }

    #[test]
    fn keeps_the_human_on_original_alone_and_encodes_the_urn() {
        let id = OasisIdentifier::parse("OASIS BIASPROFILE").expect("parse");
        assert_eq!(id.to_human(), "OASIS BIASPROFILE");
        assert_eq!(id.to_urn(), "urn:oasis:BIASPROFILE");
        let spaced = OasisIdentifier::parse("OASIS a b]c").expect("parse");
        assert_eq!(spaced.to_urn(), "urn:oasis:a%20b%5Dc");
    }

    #[test]
    fn bare_integers_are_not_versions_and_stages_are_case_sensitive() {
        // "v3" is a bare integer: nothing classifiable, the whole slug
        // stays the lead number.
        let id = OasisIdentifier::parse("OASIS x-v3-y").expect("parse");
        assert_eq!(id.number.as_deref(), Some("x-v3-y"));
        assert_eq!(id.version, None);
        assert_eq!(id.label, None);
        let staged = OasisIdentifier::parse("OASIS CMIS-v1.1-Errata01").expect("parse");
        assert_eq!(staged.stage.as_deref(), Some("Errata01"));
        assert_eq!(staged.number.as_deref(), Some("CMIS"));
    }
}
