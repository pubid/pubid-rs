//! The etsi flavor: the largest corpus — `ETSI EG 200 053 V1.5.1
//! (2004-06)`, the `ed.N` editions, the `-parts`, and the `/A`/`/C`
//! amendment/corrigendum chains.
//!
//! The artifact binds the flat captures (`type`, `number`, `parts[]`,
//! `minor`, `version`/`edition`, `year`, `month`,
//! `supplements[]` with `supplement_kind`/`supplement_number`). The
//! reference composes the number face from structured pieces
//! (`200 053`, `NFV-IFA 024`, `ARF 004`) — the artifact flattens it
//! without the separator, so the flavor re-reads the number face from
//! the input tail and keeps it only when it agrees with the
//! artifact's flat capture on digits (a genuine `minor` stays a
//! minor). The version face flattens to a scalar plus the `is_edition`
//! flag (omitted when false); the date renders `year-month`. The
//! supplements fold last-wraps-first; their URN is the BASE's (the
//! Ruby generator string-matches "SupplementIdentifier" on the class
//! name, which "Amendment" doesn't contain — corpus-recorded
//! behavior, mirrored exactly).

use serde_json::Value;

use crate::artifact::etsi_artifact;
use crate::bound::{capture_in, optional_at, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete etsi class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtsiKind {
    EtsiStandard,
    Amendment,
    Corrigendum,
}

use EtsiKind::*;

impl EtsiKind {
    /// The shared `_type` prefix every etsi tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:etsi:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            EtsiStandard => "pubid:etsi:etsi-standard",
            Amendment => "pubid:etsi:amendment",
            Corrigendum => "pubid:etsi:corrigendum",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [EtsiStandard, Amendment, Corrigendum]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }
}

/// The etsi identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtsiIdentifier {
    /// The concrete class (drives `_type` and the render shape).
    pub kind: EtsiKind,
    /// The document type token (`EG`, `TR`, `I-ETS`).
    pub doc_type: Option<String>,
    /// The composed number face (`200 053`, `NFV-IFA 024`); a
    /// supplement's ordinal.
    pub number: String,
    /// The minor face.
    pub minor: Option<String>,
    /// The dash parts (`1` in `EN 300 175-1`).
    pub parts: Vec<String>,
    /// The version face (`1.5.1`).
    pub version: Option<String>,
    /// Whether the version face is the `ed.N` edition form.
    pub is_edition: bool,
    /// The date year (`2004`).
    pub year: Option<String>,
    /// The date month (`06`).
    pub month: Option<String>,
    /// A supplement's underlying document.
    pub base: Option<Box<EtsiIdentifier>>,
}

/// The number-face composition: the complex `prefix[-prefix2] num`
/// form, else the plain digits — read off the input tail after the
/// type token, kept only when it agrees with the artifact's flat
/// capture on non-space characters (a genuine `minor` — where the
/// grammar read the second run as a minor — never agrees, so the flat
/// capture stands).
fn composed_number(tail: &str, flat: &str) -> Option<String> {
    let chars: Vec<char> = tail.chars().collect();
    let alnum = |c: char| c.is_ascii_alphanumeric();
    let span = |from: usize, to: usize| chars[from..to].iter().collect::<String>();

    let mut index = 0;
    while index < chars.len() && alnum(chars[index]) {
        index += 1;
    }
    if index == 0 {
        return None;
    }

    // The optional `-prefix2` — only when a spaced number follows it.
    let mut probe = index;
    let mut has_prefix2 = false;
    if probe < chars.len() && chars[probe] == '-' {
        let mut end = probe + 1;
        while end < chars.len() && alnum(chars[end]) {
            end += 1;
        }
        if end > probe + 1 && chars.get(end) == Some(&' ') {
            has_prefix2 = true;
            probe = end;
        }
    }

    if chars.get(probe) == Some(&' ') {
        let start = probe + 1;
        let mut end = start;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        if end > start {
            let candidate = if has_prefix2 {
                format!("{}-{} {}", span(0, index), span(index + 1, probe), span(start, end))
            } else {
                format!("{} {}", span(0, index), span(start, end))
            };
            if candidate.replace(' ', "") == flat.replace(' ', "") {
                return Some(candidate);
            }
            return None;
        }
    }

    // The plain digits form.
    let mut end = 0;
    while end < chars.len() && chars[end].is_ascii_digit() {
        end += 1;
    }
    if end > 0 {
        let candidate = span(0, end);
        if candidate == flat {
            return Some(candidate);
        }
    }
    None
}

