//! The api flavor: API MPMS / typed / typeless identifiers.
//!
//! The artifact binds the captures flat (`type`, `chapter`, `section`,
//! `subsection`, `year`, `number`, `part`, and the combined form's
//! `second_*` fields); the flavor layer reproduces the Ruby builder's
//! cast: `chapter` feeds the MPMS `number`, `type` is dispatch-only
//! (the `_type` tag pins the class), and the slash-combined form drops
//! its type and overwrites number/part/year with the second side's
//! captures (the tree backend's overwrite order, per
//! `Pubid::Api::Builder`). The pre-parse "API MPMP" typo normalization
//! is ingestion-side, exactly like `Pubid::Api::Parser.normalize_input`.

use serde_json::Value;

use crate::artifact::api_artifact;
use crate::bound::{bound_str, optional_bound_str, optional_hash_str};
use crate::error::Result;

/// What concrete API class an identifier belongs to (the `_type` tag
/// and the human type token together).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKind {
    Bulletin,
    ContinuousOperationsStandard,
    Mpms,
    Publication,
    RecommendedPractice,
    Specification,
    Standard,
    TechnicalReport,
    TypelessStandard,
}

use ApiKind::*;

impl ApiKind {
    /// The shared `_type` prefix every api tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:api:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            Bulletin => "pubid:api:bulletin",
            ContinuousOperationsStandard => "pubid:api:continuous-operations-standard",
            Mpms => "pubid:api:mpms",
            Publication => "pubid:api:publication",
            RecommendedPractice => "pubid:api:recommended-practice",
            Specification => "pubid:api:specification",
            Standard => "pubid:api:standard",
            TechnicalReport => "pubid:api:technical-report",
            TypelessStandard => "pubid:api:typeless-standard",
        }
    }

    /// The human type token (the `type_string` on the Ruby leaf; the
    /// typeless class renders none).
    fn type_string(self) -> Option<&'static str> {
        match self {
            Bulletin => Some("BULL"),
            ContinuousOperationsStandard => Some("COS"),
            Mpms => Some("MPMS"),
            Publication => Some("PUBL"),
            RecommendedPractice => Some("RP"),
            Specification => Some("SPEC"),
            Standard => Some("STD"),
            TechnicalReport => Some("TR"),
            TypelessStandard => None,
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [
            Bulletin,
            ContinuousOperationsStandard,
            Mpms,
            Publication,
            RecommendedPractice,
            Specification,
            Standard,
            TechnicalReport,
            TypelessStandard,
        ]
        .into_iter()
        .find(|kind| kind.type_tag() == tag)
    }

    /// The parse-tree `type` token -> class (`Pubid::Api::Builder::
    /// TYPE_CLASS_MAP`).
    fn from_type_token(token: &str) -> Option<Self> {
        match token {
            "BULL" => Some(Bulletin),
            "MPMS" => Some(Mpms),
            "RP" => Some(RecommendedPractice),
            "SPEC" => Some(Specification),
            "STD" => Some(Standard),
            "TR" => Some(TechnicalReport),
            "COS" => Some(ContinuousOperationsStandard),
            "PUBL" => Some(Publication),
            _ => None,
        }
    }
}

/// The api identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiIdentifier {
    /// The concrete class (drives `_type` and the human type token).
    pub kind: ApiKind,
    /// The document number; for MPMS this is the CHAPTER (the builder
    /// routes `chapter` into `number`).
    pub number: String,
    /// The `, Part N` suffix.
    pub part: Option<String>,
    /// The MPMS chapter section (after the first dot).
    pub section: Option<String>,
    /// The MPMS chapter subsection (after the second dot).
    pub subsection: Option<String>,
    /// The `-YYYY` / `:YYYY` date year.
    pub year: Option<String>,
}

