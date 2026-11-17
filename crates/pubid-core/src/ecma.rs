//! The ecma flavor: standards, technical reports, and mementos.
//!
//! The artifact dispatches the three classes through disjoint
//! captures (`number` / `tr_number` / `mem_number`); the flavor layer
//! pins the class per capture and carries the shared `edition`,
//! `volume`, and `part` faces. The wire hashes are
//! `pubid:ecma:standard` / `:technical-report` / `:memento`.
//!
//! Ingestion carries the reference's edition-word tolerance: the
//! corpus' non-normalized alias spellings (`ECMA-262 Edition 5.1`,
//! `ed. 5.1`, the bare `ECMA-262 5.1`) only reach the grammar through
//! the canonical `ed<edition>` face, so a failed parse is retried once
//! through the normalizer (exactly like `Pubid::Api`'s MPMP typo
//! normalization — ingestion-side, never a second parser).

use serde_json::Value;

use crate::artifact::ecma_artifact;
use crate::bound::{bound_str, optional_bound_str, optional_hash_str};
use crate::error::Result;

/// What concrete ECMA class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcmaKind {
    Memento,
    Standard,
    TechnicalReport,
}

use EcmaKind::*;

impl EcmaKind {
    /// The shared `_type` prefix every ecma tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:ecma:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            Memento => "pubid:ecma:memento",
            Standard => "pubid:ecma:standard",
            TechnicalReport => "pubid:ecma:technical-report",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [Memento, Standard, TechnicalReport]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }
}

/// The ecma identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcmaIdentifier {
    /// The concrete class (drives `_type` and the render shape).
    pub kind: EcmaKind,
    /// The document number (the TR/memento number for those classes).
    pub number: String,
    /// The dash-suffix part (`1` in `ECMA-418-1`).
    pub part: Option<String>,
    /// The edition (`1`, dotted `5.1`).
    pub edition: Option<String>,
    /// The volume (`1` in `ECMA-269 ed3 vol1`).
    pub volume: Option<String>,
}