impl EtsiIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and fold the
    /// supplements onto the base.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = etsi_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Self::build(&bound, input)
    }

    /// The builder cast over a bound hash (the input tail feeds the
    /// number-face composition).
    pub fn build(bound: &Value, input: &str) -> Result<Self> {
        let read = |key: &'static str| optional_at(bound, key).map(str::to_string);
        let flat_number = read("number").unwrap_or_default();
        let doc_type = read("type");

        // The number face re-reads from the input after the type token.
        let mut number = flat_number.clone();
        if let Some(doc_type) = &doc_type {
            let tail = input
                .strip_prefix("ETSI ")
                .and_then(|rest| rest.strip_prefix(doc_type))
                .unwrap_or("");
            if let Some(composed) = composed_number(tail, &flat_number) {
                number = composed;
            }
        }

        let parts = bound
            .get("parts")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| capture_in(item, "part").map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let (version, is_edition) = match (read("version"), read("edition")) {
            (Some(version), _) => (Some(version), false),
            (None, Some(edition)) => (Some(edition), true),
            _ => (None, false),
        };

        let mut current = EtsiIdentifier {
            kind: EtsiStandard,
            doc_type,
            number,
            minor: read("minor"),
            parts,
            version,
            is_edition,
            year: read("year"),
            month: read("month"),
            base: None,
        };
        // The supplements fold last-wraps-first (each entry wraps the
        // previous result).
        let supplements = bound
            .get("supplements")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for supplement in supplements {
            let kind = match optional_at(&supplement, "supplement_kind") {
                Some("/A") => Amendment,
                Some("/C") => Corrigendum,
                _ => continue,
            };
            let number = optional_at(&supplement, "supplement_number")
                .map(str::to_string)
                .unwrap_or_default();
            current = EtsiIdentifier {
                kind,
                doc_type: None,
                number,
                minor: None,
                parts: Vec::new(),
                version: None,
                is_edition: false,
                year: None,
                month: None,
                base: Some(Box::new(current)),
            };
        }
        Ok(current)
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(EtsiKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown etsi _type {:?}", hash.get("_type")),
            })?;
        let read = |field: &'static str| optional_hash_str(hash, field).map(str::to_string);
        // A document's number is a string face; a supplement's is an
        // INTEGER ordinal.
        let number = match hash.get("number") {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Number(value)) => value.to_string(),
            _ => {
                return Err(Error::MissingAttribute { capture: "number" });
            }
        };
        let base = match hash.get("base").cloned().filter(|base| !base.is_null()) {
            Some(base) => Some(Box::new(EtsiIdentifier::from_hash(&base)?)),
            None => None,
        };
        Ok(Self {
            kind,
            doc_type: read("type"),
            number,
            minor: read("minor"),
            parts: hash
                .get("parts")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            version: read("version"),
            is_edition: hash.get("is_edition") == Some(&Value::Bool(true)),
            year: read("year"),
            month: read("month"),
            base,
        })
    }

    /// The canonical hash: `_type`, the code faces, the flattened
    /// version, and the date; a supplement carries only its ordinal
    /// and the base.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        if self.kind == EtsiStandard {
            if let Some(doc_type) = &self.doc_type {
                object.insert("type".into(), doc_type.clone().into());
            }
            object.insert("number".into(), self.number.clone().into());
            if !self.parts.is_empty() {
                object.insert(
                    "parts".into(),
                    Value::Array(
                        self.parts.iter().map(|part| part.clone().into()).collect(),
                    ),
                );
            }
            if let Some(minor) = &self.minor {
                object.insert("minor".into(), minor.clone().into());
            }
            if let Some(version) = &self.version {
                object.insert("version".into(), version.clone().into());
            }
            if self.is_edition {
                object.insert("is_edition".into(), Value::Bool(true));
            }
            if let Some(year) = &self.year {
                object.insert("year".into(), year.clone().into());
            }
            if let Some(month) = &self.month {
                object.insert("month".into(), month.clone().into());
            }
        } else {
            object.insert(
                "number".into(),
                Value::Number(self.number.parse::<i64>().unwrap_or(0).into()),
            );
        }
        if let Some(base) = &self.base {
            object.insert("base".into(), base.to_hash());
        }
        Value::Object(object)
    }

    /// The canonical human.
    pub fn to_human(&self) -> String {
        if self.kind == EtsiStandard {
            let mut result = format!(
                "ETSI {} {}",
                self.doc_type.as_deref().unwrap_or(""),
                self.code()
            );
            if let Some(version) = &self.version {
                result.push(' ');
                result.push_str(&self.version_face(version));
            }
            if let Some(date) = self.date_face() {
                result.push_str(&format!(" ({date})"));
            }
            return result;
        }
        // The supplement render: the innermost base code plus the
        // chain of notations ("/A1/C2"), innermost first.
        let mut notations: Vec<String> = Vec::new();
        let mut current = Some(self);
        while let Some(identifier) = current {
            if let Some(base) = &identifier.base {
                notations.insert(0, format!("/{}", supplement_notation(identifier)));
                current = Some(base);
            } else {
                current = None;
            }
        }
        let base = self.actual_base();
        let mut result = format!(
            "ETSI {} {}{}",
            base.doc_type.as_deref().unwrap_or(""),
            base.code(),
            notations.join("")
        );
        if let Some(version) = &base.version {
            result.push(' ');
            result.push_str(&base.version_face(version));
        }
        if let Some(date) = base.date_face() {
            result.push_str(&format!(" ({date})"));
        }
        result
    }

    /// The URN: the base's type/code/version/date (a supplement's URN
    /// is its BASE's — the mirrored reference fall-through).
    pub fn to_urn(&self) -> String {
        let base = self.actual_base();
        let mut parts = vec!["urn".to_string(), "etsi".to_string()];
        if let Some(doc_type) = &base.doc_type {
            parts.push(doc_type.to_lowercase());
        }
        parts.push(base.code());
        if let Some(version) = &base.version {
            parts.push(base.version_face(version).to_lowercase());
        }
        if let Some(date) = base.date_face() {
            parts.push(date);
        }
        parts.join(":")
    }

    /// The Components::Code face: number, minor after a space, parts
    /// dashed.
    fn code(&self) -> String {
        let mut code = self.number.clone();
        if let Some(minor) = &self.minor {
            code.push(' ');
            code.push_str(minor);
        }
        for part in &self.parts {
            code.push('-');
            code.push_str(part);
        }
        code
    }

    /// The version face (`V1.5.1` or `ed.1`).
    fn version_face(&self, version: &str) -> String {
        if self.is_edition {
            format!("ed.{version}")
        } else {
            format!("V{version}")
        }
    }

    /// The date face (`2004-06`).
    fn date_face(&self) -> Option<String> {
        let year = self.year.as_deref()?;
        Some(match &self.month {
            Some(month) => format!("{year}-{month}"),
            None => year.to_string(),
        })
    }

    /// The innermost standard down the supplement chain.
    fn actual_base(&self) -> &EtsiIdentifier {
        let mut current = self;
        while let Some(base) = &current.base {
            current = base;
        }
        current
    }
}