impl ApiIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact (the MPMP typo
    /// normalizes to MPMS first, exactly like the Ruby ingestion).
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = api_artifact()?;
        let normalized = if input.contains("API MPMP") {
            input.replace("API MPMP", "API MPMS")
        } else {
            input.to_string()
        };
        let bound = artifact.parse_and_bind(Self::ENTRY, &normalized)?;

        // The combined form: drop the type dispatch and overwrite the
        // first side's number/part/year with the second side's (the
        // builder's unflatten_combined; a combined form always builds
        // the typeless class).
        let (kind, number, part, year) = if optional_bound_str(&bound, "second_number").is_some()
            || optional_bound_str(&bound, "second_type").is_some()
        {
            (
                TypelessStandard,
                bound_str(&bound, "second_number")?.to_string(),
                optional_bound_str(&bound, "second_part").map(str::to_string),
                optional_bound_str(&bound, "second_year").map(str::to_string),
            )
        } else if let Some(chapter) = optional_bound_str(&bound, "chapter") {
            // MPMS: the chapter IS the document number.
            (
                Mpms,
                chapter.to_string(),
                None,
                optional_bound_str(&bound, "year").map(str::to_string),
            )
        } else {
            let kind = optional_bound_str(&bound, "type")
                .and_then(ApiKind::from_type_token)
                .unwrap_or(TypelessStandard);
            (
                kind,
                bound_str(&bound, "number")?.to_string(),
                optional_bound_str(&bound, "part").map(str::to_string),
                optional_bound_str(&bound, "year").map(str::to_string),
            )
        };

        Ok(Self {
            kind,
            number,
            part,
            section: optional_bound_str(&bound, "section").map(str::to_string),
            subsection: optional_bound_str(&bound, "subsection").map(str::to_string),
            year,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(ApiKind::from_type_tag)
            .ok_or_else(|| crate::error::Error::InvalidHash {
                detail: format!("unknown api _type {:?}", hash.get("_type")),
            })?;
        Ok(Self {
            kind,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            part: optional_hash_str(hash, "part").map(str::to_string),
            section: optional_hash_str(hash, "section").map(str::to_string),
            subsection: optional_hash_str(hash, "subsection").map(str::to_string),
            year: optional_hash_str(hash, "year").map(str::to_string),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (absent
    /// optionals are omitted; the type token never serializes).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        for (name, value) in [
            ("part", &self.part),
            ("section", &self.section),
            ("subsection", &self.subsection),
            ("year", &self.year),
        ] {
            if let Some(value) = value {
                object.insert(name.into(), value.clone().into());
            }
        }
        Value::Object(object)
    }

    /// The canonical human representation (three render shapes, per
    /// `Pubid::Api::Renderer`).
    pub fn to_human(&self) -> String {
        let code = match &self.part {
            Some(part) => format!("{}-{}", self.number, part),
            None => self.number.clone(),
        };
        match self.kind {
            Mpms => {
                let mut parts = vec![
                    "API".to_string(),
                    "MPMS".to_string(),
                    format!("CH {}", self.number),
                ];
                if let Some(section) = &self.section {
                    parts.push(format!(".{section}"));
                    if let Some(subsection) = &self.subsection {
                        parts.push(format!(".{subsection}"));
                    }
                }
                if let Some(year) = &self.year {
                    parts.push(format!("-{year}"));
                }
                // The Ruby renderer joins with a space then closes
                // " ." -> "." so the section hangs off the chapter.
                parts.join(" ").replace(" .", ".")
            }
            TypelessStandard => {
                let mut parts = vec!["API".to_string(), code];
                if let Some(year) = &self.year {
                    parts.push(format!("-{year}"));
                }
                parts.join(" ")
            }
            kind => {
                let mut parts = vec!["API".to_string()];
                if let Some(token) = kind.type_string() {
                    parts.push(token.to_string());
                }
                parts.push(code);
                if let Some(year) = &self.year {
                    parts.push(format!("-{year}"));
                }
                parts.join(" ")
            }
        }
    }

    /// The URN: `urn:api:std:<number>[:-<part>][:<year>]` (the base
    /// generator's segments with the `urn_part` leading dash).
    pub fn to_urn(&self) -> String {
        let mut urn = format!("urn:api:std:{}", self.number);
        if let Some(part) = &self.part {
            urn.push_str(&format!(":-{part}"));
        }
        if let Some(year) = &self.year {
            urn.push_str(&format!(":{year}"));
        }
        urn
    }
}

#[cfg(test)]
mod tests {
    use super::ApiIdentifier;
    use serde_json::json;

    #[test]
    fn mpms_routes_chapter_into_number_and_glues_sections() {
        let id = ApiIdentifier::parse("API MPMS CH 11.2.5").expect("parse");
        assert_eq!(id.kind, super::ApiKind::Mpms);
        assert_eq!(id.number, "11");
        assert_eq!(id.section.as_deref(), Some("2"));
        assert_eq!(id.subsection.as_deref(), Some("5"));
        assert_eq!(id.to_human(), "API MPMS CH 11.2.5");
        assert_eq!(id.to_urn(), "urn:api:std:11");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:api:mpms",
                "number": "11",
                "section": "2",
                "subsection": "5",
            })
        );
    }

    #[test]
    fn typed_identifier_carries_the_type_token_and_part() {
        let id = ApiIdentifier::parse("API RP 100-3").expect("parse");
        assert_eq!(id.kind, super::ApiKind::RecommendedPractice);
        assert_eq!(id.number, "100");
        assert_eq!(id.part.as_deref(), Some("3"));
        assert_eq!(id.to_human(), "API RP 100-3");
        assert_eq!(id.to_urn(), "urn:api:std:100:-3");
    }

    #[test]
    fn typeless_renders_without_the_type_token() {
        let id = ApiIdentifier::parse("API 5L").expect("parse");
        assert_eq!(id.kind, super::ApiKind::TypelessStandard);
        assert_eq!(id.to_human(), "API 5L");
    }

    #[test]
    fn the_mpmp_typo_normalizes_to_mpms_before_parsing() {
        let id = ApiIdentifier::parse("API MPMP CH 10.10").expect("parse");
        assert_eq!(id.to_human(), "API MPMS CH 10.10");
        assert_eq!(id.to_urn(), "urn:api:std:10");
    }

    #[test]
    fn round_trips_through_the_hash() {
        let id = ApiIdentifier::parse("API COS 1-08").expect("parse");
        let rebuilt = ApiIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt, id);
        assert_eq!(rebuilt.to_human(), "API COS 1-08");
    }
}