impl EcmaIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact; a failed parse
    /// retries once through the edition-word normalizer (the alias
    /// spellings never reach the grammar raw).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ecma_artifact()?;
        let bound = match artifact.parse_and_bind(Self::ENTRY, input) {
            Ok(bound) => bound,
            Err(err) => match normalize_edition(input) {
                Some(normalized) => artifact.parse_and_bind(Self::ENTRY, &normalized)?,
                None => return Err(crate::error::Error::Parse(err)),
            },
        };

        let (kind, number) = if optional_bound_str(&bound, "mem_number").is_some() {
            (Memento, bound_str(&bound, "mem_number")?.to_string())
        } else if optional_bound_str(&bound, "tr_number").is_some() {
            (TechnicalReport, bound_str(&bound, "tr_number")?.to_string())
        } else {
            (Standard, bound_str(&bound, "number")?.to_string())
        };

        Ok(Self {
            kind,
            number,
            part: optional_bound_str(&bound, "part").map(str::to_string),
            edition: optional_bound_str(&bound, "edition").map(str::to_string),
            volume: optional_bound_str(&bound, "volume").map(str::to_string),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(EcmaKind::from_type_tag)
            .ok_or_else(|| crate::error::Error::InvalidHash {
                detail: format!("unknown ecma _type {:?}", hash.get("_type")),
            })?;
        Ok(Self {
            kind,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            part: optional_hash_str(hash, "part").map(str::to_string),
            edition: optional_hash_str(hash, "edition").map(str::to_string),
            volume: optional_hash_str(hash, "volume").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the present attributes.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        for (name, value) in [
            ("part", &self.part),
            ("edition", &self.edition),
            ("volume", &self.volume),
        ] {
            if let Some(value) = value {
                object.insert(name.into(), value.clone().into());
            }
        }
        Value::Object(object)
    }

    /// The canonical human: the standard glues the part to the number,
    /// the edition and volume hang off as words.
    pub fn to_human(&self) -> String {
        let mut out = match self.kind {
            Standard => {
                let mut out = format!("ECMA-{}", self.number);
                if let Some(part) = &self.part {
                    out.push('-');
                    out.push_str(part);
                }
                out
            }
            TechnicalReport => format!("ECMA TR/{}", self.number),
            Memento => format!("ECMA MEM/{}", self.number),
        };
        if let Some(edition) = &self.edition {
            out.push_str(&format!(" ed{edition}"));
        }
        if let Some(volume) = &self.volume {
            out.push_str(&format!(" vol{volume}"));
        }
        out
    }

    /// The URN: `urn:ecma:[mem|tr]:<number>[:part-<p>][:ed-<e>][:vol-<v>]`.
    pub fn to_urn(&self) -> String {
        let mut out = match self.kind {
            Memento => format!("urn:ecma:mem:{}", self.number),
            TechnicalReport => format!("urn:ecma:tr:{}", self.number),
            Standard => {
                let mut out = format!("urn:ecma:{}", self.number);
                if let Some(part) = &self.part {
                    out.push_str(&format!(":part-{part}"));
                }
                out
            }
        };
        if let Some(edition) = &self.edition {
            out.push_str(&format!(":ed-{edition}"));
        }
        if let Some(volume) = &self.volume {
            out.push_str(&format!(":vol-{volume}"));
        }
        out
    }
}

/// The ingestion normalizer: the corpus alias faces for the edition
/// (`ECMA-262 Edition 5.1`, `ed 5.1`, `ed. 5.1`, `ed.5.1`, and the
/// bare `ECMA 262 5.1`) all normalize to the canonical ` ed5.1`
/// suffix. `None` when the input has no recognizable edition tail.
fn normalize_edition(input: &str) -> Option<String> {
    let trimmed = input.trim_end();
    // The edition tail: the trailing run of digits and dots.
    let tail_start = trimmed.rfind(|c: char| !(c.is_ascii_digit() || c == '.'))? + 1;
    let (head, tail) = trimmed.split_at(tail_start);
    if tail.is_empty() || !tail.contains(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let head = head.trim_end();
    let lowered = head.to_lowercase();
    for word in [" edition", " ed.", " ed"] {
        if let Some(rest) = lowered.strip_suffix(word) {
            // The suffixes are ASCII, so the byte cut matches; the
            // dotted word form leaves its dot in the tail.
            let tail = tail.trim_start_matches('.');
            return Some(format!("{} ed{}", &head[..rest.len()], tail));
        }
    }
    // The bare form: `ECMA 262 5.1` / `ECMA-262 5.1` — the head is
    // ECMA followed only by digits, dashes, and spaces.
    let body = stripped_prefix(head, "ECMA")?;
    if !body.is_empty()
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == ' ')
        && body.contains(|c: char| c.is_ascii_digit())
    {
        return Some(format!("{head} ed{tail}"));
    }
    None
}

/// Case-insensitive ASCII prefix strip.
fn stripped_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{EcmaIdentifier, EcmaKind};
    use serde_json::json;

    #[test]
    fn the_standard_glasses_the_part_and_hangs_the_edition_off() {
        let id = EcmaIdentifier::parse("ECMA-262 ed5.1").expect("parse");
        assert_eq!(id.kind, EcmaKind::Standard);
        assert_eq!(id.number, "262");
        assert_eq!(id.edition.as_deref(), Some("5.1"));
        assert_eq!(id.to_human(), "ECMA-262 ed5.1");
        assert_eq!(id.to_urn(), "urn:ecma:262:ed-5.1");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ecma:standard",
                "number": "262",
                "edition": "5.1",
            })
        );
    }

    #[test]
    fn the_edition_word_aliases_normalize_to_the_canonical_face() {
        for spelling in [
            "ECMA 262 5.1",
            "ECMA-262 5.1",
            "ECMA-262 Edition 5.1",
            "ECMA-262 ed 5.1",
            "ECMA-262 ed. 5.1",
            "ECMA-262 ed.5.1",
        ] {
            let id = EcmaIdentifier::parse(spelling).expect(spelling);
            assert_eq!(id.to_human(), "ECMA-262 ed5.1", "{spelling}");
        }
    }

    #[test]
    fn volume_and_part_render_in_the_wire_order() {
        let id = EcmaIdentifier::parse("ECMA-269 ed3 vol1").expect("parse");
        assert_eq!(id.volume.as_deref(), Some("1"));
        assert_eq!(id.to_human(), "ECMA-269 ed3 vol1");
        assert_eq!(id.to_urn(), "urn:ecma:269:ed-3:vol-1");

        let id = EcmaIdentifier::parse("ECMA-418-1 ed1").expect("parse");
        assert_eq!(id.part.as_deref(), Some("1"));
        assert_eq!(id.to_human(), "ECMA-418-1 ed1");
        assert_eq!(id.to_urn(), "urn:ecma:418:part-1:ed-1");
    }

    #[test]
    fn technical_reports_and_mementos_use_their_faces() {
        let tr = EcmaIdentifier::parse("ECMA TR/100 ed1").expect("parse");
        assert_eq!(tr.kind, EcmaKind::TechnicalReport);
        assert_eq!(tr.to_human(), "ECMA TR/100 ed1");
        assert_eq!(tr.to_urn(), "urn:ecma:tr:100:ed-1");

        let mem = EcmaIdentifier::parse("ECMA MEM/1962").expect("parse");
        assert_eq!(mem.kind, EcmaKind::Memento);
        assert_eq!(mem.to_human(), "ECMA MEM/1962");
        assert_eq!(mem.to_urn(), "urn:ecma:mem:1962");
        assert_eq!(
            mem.to_hash(),
            json!({"_type": "pubid:ecma:memento", "number": "1962"})
        );
    }

    #[test]
    fn round_trips_through_the_hash() {
        let id = EcmaIdentifier::parse("ECMA-100 ed1").expect("parse");
        let rebuilt = EcmaIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt, id);
        assert_eq!(rebuilt.to_hash(), id.to_hash());
    }
}