/// The supplement notation (`A1`, `C2`).
fn supplement_notation(identifier: &EtsiIdentifier) -> String {
    let prefix = match identifier.kind {
        Corrigendum => "C",
        _ => "A",
    };
    format!("{prefix}{}", identifier.number)
}

#[cfg(test)]
mod tests {
    use super::{EtsiIdentifier, EtsiKind};
    use serde_json::json;

    #[test]
    fn the_spaced_number_keeps_its_face() {
        let id = EtsiIdentifier::parse("ETSI EG 200 053 V1.5.1 (2004-06)").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:etsi:etsi-standard",
            "type": "EG",
            "number": "200 053",
            "version": "1.5.1",
            "year": "2004",
            "month": "06",
        }));
        assert_eq!(id.to_human(), "ETSI EG 200 053 V1.5.1 (2004-06)");
        assert_eq!(id.to_urn(), "urn:etsi:eg:200 053:v1.5.1:2004-06");
    }

    #[test]
    fn the_edition_form_flags_is_edition() {
        let id = EtsiIdentifier::parse("ETSI ETR 108 ed.1 (1995-08)").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:etsi:etsi-standard",
            "type": "ETR",
            "number": "108",
            "version": "1",
            "is_edition": true,
            "year": "1995",
            "month": "08",
        }));
        assert_eq!(id.to_human(), "ETSI ETR 108 ed.1 (1995-08)");
        assert_eq!(id.to_urn(), "urn:etsi:etr:108:ed.1:1995-08");
    }

    #[test]
    fn the_alfa_prefix_number_composes() {
        let id = EtsiIdentifier::parse("ETSI GS NFV-IFA 024 V2.1.1 (2018-02)").expect("parse");
        assert_eq!(id.number, "NFV-IFA 024");
        assert_eq!(id.to_human(), "ETSI GS NFV-IFA 024 V2.1.1 (2018-02)");

        let arf = EtsiIdentifier::parse("ETSI GR ARF 004-1 V1.2.1 (2023-02)").expect("parse");
        assert_eq!(arf.number, "ARF 004");
        assert_eq!(arf.parts, vec!["1"]);
    }

    #[test]
    fn the_supplement_chain_renders_innermost_first() {
        let id = EtsiIdentifier::parse("ETSI ETS 300 011-3/A1/C2 ed.1 (1994-12)").expect("parse");
        assert_eq!(id.kind, EtsiKind::Corrigendum);
        assert_eq!(id.to_human(), "ETSI ETS 300 011-3/A1/C2 ed.1 (1994-12)");
        assert_eq!(id.to_urn(), "urn:etsi:ets:300 011-3:ed.1:1994-12");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:etsi:corrigendum",
                "number": 2,
                "base": {
                    "_type": "pubid:etsi:amendment",
                    "number": 1,
                    "base": {
                        "_type": "pubid:etsi:etsi-standard",
                        "type": "ETS",
                        "number": "300 011",
                        "parts": ["3"],
                        "version": "1",
                        "is_edition": true,
                        "year": "1994",
                        "month": "12",
                    },
                },
            })
        );
    }

    #[test]
    fn the_single_amendment_folds_onto_its_base() {
        let id = EtsiIdentifier::parse("ETSI ETR 108/A1 ed.1 (1995-08)").expect("parse");
        assert_eq!(id.to_human(), "ETSI ETR 108/A1 ed.1 (1995-08)");
        assert_eq!(id.to_hash()["number"], json!(1));
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "ETSI EG 200 053 V1.5.1 (2004-06)",
            "ETSI ETR 108 ed.1 (1995-08)",
            "ETSI EN 300 175-1 V2.4.1 (2019-06)",
            "ETSI ETS 300 011-3/A1/C2 ed.1 (1994-12)",
        ] {
            let id = EtsiIdentifier::parse(spelling).expect(spelling);
            let rebuilt = EtsiIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{spelling}");
        }
    }
}
