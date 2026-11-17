//! The ccsds flavor: the CCSDS standards (`CCSDS 120.0-G-3`), the
//! `-S` superseded suffixes, the translated editions (`CCSDS
//! 551.1-O-2 - Russian Translated`), and the corrigenda (`CCSDS
//! 121.0-B-1-S Cor. 1`).
//!
//! The artifact binds the flat code faces (`number`, `part`, `type`,
//! `edition`, `suffix`, `language`) and the `corrigenda` array; the
//! flavor layer folds the corrigenda last-wraps-first onto the base
//! (the reference's `Builder.build`). The update-code line
//! (`Corrigendum 1` → `Cor. 1`) normalizes before the parse. The URN
//! body is ONE segment (`121.0-B-1-S`) with `:cor.<n>` supplement
//! segments; a language face makes the reference's own `to_urn`
//! raise (`language&.code` on a plain String — the exporter omits
//! those URNs), mirrored: no URN is emitted for it.

use serde_json::Value;

use crate::artifact::ccsds_artifact;
use crate::bound::{optional_at, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete ccsds class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CcsdsKind {
    Base,
    Corrigendum,
}

use CcsdsKind::*;

impl CcsdsKind {
    /// The shared `_type` prefix every ccsds tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:ccsds:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            Base => "pubid:ccsds:base",
            Corrigendum => "pubid:ccsds:corrigendum",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [Base, Corrigendum]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }
}

/// The ccsds identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcsdsIdentifier {
    /// The concrete class (drives `_type` and the render shape).
    pub kind: CcsdsKind,
    /// The document number (`320` in `CCSDS 320.0-B-1`); a
    /// corrigendum's sequence number (`1` in `Cor. 1`).
    pub number: String,
    /// The part (`0` in `320.0`).
    pub part: Option<String>,
    /// The type letter (`B`, `G`, `O`).
    pub doc_type: Option<String>,
    /// The edition (`1` in `-1`).
    pub edition: Option<String>,
    /// The suffix (`S` in `-S`).
    pub suffix: Option<String>,
    /// The translation language face (`French`).
    pub language: Option<String>,
    /// A corrigendum's underlying document.
    pub base: Option<Box<CcsdsIdentifier>>,
}

/// The pre-parse update-code lines (Core::UpdateCodes, full-line).
fn apply_update_codes(input: &str) -> String {
    match input {
        "CCSDS 320.0-B-1-S Corrigendum 1" => "CCSDS 320.0-B-1-S Cor. 1".to_string(),
        other => other.to_string(),
    }
}

impl CcsdsIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and fold the
    /// corrigenda onto the base.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ccsds_artifact()?;
        let normalized = apply_update_codes(input);
        let bound = artifact.parse_and_bind(Self::ENTRY, &normalized)?;
        Ok(build(&bound))
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(CcsdsKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown ccsds _type {:?}", hash.get("_type")),
            })?;
        let base = match hash.get("base").cloned().filter(|base| !base.is_null()) {
            Some(base) => Some(Box::new(CcsdsIdentifier::from_hash(&base)?)),
            None => None,
        };
        Ok(Self {
            kind,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            part: optional_hash_str(hash, "part").map(str::to_string),
            doc_type: optional_hash_str(hash, "type").map(str::to_string),
            edition: optional_hash_str(hash, "edition").map(str::to_string),
            suffix: optional_hash_str(hash, "suffix").map(str::to_string),
            language: optional_hash_str(hash, "language").map(str::to_string),
            base,
        })
    }

    /// The canonical hash: `_type`, the code faces, then the base (a
    /// corrigendum carries only its number and the base).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if self.kind == Base {
            for (key, value) in [
                ("part", &self.part),
                ("edition", &self.edition),
                ("type", &self.doc_type),
                ("suffix", &self.suffix),
                ("language", &self.language),
            ] {
                if let Some(value) = value {
                    object.insert(key.into(), value.clone().into());
                }
            }
        }
        if let Some(base) = &self.base {
            object.insert("base".into(), base.to_hash());
        }
        Value::Object(object)
    }

    /// The canonical human (the hand-composed reference faces).
    pub fn to_human(&self) -> String {
        if self.kind == Corrigendum {
            let base = self
                .base
                .as_deref()
                .map(CcsdsIdentifier::to_human)
                .unwrap_or_default();
            return format!("{base} Cor. {}", self.number);
        }
        let mut result = format!("CCSDS {}", self.number);
        if let Some(part) = &self.part {
            result.push('.');
            result.push_str(part);
        }
        if let Some(doc_type) = &self.doc_type {
            result.push('-');
            result.push_str(doc_type);
        }
        if let Some(edition) = &self.edition {
            result.push('-');
            result.push_str(edition);
        }
        if let Some(suffix) = &self.suffix {
            result.push('-');
            result.push_str(suffix);
        }
        if let Some(language) = &self.language {
            result.push_str(&format!(" - {language} Translated"));
        }
        result
    }

    /// The URN; `None` when a language face is present (the
    /// reference's `to_urn` raises on it — mirrored, not papered
    /// over).
    pub fn to_urn(&self) -> Option<String> {
        if self.language.is_some() {
            return None;
        }
        if self.kind == Corrigendum {
            // The chain unwraps outermost-first; every link appends
            // its `:cor.<n>` segment to the base body.
            let mut chain: Vec<&CcsdsIdentifier> = Vec::new();
            let mut current = self;
            while current.kind == Corrigendum {
                chain.push(current);
                current = current.base.as_deref()?;
            }
            let body = current.base_body()?;
            let mut parts = vec![format!("urn:ccsds:{body}")];
            for link in chain {
                if !link.number.is_empty() {
                    parts.push(format!("cor.{}", link.number));
                }
            }
            return Some(parts.join(":"));
        }
        self.base_body().map(|body| format!("urn:ccsds:{body}"))
    }

    /// The base URN body: one segment, `number[.part][-type][-edition]
    /// [-suffix]`.
    fn base_body(&self) -> Option<String> {
        let mut face = self.number.clone();
        if let Some(part) = &self.part {
            face.push('.');
            face.push_str(part);
        }
        if let Some(doc_type) = &self.doc_type {
            face.push('-');
            face.push_str(doc_type);
        }
        if let Some(edition) = &self.edition {
            face.push('-');
            face.push_str(edition);
        }
        if let Some(suffix) = &self.suffix {
            face.push('-');
            face.push_str(suffix);
        }
        Some(face)
    }
}

