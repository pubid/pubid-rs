//! The nesc flavor: National Electrical Safety Code identifiers (the
//! IEEE C2 family).
//!
//! The artifact binds the flat captures (`code`, `year`, the registered
//! marks, `variant`, `edition`, `month`, `preprint`,
//! `nesc_draft_marker`); the flavor layer dispatches the concrete class
//! exactly like `Pubid::Ieee::Nesc::Builder` (preprint -> draft ->
//! variant -> code -> plain edition) and hoists the C2 designation
//! through the shared IEEE code split (`prefix "C"`, `number "2"`).
//! Every NESC document keys under C2 even when its spelling prints no
//! code (the builder's `DEFAULT_CODE`).

use serde_json::Value;

use crate::artifact::nesc_artifact;
use crate::bound::{optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};
use crate::ieee_code::IeeeCode;

/// What concrete NESC class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NescKind {
    Standard,
    Edition,
    Handbook,
    Redline,
    Preprint,
    Draft,
}

impl NescKind {
    /// The shared `_type` prefix every nesc tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:ieee:nesc-";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            NescKind::Standard => "pubid:ieee:nesc-standard",
            NescKind::Edition => "pubid:ieee:nesc-edition",
            NescKind::Handbook => "pubid:ieee:nesc-handbook",
            NescKind::Redline => "pubid:ieee:nesc-redline",
            NescKind::Preprint => "pubid:ieee:nesc-preprint",
            NescKind::Draft => "pubid:ieee:nesc-draft",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        match tag {
            "pubid:ieee:nesc-standard" => Some(NescKind::Standard),
            "pubid:ieee:nesc-edition" => Some(NescKind::Edition),
            "pubid:ieee:nesc-handbook" => Some(NescKind::Handbook),
            "pubid:ieee:nesc-redline" => Some(NescKind::Redline),
            "pubid:ieee:nesc-preprint" => Some(NescKind::Preprint),
            "pubid:ieee:nesc-draft" => Some(NescKind::Draft),
            _ => None,
        }
    }
}

/// The nesc identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NescIdentifier {
    /// The concrete class (drives `_type` and the human shape).
    pub kind: NescKind,
    /// The bare-digit number of the C2 code (always `"2"`).
    pub number: String,
    /// The letter prefix of the C2 code (always `"C"`).
    pub prefix: String,
    /// The publication year.
    pub year: Option<String>,
    /// The `Handbook` / `Redline` variant word.
    pub variant: Option<String>,
    /// The handbook edition (`Seventh Edition`).
    pub edition: Option<String>,
    /// The draft month (`January`).
    pub month: Option<String>,
    /// The `(R)` after the full name or the bare NESC abbreviation.
    pub registered: bool,
    /// The `(NESC)` / `(NESC(R))` abbreviation suffix was present.
    pub abbr_suffix: bool,
    /// The `(R)` inside that abbreviation suffix.
    pub abbr_suffix_registered: bool,
}

impl NescIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "nesc_identifier";

    /// The NESC's own designation: every document in the family keys
    /// under it, even when the spelling prints no code.
    const DEFAULT_CODE: &'static str = "C2";

    /// Parse an input through the vendored artifact and dispatch the
    /// class (`Pubid::Ieee::Nesc::Builder` order).
    pub fn parse(input: &str) -> Result<Self> {
        // The rendered faces carry the IEEE publication prefix; the
        // NESC grammar parses the year-first/code-first forms, so the
        // prefix strips exactly like the Ruby flavor's preprocessor.
        let input = input.strip_prefix("IEEE Std ").unwrap_or(input);
        let artifact = nesc_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let kind = if optional_bound_str(&bound, "preprint").is_some() {
            NescKind::Preprint
        } else if optional_bound_str(&bound, "nesc_draft_marker").is_some() {
            NescKind::Draft
        } else if let Some(variant) = optional_bound_str(&bound, "variant") {
            match variant {
                "Handbook" => NescKind::Handbook,
                _ => NescKind::Redline,
            }
        } else if optional_bound_str(&bound, "code").is_some() {
            NescKind::Standard
        } else {
            NescKind::Edition
        };

        let code = IeeeCode::parse(
            optional_bound_str(&bound, "code").unwrap_or(Self::DEFAULT_CODE),
        )
        .ok_or(Error::InvalidHash {
            detail: "nesc code does not split".to_string(),
        })?;

        Ok(Self {
            kind,
            number: code.number,
            prefix: code.prefix.ok_or(Error::InvalidHash {
                detail: "nesc code has no letter prefix".to_string(),
            })?,
            year: optional_bound_str(&bound, "year").map(str::to_string),
            variant: optional_bound_str(&bound, "variant").map(str::to_string),
            edition: optional_bound_str(&bound, "edition").map(str::to_string),
            month: optional_bound_str(&bound, "month").map(str::to_string),
            registered: optional_bound_str(&bound, "name_registered").is_some()
                || optional_bound_str(&bound, "abbr_registered").is_some(),
            abbr_suffix: optional_bound_str(&bound, "paren_abbr").is_some(),
            abbr_suffix_registered: optional_bound_str(&bound, "paren_registered").is_some(),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(NescKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown nesc _type {:?}", hash.get("_type")),
            })?;
        Ok(Self {
            kind,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            prefix: crate::bound::hash_str(hash, "prefix")?.to_string(),
            year: optional_hash_str(hash, "year").map(str::to_string),
            variant: optional_hash_str(hash, "variant").map(str::to_string),
            edition: optional_hash_str(hash, "edition").map(str::to_string),
            month: optional_hash_str(hash, "month").map(str::to_string),
            registered: hash.get("registered") == Some(&Value::Bool(true)),
            abbr_suffix: hash.get("abbr_suffix") == Some(&Value::Bool(true)),
            abbr_suffix_registered: hash.get("abbr_suffix_registered")
                == Some(&Value::Bool(true)),
        })
    }

    /// The canonical hash: `_type` first, then the attributes (true
    /// flags only; the C2 split columns always serialize).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        if self.registered {
            object.insert("registered".into(), Value::Bool(true));
        }
        if self.abbr_suffix {
            object.insert("abbr_suffix".into(), Value::Bool(true));
        }
        if self.abbr_suffix_registered {
            object.insert("abbr_suffix_registered".into(), Value::Bool(true));
        }
        if let Some(edition) = &self.edition {
            object.insert("edition".into(), edition.clone().into());
        }
        if let Some(variant) = &self.variant {
            object.insert("variant".into(), variant.clone().into());
        }
        object.insert("prefix".into(), self.prefix.clone().into());
        Value::Object(object)
    }

    /// The canonical human representation (one shape per class, per
    /// the NESC leaves; `registered` marks print only on the faces
    /// that read them).
    pub fn to_human(&self) -> String {
        match self.kind {
            NescKind::Standard => {
                let mut code_part = "C2".to_string();
                if let Some(year) = &self.year {
                    code_part.push_str(&format!("-{year}"));
                }
                format!("{code_part} National Electrical Safety Code")
            }
            NescKind::Edition => {
                format!("IEEE Std {} {}", self.year_or_empty(), self.name_portion())
            }
            NescKind::Handbook => {
                let mut abbr = "NESC".to_string();
                if self.registered {
                    abbr.push_str("(R)");
                }
                let mut result =
                    format!("IEEE Std {} {abbr} Handbook", self.year_or_empty());
                if let Some(edition) = &self.edition {
                    result.push_str(&format!(", {edition}"));
                }
                result
            }
            NescKind::Redline => format!("{} NESC Redline", self.year_or_empty()),
            NescKind::Preprint => {
                format!(
                    "IEEE Std C2.{}.Preprint",
                    self.year.as_deref().unwrap_or("")
                )
            }
            NescKind::Draft => {
                let mut result = "Draft National Electrical Safety Code".to_string();
                if let (Some(month), Some(year)) = (&self.month, &self.year) {
                    result.push_str(&format!(", {month} {year}"));
                }
                result
            }
        }
    }

    /// The URN: `urn:ieee:ieee:C2[:<year>][:ed.<edition>][:preprint]`
    /// (the IEEE generator's segments; the redline prints no suffix —
    /// its `variant` is not the base `redline` attribute).
    pub fn to_urn(&self) -> String {
        let mut urn = format!("urn:ieee:ieee:{}{}", self.prefix, self.number);
        if let Some(year) = &self.year {
            urn.push_str(&format!(":{year}"));
        }
        if let Some(edition) = &self.edition {
            urn.push_str(&format!(":ed.{edition}"));
        }
        if self.kind == NescKind::Preprint {
            urn.push_str(":preprint");
        }
        urn
    }

    /// The year, or the empty string when absent (the faces that always
    /// print a year slot).
    fn year_or_empty(&self) -> &str {
        self.year.as_deref().unwrap_or("")
    }

    /// The document-name portion: the full name with its registered
    /// mark and the optional `(NESC(R))` abbreviation suffix
    /// (`Nesc::Base#name_portion`).
    fn name_portion(&self) -> String {
        let mut name = "National Electrical Safety Code".to_string();
        if self.registered {
            name.push_str("(R)");
        }
        if self.abbr_suffix {
            let mut suffix = "NESC".to_string();
            if self.abbr_suffix_registered {
                suffix.push_str("(R)");
            }
            name.push_str(&format!(" ({suffix})"));
        }
        name
    }
}