/// The builder cast: the corrigenda fold last-wraps-first onto the
/// base (each array entry wraps the previous result; the last entry is
/// the outermost identifier).
fn build(bound: &Value) -> CcsdsIdentifier {
    let read = |key: &'static str| optional_at(bound, key).map(str::to_string);
    let base = CcsdsIdentifier {
        kind: Base,
        number: read("number").unwrap_or_default(),
        part: read("part"),
        doc_type: read("type"),
        edition: read("edition"),
        suffix: read("suffix"),
        language: read("language"),
        base: None,
    };
    let corrigenda = bound
        .get("corrigenda")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut current = base;
    for corrigendum in corrigenda {
        let number = optional_at(&corrigendum, "cor_number")
            .map(str::to_string)
            .unwrap_or_default();
        current = CcsdsIdentifier {
            kind: Corrigendum,
            number,
            part: None,
            doc_type: None,
            edition: None,
            suffix: None,
            language: None,
            base: Some(Box::new(current)),
        };
    }
    current
}

#[cfg(test)]
mod tests {
    use super::{CcsdsIdentifier, CcsdsKind};
    use serde_json::json;

    #[test]
    fn the_base_code_composes_one_segment() {
        let id = CcsdsIdentifier::parse("CCSDS 120.0-G-3").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ccsds:base",
                "number": "120",
                "part": "0",
                "edition": "3",
                "type": "G",
            })
        );
        assert_eq!(id.to_human(), "CCSDS 120.0-G-3");
        assert_eq!(id.to_urn().as_deref(), Some("urn:ccsds:120.0-G-3"));
    }

    #[test]
    fn the_suffix_and_update_code_line_normalize() {
        let plain = CcsdsIdentifier::parse("CCSDS 320.0-B-1-S").expect("parse");
        assert_eq!(plain.suffix.as_deref(), Some("S"));

        let spelled = CcsdsIdentifier::parse("CCSDS 320.0-B-1-S Corrigendum 1").expect("parse");
        assert_eq!(spelled.kind, CcsdsKind::Corrigendum);
        assert_eq!(spelled.to_human(), "CCSDS 320.0-B-1-S Cor. 1");
        assert_eq!(
            spelled.to_urn().as_deref(),
            Some("urn:ccsds:320.0-B-1-S:cor.1")
        );
    }

    #[test]
    fn the_language_face_drops_the_urn() {
        let id = CcsdsIdentifier::parse("CCSDS 551.1-O-2 - Russian Translated").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ccsds:base",
                "number": "551",
                "part": "1",
                "edition": "2",
                "type": "O",
                "language": "Russian",
            })
        );
        assert_eq!(id.to_human(), "CCSDS 551.1-O-2 - Russian Translated");
        assert_eq!(id.to_urn(), None);
    }

    #[test]
    fn the_corrigendum_folds_onto_its_base() {
        let id = CcsdsIdentifier::parse("CCSDS 121.0-B-1-S Cor. 1").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ccsds:corrigendum",
                "number": "1",
                "base": {
                    "_type": "pubid:ccsds:base",
                    "number": "121",
                    "part": "0",
                    "edition": "1",
                    "type": "B",
                    "suffix": "S",
                },
            })
        );
        assert_eq!(id.to_human(), "CCSDS 121.0-B-1-S Cor. 1");
        assert_eq!(id.to_urn().as_deref(), Some("urn:ccsds:121.0-B-1-S:cor.1"));
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "CCSDS 120.0-G-3",
            "CCSDS 650.0-M-2 - French Translated",
            "CCSDS 122.0-B-1-S Cor. 2",
        ] {
            let id = CcsdsIdentifier::parse(spelling).expect(spelling);
            let rebuilt = CcsdsIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{spelling}");
        }
    }
}
