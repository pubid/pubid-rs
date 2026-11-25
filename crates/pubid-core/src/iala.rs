//! The iala flavor: the International Organization for Marine Aids to
//! Navigation's identifier space, ported from the gem's `Pubid::Iala`
//! (the reference is pubid a604fc07 — the codeless slug publications
//! are gem #772, the S-2xx product specifications gem #777; the parse
//! runs through the vendored `iala.json` PARG envelope, grammar 0.2.0,
//! whose `slug_identifier` sits LAST in the ordered choice so every
//! typed code keeps priority).
//!
//! Three families share one type-dispatched model:
//!
//!   IALA S1070 Ed 2.0                the legacy zero-padded series
//!                                    (S/R/G/M/C, plus the reserved
//!                                    X/P and the L letters — width 4,
//!                                    L verbatim)
//!   IALA GA01.01                     the GA resolutions (width 2, each
//!                                    dotted segment width 2)
//!   IALA A12-01                      the legacy A-session form — the
//!                                    dashed numbers render the A
//!                                    letter, and parse back to GA
//!   IALA C0103-1 Ed 3.0              the dash subpart
//!   IALA G1128 ANNEX A Ed 1.6        the annex wrapper over a typed
//!                                    base (marker case preserved)
//!   S-201 2.0.0                      the S-100-family product
//!                                    specification: the IHO-granted
//!                                    S-201..S-299 range, hyphenated
//!                                    code, the dotted x.y.z edition
//!                                    on the BARE face (gem #777)
//!   IALA arctic-seminar-report (E)   the codeless slug publication —
//!                                    the slug is `number`, the `IALA `
//!                                    prefix is mandatory, no MRN is
//!                                    assigned (gem #772)
//!
//! The grant bounds the grammar (`s100_number = (!"200" "2" digit
//! digit)`): S-200 is excluded ("spans from S-201") and every other
//! hundreds digit is another org's grant, so the six grant-bound
//! rejects (`S-200`, `S-411` — WMO's, `S-101` — IHO's, `S-30`,
//! `S-2011`, `S-201 Ed`) fail at the parse. This is the pubid#469/#763
//! dual-perspective law: the lexical S form leads no prefix and each
//! org assigns its own identifier, so the reference's router hands the
//! bare S-2xx strings to IALA (its exact-round-trip parse wins) while
//! WMO keeps S-411/S-412 and IHO keeps the prefixed `IHO S-2xx` faces —
//! this engine has no umbrella router (each flavor parses explicitly),
//! so the reclaim shows up as the exact round trip below plus the
//! flavor's negative ledger.
//!
//! The MRN URNs follow the official scheme of IALA Recommendation
//! R1023 (Maritime Resource Names) and Guideline G1143 (whose Annex A4
//! names `urn:mrn:iala:pub:<managed name space>`):
//!
//!   urn:mrn:iala:pub:<code>[:annex[-<letter>]][:ed<edition>][:<lang>]
//!
//! with the S-100-family codes hyphenated (`s-201`) and the slug
//! publications carrying `to_urn = nil` — IALA assigns no MRN to them.

use serde_json::Value;

use crate::artifact::iala_artifact;
use crate::bound::{bound_str, capture_in, hash_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// The concrete IALA class (`Pubid::Iala::Identifiers::*` in the
/// reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IalaKind {
    /// The zero-padded S series (`IALA S1070 Ed 2.0`).
    Standard,
    /// The S-100-family product specification (`S-201 2.0.0`).
    ProductSpecification,
    /// The R series (`IALA R0126 Ed 2.0`).
    Recommendation,
    /// The G series (`IALA G1164 Ed 1.1`).
    Guideline,
    /// The M series (`IALA M0001 Ed 9.0`).
    Manual,
    /// The C series (`IALA C0103-1 Ed 3.0`).
    ModelCourse,
    /// The GA resolutions (`IALA GA01.01`); the legacy dashed A-session
    /// numbers (`IALA A12-01`) are the same class.
    GeneralAssembly,
    /// The L letters (`IALA L2.1.11 Ed 2`) — numbering verbatim.
    Letter,
    /// The reserved X series (`IALA X0123`).
    Report,
    /// The reserved P series (`IALA P0123`).
    Resolution,
    /// An annex wrapper over a typed or S-100 base.
    Annex,
    /// A codeless publication — the product-URL slug is the identity.
    Publication,
}

impl IalaKind {
    /// The shared `_type` prefix every iala tag carries.
    pub const KIND_PREFIX: &'static str = "pubid:iala:";

    /// The polymorphic type tag carried in the canonical hash (the
    /// reference's `IALA_TYPE_MAP`).
    pub fn type_tag(self) -> &'static str {
        match self {
            IalaKind::Standard => "pubid:iala:standard",
            IalaKind::ProductSpecification => "pubid:iala:product-specification",
            IalaKind::Recommendation => "pubid:iala:recommendation",
            IalaKind::Guideline => "pubid:iala:guideline",
            IalaKind::Manual => "pubid:iala:manual",
            IalaKind::ModelCourse => "pubid:iala:model-course",
            IalaKind::GeneralAssembly => "pubid:iala:general-assembly",
            IalaKind::Letter => "pubid:iala:letter",
            IalaKind::Report => "pubid:iala:report",
            IalaKind::Resolution => "pubid:iala:resolution",
            IalaKind::Annex => "pubid:iala:annex",
            IalaKind::Publication => "pubid:iala:publication",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        match tag {
            "pubid:iala:standard" => Some(IalaKind::Standard),
            "pubid:iala:product-specification" => Some(IalaKind::ProductSpecification),
            "pubid:iala:recommendation" => Some(IalaKind::Recommendation),
            "pubid:iala:guideline" => Some(IalaKind::Guideline),
            "pubid:iala:manual" => Some(IalaKind::Manual),
            "pubid:iala:model-course" => Some(IalaKind::ModelCourse),
            "pubid:iala:general-assembly" => Some(IalaKind::GeneralAssembly),
            "pubid:iala:letter" => Some(IalaKind::Letter),
            "pubid:iala:report" => Some(IalaKind::Report),
            "pubid:iala:resolution" => Some(IalaKind::Resolution),
            "pubid:iala:annex" => Some(IalaKind::Annex),
            "pubid:iala:publication" => Some(IalaKind::Publication),
            _ => None,
        }
    }

    /// The class a type letter routes to (`Iala
    /// .identifier_klass_for_type_letter`): the series letters, the GA
    /// head, and the legacy A-session alias that resolves to
    /// GeneralAssembly. The letterless classes (ProductSpecification,
    /// Annex, Publication) are not in this table.
    fn from_letter(letter: &str) -> Option<Self> {
        match letter {
            "S" => Some(IalaKind::Standard),
            "R" => Some(IalaKind::Recommendation),
            "G" => Some(IalaKind::Guideline),
            "M" => Some(IalaKind::Manual),
            "C" => Some(IalaKind::ModelCourse),
            "GA" | "A" => Some(IalaKind::GeneralAssembly),
            "L" => Some(IalaKind::Letter),
            "X" => Some(IalaKind::Report),
            "P" => Some(IalaKind::Resolution),
            _ => None,
        }
    }

    /// The static type letter; GeneralAssembly resolves at runtime (the
    /// dashed numbers carry the legacy A), and the letterless classes
    /// have none (their code faces are rendered directly).
    fn static_letter(self) -> Option<&'static str> {
        match self {
            IalaKind::Standard => Some("S"),
            IalaKind::Recommendation => Some("R"),
            IalaKind::Guideline => Some("G"),
            IalaKind::Manual => Some("M"),
            IalaKind::ModelCourse => Some("C"),
            IalaKind::Letter => Some("L"),
            IalaKind::Report => Some("X"),
            IalaKind::Resolution => Some("P"),
            IalaKind::GeneralAssembly
            | IalaKind::ProductSpecification
            | IalaKind::Annex
            | IalaKind::Publication => None,
        }
    }

    /// The canonical zero-pad width of the leading number
    /// (`canonical_number_width`): 4 for the numbered series, 2 for GA,
    /// nil — preserve verbatim — for the L letters and the letterless
    /// classes.
    fn number_width(self) -> Option<usize> {
        match self {
            IalaKind::Standard
            | IalaKind::Recommendation
            | IalaKind::Guideline
            | IalaKind::Manual
            | IalaKind::ModelCourse
            | IalaKind::Report
            | IalaKind::Resolution => Some(4),
            IalaKind::GeneralAssembly => Some(2),
            IalaKind::Letter
            | IalaKind::ProductSpecification
            | IalaKind::Annex
            | IalaKind::Publication => None,
        }
    }

    /// The zero-pad width of each dotted continuation segment
    /// (`canonical_dotted_segment_width`): only GA pads (its
    /// series.index form `01.01` is the canonical cover-page shape).
    fn dotted_segment_width(self) -> Option<usize> {
        match self {
            IalaKind::GeneralAssembly => Some(2),
            _ => None,
        }
    }
}

/// The iala identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IalaIdentifier {
    /// The concrete class (drives `_type`, the human face and the URN
    /// code segment).
    pub kind: IalaKind,
    /// The document number: the zero-padded typed code (`"1070"`,
    /// `"01.01"`, `"0103-1"`, `"2.7.1-2"`), the S-100 digits (`"201"`),
    /// or the codeless slug. On an annex this is the BASE's number —
    /// the grammar binds one number per match and the reference hoists
    /// it into the base.
    pub number: String,
    /// The edition, verbatim (`"2.0"`, `"2"`, the S-100 `"2.0.0"`).
    pub edition: Option<String>,
    /// The language letter (`"E"`, `"F"`, `"S"`, `"C"`, `"A"`, `"R"`).
    pub language: Option<String>,
    /// Annex only: the base's concrete class (the base carries only its
    /// designation, never an edition or language).
    pub base_kind: Option<IalaKind>,
    /// Annex only: the marker as spelled (`"Annex"` / `"ANNEX"`),
    /// preserved verbatim for round-trip fidelity.
    pub annex_form: Option<String>,
    /// Annex only: the letter with its optional sub-number (`"A"`,
    /// `"B2"`); absent on the bare form.
    pub letter: Option<String>,
}

impl IalaIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// The MRN namespace the URNs live under
    /// (`Pubid::Iala::UrnParser::PREFIX`).
    const MRN_PREFIX: &'static str = "urn:mrn:iala:pub:";

    /// Parse an input spelling — a human form or an MRN URN — into an
    /// identifier. The `urn:` prefix dispatches to the URN path (the
    /// reference's `FormatDetector`).
    pub fn parse(input: &str) -> Result<Self> {
        if input.len() >= 4 && input.as_bytes()[..4].eq_ignore_ascii_case(b"urn:") {
            Self::parse_urn(input)
        } else {
            Self::parse_human(input)
        }
    }

    /// Parse a human spelling through the vendored artifact and
    /// dispatch the class off the resolved captures: `annex_marker`
    /// marks an annex, `s100_number` a product specification, `slug` a
    /// codeless publication, the typed `type_letter` otherwise (the
    /// builder's dispatch order; `slug_identifier` is the grammar's
    /// last alternative, so every typed code keeps priority).
    fn parse_human(input: &str) -> Result<Self> {
        let artifact = iala_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Self::from_bound(&bound)
    }

    /// Build from the artifact's flat bound map (the grammar's
    /// bindings block; the annex base composite stays unbound, its
    /// captures hoisted to the top level).
    fn from_bound(bound: &Value) -> Result<Self> {
        let edition = optional_bound_str(bound, "edition_value").map(str::to_string);
        let language =
            optional_bound_str(bound, "language").map(|letter| letter.to_ascii_uppercase());

        if let Some(annex_form) = optional_bound_str(bound, "annex_marker") {
            let (base_kind, number) = if let Some(s100) = optional_bound_str(bound, "s100_number") {
                (Some(IalaKind::ProductSpecification), s100.to_string())
            } else {
                let kind = Self::kind_for_letter(bound)?;
                (Some(kind), Self::compose_number(bound, kind)?)
            };
            Ok(Self {
                kind: IalaKind::Annex,
                number,
                edition,
                language,
                base_kind,
                annex_form: Some(annex_form.to_string()),
                letter: optional_bound_str(bound, "annex_letter").map(str::to_string),
            })
        } else if let Some(s100) = optional_bound_str(bound, "s100_number") {
            Ok(Self {
                kind: IalaKind::ProductSpecification,
                number: s100.to_string(),
                edition,
                language,
                base_kind: None,
                annex_form: None,
                letter: None,
            })
        } else if let Some(slug) = optional_bound_str(bound, "slug") {
            Ok(Self {
                kind: IalaKind::Publication,
                number: slug.to_string(),
                edition,
                language,
                base_kind: None,
                annex_form: None,
                letter: None,
            })
        } else {
            let kind = Self::kind_for_letter(bound)?;
            Ok(Self {
                kind,
                number: Self::compose_number(bound, kind)?,
                edition,
                language,
                base_kind: None,
                annex_form: None,
                letter: None,
            })
        }
    }

    /// The class a bound `type_letter` routes to, upcased like the
    /// builder's lookup.
    fn kind_for_letter(bound: &Value) -> Result<IalaKind> {
        let letter = bound_str(bound, "type_letter")?.to_ascii_uppercase();
        IalaKind::from_letter(&letter).ok_or_else(|| Error::InvalidHash {
            detail: format!("unknown iala type letter {letter:?}"),
        })
    }

    /// Compose the number from the bound map's three columns: the
    /// leading `doc_number` (zero-padded to the class's width), the
    /// dotted continuation segments (`doc_number_dots[].dotted_segment`,
    /// each padded to the class's dotted width) and the dash subpart
    /// runs (`subpart[].subpart_number`). GA01.13 → `"01.13"`,
    /// L2.7.1-2 → `"2.7.1-2"`, C0103-1 → `"0103-1"`.
    fn compose_number(bound: &Value, kind: IalaKind) -> Result<String> {
        let base = bound_str(bound, "doc_number")?;
        let mut out = pad_left(base, kind.number_width());

        let dots: Vec<String> = match kind.dotted_segment_width() {
            Some(width) => Self::segments(bound, "doc_number_dots", "dotted_segment")
                .into_iter()
                .map(|segment| pad_left(&segment, Some(width)))
                .collect(),
            None => Self::segments(bound, "doc_number_dots", "dotted_segment"),
        };
        if !dots.is_empty() {
            out.push('.');
            out.push_str(&dots.join("."));
        }

        let subparts = Self::segments(bound, "subpart", "subpart_number");
        if !subparts.is_empty() {
            out.push('-');
            out.push_str(&subparts.join("-"));
        }
        Ok(out)
    }

    /// The string values of a bound repetition column:
    /// `doc_number_dots` carries `{dotted_segment: …}` elements,
    /// `subpart` carries `{subpart_number: …}` elements.
    fn segments(bound: &Value, column: &'static str, key: &'static str) -> Vec<String> {
        bound
            .get(column)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| capture_in(item, key).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The type letter this identifier renders with: GeneralAssembly
    /// resolves at runtime — the dashed numbers are the legacy A-session
    /// form (`A12-01`), the dotted numbers the canonical GA — the other
    /// typed classes carry their static letter, and the letterless
    /// classes render none.
    fn type_letter(&self) -> Option<String> {
        match self.kind {
            IalaKind::GeneralAssembly => {
                Some(if self.number.contains('-') { "A" } else { "GA" }.to_string())
            }
            kind => kind.static_letter().map(str::to_string),
        }
    }

    /// Parse an MRN URN (`Pubid::Iala::UrnParser`): the namespace
    /// prefix, one code segment, then the optional annex, edition and
    /// language segments. Matching is caseless in the MRN (G1143: the
    /// MRN is case-insensitive) and every extracted value re-cases
    /// deterministically, so the lowercased body is equivalent. The
    /// segments rebuild the human form, which re-parses through the
    /// artifact — the reference's exact shape (one construction path);
    /// the R1026 PDF's malformed editions (`Ed1.0`, `ed.1.0`) normalize
    /// here. Unknown segments are silently ignored, like the reference's
    /// case statement (no else branch) — the grammar re-validates
    /// whatever the rebuild produces.
    fn parse_urn(urn: &str) -> Result<Self> {
        let lowered = urn.to_lowercase();
        let body = lowered
            .strip_prefix(Self::MRN_PREFIX)
            .ok_or_else(|| urn_error(format!("invalid IALA URN: {urn:?}")))?;
        let parts: Vec<&str> = body.split(':').collect();

        let mut annex_form: Option<&'static str> = None;
        let mut annex_letter: Option<String> = None;
        let mut edition: Option<String> = None;
        let mut language: Option<String> = None;
        for segment in &parts[1..] {
            if let Some(letter) = annex_segment(segment) {
                // The generator's casing: the lettered form re-renders
                // ANNEX, the bare one Annex.
                annex_form = Some(if letter.is_some() { "ANNEX" } else { "Annex" });
                annex_letter = letter;
            } else if let Some(rest) = segment.strip_prefix("ed") {
                let value = rest.strip_prefix('.').unwrap_or(rest);
                edition = Some(value.to_string());
            } else if segment.len() == 1 && segment.as_bytes()[0].is_ascii_lowercase() {
                language = Some(segment.to_ascii_uppercase());
            }
        }

        // The rebuild in the reference's face order: code, annex form,
        // letter, edition, language.
        let mut text = format!("IALA {}", parts[0].to_ascii_uppercase());
        if let Some(form) = annex_form {
            text.push(' ');
            text.push_str(form);
        }
        if let Some(letter) = &annex_letter {
            text.push(' ');
            text.push_str(letter);
        }
        if let Some(edition) = &edition {
            // The S-100-family codes carry the bare dotted edition on
            // the human face; the rebuilt text names the edition
            // through the grammar's "Ed" alias and the renderer
            // restores the bare face.
            text.push_str(" Ed ");
            text.push_str(edition);
        }
        if let Some(language) = &language {
            text.push_str(&format!(" ({language})"));
        }
        Self::parse_human(&text)
    }

    /// Build from a canonical hash (`_type`-tagged attribute record;
    /// the annex nests its base designation).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(IalaKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown iala _type {:?}", hash.get("_type")),
            })?;
        if kind == IalaKind::Annex {
            let base = hash.get("base").ok_or_else(|| Error::InvalidHash {
                detail: "annex has no base".to_string(),
            })?;
            let base_kind = base
                .get("_type")
                .and_then(Value::as_str)
                .and_then(IalaKind::from_type_tag)
                .ok_or_else(|| Error::InvalidHash {
                    detail: format!("annex base has unknown _type {:?}", base.get("_type")),
                })?;
            Ok(Self {
                kind,
                number: hash_str(base, "number")?.to_string(),
                edition: optional_hash_str(hash, "edition").map(str::to_string),
                language: optional_hash_str(hash, "language").map(str::to_string),
                base_kind: Some(base_kind),
                annex_form: Some(hash_str(hash, "annex_form")?.to_string()),
                letter: optional_hash_str(hash, "letter").map(str::to_string),
            })
        } else {
            Ok(Self {
                kind,
                number: hash_str(hash, "number")?.to_string(),
                edition: optional_hash_str(hash, "edition").map(str::to_string),
                language: optional_hash_str(hash, "language").map(str::to_string),
                base_kind: None,
                annex_form: None,
                letter: None,
            })
        }
    }

    /// The canonical hash. The publisher default (`IALA`) never
    /// serializes — `IALA ` is the citation prefix and the S-100 `S-`
    /// designation is a code face, neither an attribute; absent
    /// optionals drop. The annex nests the base designation.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        if self.kind == IalaKind::Annex {
            let base_kind = self.base_kind.unwrap_or(self.kind);
            object.insert(
                "base".into(),
                serde_json::json!({
                    "_type": base_kind.type_tag(),
                    "number": self.number,
                }),
            );
            object.insert(
                "annex_form".into(),
                self.annex_form.clone().unwrap_or_default().into(),
            );
            if let Some(letter) = &self.letter {
                object.insert("letter".into(), letter.clone().into());
            }
        } else {
            object.insert("number".into(), self.number.clone().into());
        }
        if let Some(edition) = &self.edition {
            object.insert("edition".into(), edition.clone().into());
        }
        if let Some(language) = &self.language {
            object.insert("language".into(), language.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human (the reference's `Renderer`). The typed
    /// faces carry the mandatory `IALA ` prefix; the S-100 product
    /// specification prints the bare designation (no publisher token —
    /// `publisher_prefixes` is `[]`, the WMO S-100 rule); the codeless
    /// publication prints the slug in the letter slot. The annex face
    /// order is the reference's: base, marker, letter, edition (bare
    /// for an S-100-family base, `Ed` for the legacy series), language.
    pub fn to_human(&self) -> String {
        let mut out = match self.kind {
            IalaKind::Annex => {
                let base = Self {
                    kind: self.base_kind.unwrap_or(self.kind),
                    number: self.number.clone(),
                    edition: None,
                    language: None,
                    base_kind: None,
                    annex_form: None,
                    letter: None,
                };
                let mut out = base.to_human();
                out.push(' ');
                out.push_str(self.annex_form.as_deref().unwrap_or("Annex"));
                if let Some(letter) = &self.letter {
                    out.push(' ');
                    out.push_str(letter);
                }
                out
            }
            IalaKind::ProductSpecification => format!("S-{}", self.number),
            IalaKind::Publication => format!("IALA {}", self.number),
            kind => {
                let letter = kind
                    .static_letter()
                    .map(str::to_string)
                    .or_else(|| self.type_letter())
                    .unwrap_or_default();
                format!("IALA {letter}{}", self.number)
            }
        };
        if let Some(edition) = &self.edition {
            match self.kind {
                // The annex face follows the base's family; the S-100
                // specification carries the bare dotted edition.
                IalaKind::Annex if self.base_kind == Some(IalaKind::ProductSpecification) => {
                    out.push(' ');
                    out.push_str(edition);
                }
                IalaKind::ProductSpecification => {
                    out.push(' ');
                    out.push_str(edition);
                }
                _ => {
                    out.push_str(" Ed ");
                    out.push_str(edition);
                }
            }
        }
        if let Some(language) = &self.language {
            out.push_str(&format!(" ({language})"));
        }
        out
    }

    /// The MRN URN (`Pubid::Wmo::UrnGenerator` shape — the official
    /// R1023/G1143 scheme): the lowercased code segment (`s1070`,
    /// `s-201`, `ga01.01`, `a12-01`), the annex segment (`annex` /
    /// `annex-<letter>`), then `ed<edition>` and the language letter.
    /// A codeless publication carries NO URN — IALA assigns no MRN to
    /// it (`Identifiers::Publication#to_urn` is nil), so `None`.
    pub fn to_urn(&self) -> Option<String> {
        if self.kind == IalaKind::Publication {
            return None;
        }
        let mut parts = vec!["urn:mrn:iala:pub".to_string(), self.code_segment()];
        if self.kind == IalaKind::Annex {
            match &self.letter {
                Some(letter) => parts.push(format!("annex-{}", letter.to_lowercase())),
                None => parts.push("annex".to_string()),
            }
        }
        if let Some(edition) = &self.edition {
            parts.push(format!("ed{edition}"));
        }
        if let Some(language) = &self.language {
            parts.push(language.to_lowercase());
        }
        Some(parts.join(":"))
    }

    /// The lowercase document code in IALA's managed namespace (the
    /// legacy codes join the letter and the number — `s1070`, `ga01.01`
    /// — and the S-100-family codes hyphenate — `s-201`). On an annex
    /// the code names the BASE.
    fn code_segment(&self) -> String {
        let kind = match self.kind {
            IalaKind::Annex => self.base_kind.unwrap_or(self.kind),
            kind => kind,
        };
        if kind == IalaKind::ProductSpecification {
            return format!("s-{}", self.number);
        }
        let letter = match kind {
            IalaKind::GeneralAssembly => {
                if self.number.contains('-') {
                    "A"
                } else {
                    "GA"
                }
            }
            kind => kind.static_letter().unwrap_or_default(),
        };
        format!("{}{}", letter.to_lowercase(), self.number)
    }

    /// Whether this identifier renders the bare S-100 face (the
    /// product specification, and the annex over one): the reference's
    /// `publisher_prefixes == []` classes.
    pub fn is_s100_family(&self) -> bool {
        self.kind == IalaKind::ProductSpecification
            || (self.kind == IalaKind::Annex
                && self.base_kind == Some(IalaKind::ProductSpecification))
    }
}

/// The Ruby `String#rjust(width, "0")` on the numeric runs: shorter
/// runs zero-pad to the width, longer and unpadded inputs stay
/// verbatim.
fn pad_left(text: &str, width: Option<usize>) -> String {
    match width {
        Some(width) if text.len() < width => {
            let mut out = "0".repeat(width - text.len());
            out.push_str(text);
            out
        }
        _ => text.to_string(),
    }
}

/// An `annex` / `annex-<letter>` URN segment, matching the reference's
/// `/\Aannex(-([a-z]\d?))?\z/i`: `Some(None)` is the bare form,
/// `Some(Some(letter))` the lettered one (one lowercase letter with an
/// optional sub-number digit, upcased for the attested face), `None`
/// when the segment is not an annex at all — in which case the
/// reference's case statement ignores it.
fn annex_segment(segment: &str) -> Option<Option<String>> {
    let rest = segment.strip_prefix("annex")?;
    let Some(letter) = rest.strip_prefix('-') else {
        return rest.is_empty().then_some(None);
    };
    let bytes = letter.as_bytes();
    let well_formed = !bytes.is_empty()
        && bytes.len() <= 2
        && bytes[0].is_ascii_lowercase()
        && (bytes.len() == 1 || bytes[1].is_ascii_digit());
    well_formed.then(|| Some(letter.to_ascii_uppercase()))
}

fn urn_error(detail: impl Into<String>) -> Error {
    Error::Urn(detail.into())
}

#[cfg(test)]
mod tests {
    use super::IalaIdentifier;
    use crate::iala::IalaKind;
    use crate::iho::IhoIdentifier;
    use crate::wmo::WmoIdentifier;
    use serde_json::json;

    /// The URN → human shape table (the generator/parser pair over
    /// every class), shared by the parse and round-trip tests.
    const MRN_SHAPE_URN_EXAMPLES: &[(&str, &str)] = &[
        ("urn:mrn:iala:pub:s1070", "IALA S1070"),
        ("urn:mrn:iala:pub:s1070:ed2.0", "IALA S1070 Ed 2.0"),
        ("urn:mrn:iala:pub:r1016:ed2.0:f", "IALA R1016 Ed 2.0 (F)"),
        ("urn:mrn:iala:pub:c0103-1", "IALA C0103-1"),
        ("urn:mrn:iala:pub:c0103-1:ed3.0", "IALA C0103-1 Ed 3.0"),
        ("urn:mrn:iala:pub:m0001:ed9.0", "IALA M0001 Ed 9.0"),
        ("urn:mrn:iala:pub:a12-01", "IALA A12-01"),
        ("urn:mrn:iala:pub:ga01.01", "IALA GA01.01"),
        ("urn:mrn:iala:pub:l2.1.11:ed2", "IALA L2.1.11 Ed 2"),
        ("urn:mrn:iala:pub:g1045:annex:ed1", "IALA G1045 Annex Ed 1"),
        (
            "urn:mrn:iala:pub:g1128:annex-a:ed1.6",
            "IALA G1128 ANNEX A Ed 1.6",
        ),
        (
            "urn:mrn:iala:pub:g1128:annex-d:ed1.6:e",
            "IALA G1128 ANNEX D Ed 1.6 (E)",
        ),
        ("urn:mrn:iala:pub:s-201", "S-201"),
        ("urn:mrn:iala:pub:s-201:ed2.0.0", "S-201 2.0.0"),
        ("urn:mrn:iala:pub:s-230:ed1.0.0:e", "S-230 1.0.0 (E)"),
        ("urn:mrn:iala:pub:s-201:annex-b2", "S-201 ANNEX B2"),
        (
            "urn:mrn:iala:pub:s-201:annex-a:ed1.0.0",
            "S-201 ANNEX A 1.0.0",
        ),
        // The published corpus URNs (mn-samples-iala reference-docs).
        ("urn:mrn:iala:pub:s1040:ed2.0", "IALA S1040 Ed 2.0"),
        ("urn:mrn:iala:pub:r1023:ed1.0", "IALA R1023 Ed 1.0"),
        ("urn:mrn:iala:pub:g1195:ed1.0", "IALA G1195 Ed 1.0"),
    ];

    #[test]
    fn parses_the_typed_series() {
        for (input, kind, number, canonical) in [
            ("S1070", IalaKind::Standard, "1070", "IALA S1070"),
            (
                "S1070 Ed 2.0",
                IalaKind::Standard,
                "1070",
                "IALA S1070 Ed 2.0",
            ),
            (
                "IALA S1070 Ed 2.0",
                IalaKind::Standard,
                "1070",
                "IALA S1070 Ed 2.0",
            ),
            (
                "R0126 Ed 2.0",
                IalaKind::Recommendation,
                "0126",
                "IALA R0126 Ed 2.0",
            ),
            (
                "R1016:ed2.0(F)",
                IalaKind::Recommendation,
                "1016",
                "IALA R1016 Ed 2.0 (F)",
            ),
            (
                "G1015 Ed 2.2",
                IalaKind::Guideline,
                "1015",
                "IALA G1015 Ed 2.2",
            ),
            (
                "C0103-1 Ed 3.0",
                IalaKind::ModelCourse,
                "0103-1",
                "IALA C0103-1 Ed 3.0",
            ),
            (
                "IALA M0001 Ed 9.0",
                IalaKind::Manual,
                "0001",
                "IALA M0001 Ed 9.0",
            ),
            ("IALA X0123", IalaKind::Report, "0123", "IALA X0123"),
            ("IALA P0123", IalaKind::Resolution, "0123", "IALA P0123"),
            (
                "IALA L2.1.11 Ed 2",
                IalaKind::Letter,
                "2.1.11",
                "IALA L2.1.11 Ed 2",
            ),
        ] {
            let id = IalaIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, kind, "{input}");
            assert_eq!(id.number, number, "{input}");
            assert_eq!(id.to_human(), canonical, "{input}");
        }
        let f = IalaIdentifier::parse("R1016:ed2.0(F)").expect("parse");
        assert_eq!(f.edition.as_deref(), Some("2.0"));
        assert_eq!(f.language.as_deref(), Some("F"));
    }

    #[test]
    fn the_short_inputs_zero_pad_to_the_canonical_widths() {
        assert_eq!(IalaIdentifier::parse("M1").expect("parse").number, "0001");
        assert_eq!(
            IalaIdentifier::parse("C103-1").expect("parse").number,
            "0103-1"
        );
        assert_eq!(
            IalaIdentifier::parse("IALA GA1.1").expect("parse").number,
            "01.01"
        );
        // The L series preserves its numbering verbatim.
        assert_eq!(
            IalaIdentifier::parse("IALA L2.7.1-2 Ed 2")
                .expect("parse")
                .number,
            "2.7.1-2"
        );
    }

    #[test]
    fn the_general_assembly_forms_share_one_class() {
        let ga = IalaIdentifier::parse("IALA GA01.13").expect("parse");
        assert_eq!(ga.kind, IalaKind::GeneralAssembly);
        assert_eq!(ga.number, "01.13");
        assert_eq!(ga.to_human(), "IALA GA01.13");
        assert_eq!(ga.to_urn().as_deref(), Some("urn:mrn:iala:pub:ga01.13"));

        // The legacy dashed A-session form renders the A letter and is
        // the same class.
        let a = IalaIdentifier::parse("IALA A12-01").expect("parse");
        assert_eq!(a.kind, IalaKind::GeneralAssembly);
        assert_eq!(a.number, "12-01");
        assert_eq!(a.to_human(), "IALA A12-01");
        assert_eq!(a.to_urn().as_deref(), Some("urn:mrn:iala:pub:a12-01"));
    }

    #[test]
    fn parses_the_annex_wrappers() {
        let lettered = IalaIdentifier::parse("IALA G1128 ANNEX A Ed 1.6").expect("parse");
        assert_eq!(lettered.kind, IalaKind::Annex);
        assert_eq!(lettered.base_kind, Some(IalaKind::Guideline));
        assert_eq!(lettered.number, "1128");
        assert_eq!(lettered.annex_form.as_deref(), Some("ANNEX"));
        assert_eq!(lettered.letter.as_deref(), Some("A"));
        assert_eq!(lettered.edition.as_deref(), Some("1.6"));
        assert_eq!(lettered.to_human(), "IALA G1128 ANNEX A Ed 1.6");
        assert_eq!(
            lettered.to_urn().as_deref(),
            Some("urn:mrn:iala:pub:g1128:annex-a:ed1.6")
        );
        assert_eq!(
            lettered.to_hash(),
            json!({
                "_type": "pubid:iala:annex",
                "edition": "1.6",
                "base": { "_type": "pubid:iala:guideline", "number": "1128" },
                "annex_form": "ANNEX",
                "letter": "A",
            })
        );

        let bare = IalaIdentifier::parse("IALA G1045 Annex Ed 1").expect("parse");
        assert_eq!(bare.annex_form.as_deref(), Some("Annex"));
        assert_eq!(bare.letter, None);
        assert_eq!(bare.to_human(), "IALA G1045 Annex Ed 1");
        assert_eq!(
            bare.to_urn().as_deref(),
            Some("urn:mrn:iala:pub:g1045:annex:ed1")
        );

        let language = IalaIdentifier::parse("IALA G1128 ANNEX D Ed 1.6 (E)").expect("parse");
        assert_eq!(language.to_human(), "IALA G1128 ANNEX D Ed 1.6 (E)");
        assert_eq!(
            language.to_urn().as_deref(),
            Some("urn:mrn:iala:pub:g1128:annex-d:ed1.6:e")
        );
    }

    #[test]
    fn parses_the_s100_product_specifications() {
        // The granted range (iala.int's dev-status pages), the bare
        // citation face, and the dotted x.y.z edition verbatim.
        for (input, number, edition, language) in [
            ("S-201", "201", None, None),
            ("S-201 2.0.0", "201", Some("2.0.0"), None),
            ("S-201 0.0.7", "201", Some("0.0.7"), None),
            ("S-230 1.0.0", "230", Some("1.0.0"), None),
            ("S-247 0.2.0 (E)", "247", Some("0.2.0"), Some("E")),
        ] {
            let id = IalaIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, IalaKind::ProductSpecification, "{input}");
            assert_eq!(id.number, number, "{input}");
            assert_eq!(id.edition.as_deref(), edition, "{input}");
            assert_eq!(id.language.as_deref(), language, "{input}");
            assert_eq!(id.to_human(), input, "{input}");
        }

        // The MRN face carries the dotted edition on the ed marker.
        assert_eq!(
            IalaIdentifier::parse("S-201 2.0.0")
                .expect("parse")
                .to_urn()
                .as_deref(),
            Some("urn:mrn:iala:pub:s-201:ed2.0.0")
        );
        assert_eq!(
            IalaIdentifier::parse("S-247 0.2.0 (E)")
                .expect("parse")
                .to_urn()
                .as_deref(),
            Some("urn:mrn:iala:pub:s-247:ed0.2.0:e")
        );
    }

    #[test]
    fn the_prefixed_s100_spellings_normalize_to_the_bare_face() {
        // The designation carries no publisher token, so the "IALA "
        // prefix, the "Ed" marker, the compact ":ed" form and the
        // lower-case code spelling all parse to the bare face.
        for alias in [
            "IALA S-201 2.0.0",
            "IALA S-201 Ed 2.0.0",
            "IALA S-201:ed2.0.0",
            "IALA s-201 2.0.0",
        ] {
            let id = IalaIdentifier::parse(alias).expect(alias);
            assert_eq!(id.to_human(), "S-201 2.0.0", "{alias}");
            assert_eq!(
                id.to_urn().as_deref(),
                Some("urn:mrn:iala:pub:s-201:ed2.0.0"),
                "{alias}"
            );
        }
    }

    #[test]
    fn the_s100_annex_takes_the_sub_numbered_letters() {
        let id = IalaIdentifier::parse("S-201 ANNEX B2").expect("parse");
        assert_eq!(id.kind, IalaKind::Annex);
        assert_eq!(id.base_kind, Some(IalaKind::ProductSpecification));
        assert_eq!(id.number, "201");
        assert_eq!(id.annex_form.as_deref(), Some("ANNEX"));
        assert_eq!(id.letter.as_deref(), Some("B2"));
        assert_eq!(id.to_human(), "S-201 ANNEX B2");
        assert_eq!(
            id.to_urn().as_deref(),
            Some("urn:mrn:iala:pub:s-201:annex-b2")
        );
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:iala:annex",
                "base": { "_type": "pubid:iala:product-specification", "number": "201" },
                "annex_form": "ANNEX",
                "letter": "B2",
            })
        );

        // The annex edition face follows the base's family (bare here),
        // and the bare-letter form exists.
        let edition = IalaIdentifier::parse("S-201 Annex A 1.0.0").expect("parse");
        assert_eq!(edition.annex_form.as_deref(), Some("Annex"));
        assert_eq!(edition.to_human(), "S-201 Annex A 1.0.0");
        assert_eq!(
            edition.to_urn().as_deref(),
            Some("urn:mrn:iala:pub:s-201:annex-a:ed1.0.0")
        );
    }

    #[test]
    fn rejects_the_s100_grant_bounds() {
        // The grammar's grant (`(!"200" ("2" digit digit))`): 200 is
        // excluded ("spans from S-201"), the other hundreds digits are
        // other orgs' grants, and the shape is exactly three digits.
        for input in [
            "S-200 1.0.0",  // excluded: the range starts AT S-201
            "S-411 1.2.1",  // WMO's grant
            "S-101 1.0.0",  // IHO's S-1xx family
            "S-30 2.0",     // not the three-digit shape
            "S-2011 2.0.0", // not the three-digit shape
            "S-201 Ed",     // an edition needs a value
            "S-",           // no number
        ] {
            assert!(
                IalaIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn the_s100_routing_reclaims_the_bare_s2xx_face() {
        // The dual-perspective law (pubid#469/#763): the bare lexical
        // form names a different document per org. IALA's exact round
        // trip on the S-2xx face is what reclaims the strings the WMO
        // flavor's generic S-rule also parses (this engine routes by
        // explicit flavor, so both parse their own claim; the
        // reference's router prefers IALA's exact round trip).
        let id = IalaIdentifier::parse("S-201 2.0.0").expect("parse");
        let round = IalaIdentifier::parse(&id.to_urn().expect("urn")).expect("round");
        assert_eq!(round, id);
        assert_eq!(round.to_human(), "S-201 2.0.0");

        // WMO keeps its own grant (S-411/S-412) and IHO keeps the
        // prefixed form.
        assert!(WmoIdentifier::parse("S-411 1.2.1").is_ok());
        assert!(IhoIdentifier::parse("IHO S-201 1.0.0").is_ok());
        // And the iala flavor never claims another org's S-series.
        assert!(IalaIdentifier::parse("S-411 1.2.1").is_err());
        assert!(IalaIdentifier::parse("S-101 1.0.0").is_err());
    }

    #[test]
    fn parses_the_codeless_slug_publications() {
        for input in [
            "IALA arctic-seminar-report",
            "IALA arctic-seminar-report (E)",
            "IALA 13th-vts-symposium-sustainable-safe-navigation-august-2016-report",
            "IALA iala-world-wide-radio-navigation-plan Ed 2",
            "IALA guideline-for-preparation-of-conference-or-symposium Ed 3.0 (E)",
        ] {
            let id = IalaIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, IalaKind::Publication, "{input}");
            assert_eq!(id.to_human(), input, "{input}");
        }

        let detailed = IalaIdentifier::parse(
            "IALA guideline-for-preparation-of-conference-or-symposium Ed 3.0 (E)",
        )
        .expect("parse");
        assert_eq!(
            detailed.number,
            "guideline-for-preparation-of-conference-or-symposium"
        );
        assert_eq!(detailed.edition.as_deref(), Some("3.0"));
        assert_eq!(detailed.language.as_deref(), Some("E"));
        // The slug serializes as number.
        assert_eq!(
            detailed.to_hash(),
            json!({
                "_type": "pubid:iala:publication",
                "number": "guideline-for-preparation-of-conference-or-symposium",
                "edition": "3.0",
                "language": "E",
            })
        );
        // An integer edition stays verbatim.
        let integer = IalaIdentifier::parse("IALA iala-world-wide-radio-navigation-plan Ed 2")
            .expect("parse");
        assert_eq!(integer.edition.as_deref(), Some("2"));

        // Digit-leading slugs parse (the corpus's symposium reports).
        let digits = IalaIdentifier::parse(
            "IALA 13th-vts-symposium-sustainable-safe-navigation-august-2016-report (E)",
        )
        .expect("parse");
        assert_eq!(digits.language.as_deref(), Some("E"));
    }

    #[test]
    fn a_slug_publication_has_no_urn() {
        // IALA assigns no MRN to the codeless publications.
        let id = IalaIdentifier::parse("IALA arctic-seminar-report (E)").expect("parse");
        assert_eq!(id.to_urn(), None);
        assert_eq!(
            id.to_hash().get("number"),
            Some(&json!("arctic-seminar-report"))
        );
    }

    #[test]
    fn the_slug_forms_stay_narrow() {
        // The slug shape: two or more [a-z0-9] runs joined by single
        // hyphens, the first run not code-like, the IALA prefix
        // mandatory — a loud parse error, never a quiet Publication.
        for input in [
            "arctic-seminar-report",         // the prefix is mandatory
            "IALA Arctic-Seminar-Report",    // a slug is lower-case
            "IALA arctic seminar report",    // a slug has no space
            "IALA -arctic-seminar-report",   // starts with a letter/digit
            "IALA arctic-seminar-report Ed", // an edition needs a value
            "IALA arctic",                   // two or more runs
            "IALA arctic-",                  // single hyphens
            "IALA arctic--report",
            // A lower-case or letterless typo of a typed code is not a
            // slug — it stays the loud parse error it always was.
            "IALA s1070",
            "IALA c0103-1",
            "IALA g1164 Ed 1.1",
            "IALA ga01-13",
            "IALA 1070 Ed 2.0",
        ] {
            assert!(
                IalaIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn the_typed_codes_keep_priority_over_the_slug() {
        // slug_identifier is the grammar's LAST alternative.
        for (input, kind) in [
            ("IALA G1164 Ed 1.1", IalaKind::Guideline),
            ("IALA GA01.13", IalaKind::GeneralAssembly),
            ("IALA S1070 Ed 2.0 (F)", IalaKind::Standard),
            ("IALA G1128 ANNEX A Ed 1.6", IalaKind::Annex),
            ("IALA s-201 2.0.0", IalaKind::ProductSpecification),
        ] {
            let id = IalaIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, kind, "{input}");
            assert_ne!(id.kind, IalaKind::Publication, "{input}");
        }
    }

    #[test]
    fn renders_the_mrn_urn_forms() {
        for (input, urn) in [
            ("S1070", "urn:mrn:iala:pub:s1070"),
            ("S1070 Ed 2.0", "urn:mrn:iala:pub:s1070:ed2.0"),
            ("R1016 Ed 2.0 (F)", "urn:mrn:iala:pub:r1016:ed2.0:f"),
            ("C0103-1 Ed 3.0", "urn:mrn:iala:pub:c0103-1:ed3.0"),
            ("IALA M0001 Ed 9.0", "urn:mrn:iala:pub:m0001:ed9.0"),
            ("IALA GA01.01", "urn:mrn:iala:pub:ga01.01"),
            ("IALA L2.1.11 Ed 2", "urn:mrn:iala:pub:l2.1.11:ed2"),
            ("IALA A12-01", "urn:mrn:iala:pub:a12-01"),
            ("IALA G1045 Annex Ed 1", "urn:mrn:iala:pub:g1045:annex:ed1"),
            (
                "IALA G1128 ANNEX A Ed 1.6",
                "urn:mrn:iala:pub:g1128:annex-a:ed1.6",
            ),
        ] {
            assert_eq!(
                IalaIdentifier::parse(input).expect(input).to_urn(),
                Some(urn.to_string()),
                "{input}"
            );
        }
    }

    #[test]
    fn parses_the_mrn_urn_forms() {
        for (urn, human) in MRN_SHAPE_URN_EXAMPLES {
            let id = IalaIdentifier::parse(urn).expect(urn);
            assert_eq!(&id.to_human(), human, "{urn}");
            // Every MRN form round-trips exactly.
            assert_eq!(id.to_urn().as_deref(), Some(*urn), "{urn}");
        }
    }

    #[test]
    fn every_generated_urn_rebuilds_its_identifier() {
        // The human → MRN → re-parse → human conversion is exact over
        // every face (the reference's fixtures URN round-trip spec).
        for input in [
            "S1070 Ed 2.0",
            "R1016 Ed 2.0 (F)",
            "C0103-1 Ed 3.0",
            "IALA M0001 Ed 9.0",
            "IALA GA01.01",
            "IALA A12-01",
            "IALA L2.7.1-2 Ed 2 (E)",
            "IALA G1045 Annex Ed 1",
            "IALA G1128 ANNEX A Ed 1.6",
            "IALA G1128 ANNEX D Ed 1.6 (E)",
            "S-201 2.0.0",
            "S-230 1.0.0 (E)",
            "S-201 ANNEX B2",
            // The attested lettered face: the lettered annex URN decode
            // re-cases to the generator's ANNEX casing, so the round
            // runs on it (the "Annex A" spelling stays a human alias).
            "S-201 ANNEX A 1.0.0",
        ] {
            let id = IalaIdentifier::parse(input).expect(input);
            let urn = id.to_urn().expect(input);
            let round = IalaIdentifier::parse(&urn).expect(&urn);
            assert_eq!(round, id, "{input}");
            assert_eq!(round.to_human(), id.to_human(), "{input}");
        }
    }

    #[test]
    fn the_published_r1026_malformed_urns_normalize() {
        // Both variants printed in the published R1026 PDF.
        for (urn, canonical) in [
            (
                "urn:mrn:iala:pub:r1026:Ed1.0",
                "urn:mrn:iala:pub:r1026:ed1.0",
            ),
            (
                "urn:mrn:iala:pub:r1026:ed.1.0",
                "urn:mrn:iala:pub:r1026:ed1.0",
            ),
        ] {
            let id = IalaIdentifier::parse(urn).expect(urn);
            assert_eq!(id.to_human(), "IALA R1026 Ed 1.0", "{urn}");
            assert_eq!(id.to_urn().as_deref(), Some(canonical), "{urn}");
        }
    }

    #[test]
    fn urn_segments_are_caseless() {
        let id = IalaIdentifier::parse("URN:MRN:IALA:PUB:S-201:ANNEX-B2").expect("parse");
        assert_eq!(id.to_human(), "S-201 ANNEX B2");
        let edition = IalaIdentifier::parse("URN:MRN:IALA:PUB:S-230:ED1.0.0:E").expect("parse");
        assert_eq!(edition.to_human(), "S-230 1.0.0 (E)");
    }

    #[test]
    fn rejects_the_bad_urns() {
        for urn in [
            "urn:mrn:wmo:pub:s-411:ed1.2.1", // not the iala namespace
            "urn:iala:pub:s1070",            // not the mrn namespace
            "urn:mrn:iala:lib:s1070",        // not the pub sub-namespace
            "urn:mrn:iala:pub:",             // no code segment
            // A slug-shaped code is no MRN: the rebuild fails in the
            // grammar (IALA assigns no MRN to the codeless forms).
            "urn:mrn:iala:pub:arctic-seminar-report",
        ] {
            assert!(
                IalaIdentifier::parse(urn).is_err(),
                "{urn} must be rejected"
            );
        }
    }

    #[test]
    fn round_trips_through_the_hash() {
        for (input, canonical) in [
            ("S1070", "IALA S1070"),
            ("S1070 Ed 2.0", "IALA S1070 Ed 2.0"),
            ("R0126 Ed 2.0 (F)", "IALA R0126 Ed 2.0 (F)"),
            ("C0103-1 Ed 3.0", "IALA C0103-1 Ed 3.0"),
            ("IALA M0001 Ed 9.0", "IALA M0001 Ed 9.0"),
            ("IALA A12-01", "IALA A12-01"),
            ("IALA GA01.01", "IALA GA01.01"),
            ("IALA L2.7.1-2 Ed 2 (E)", "IALA L2.7.1-2 Ed 2 (E)"),
            ("IALA X0123", "IALA X0123"),
            ("IALA G1045 Annex Ed 1", "IALA G1045 Annex Ed 1"),
            ("IALA G1128 ANNEX A Ed 1.6", "IALA G1128 ANNEX A Ed 1.6"),
            ("S-201 2.0.0", "S-201 2.0.0"),
            ("S-230 1.0.0 (E)", "S-230 1.0.0 (E)"),
            ("S-201 ANNEX B2", "S-201 ANNEX B2"),
            (
                "IALA arctic-seminar-report (E)",
                "IALA arctic-seminar-report (E)",
            ),
        ] {
            let id = IalaIdentifier::parse(input).expect("parse");
            let rebuilt = IalaIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id, "{input}");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{input}");
            assert_eq!(rebuilt.to_human(), canonical, "{input}");
            assert_eq!(rebuilt.to_urn(), id.to_urn(), "{input}");
        }
    }

    #[test]
    fn rejects_the_unattested_shapes() {
        for input in [
            "nonsense",
            "XYZ123-BAD",
            "IALA",
            "IALA G",
            "IALA Annex Ed 1",
        ] {
            assert!(
                IalaIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn the_from_hash_annex_requires_a_base() {
        let err = IalaIdentifier::from_hash(&json!({
            "_type": "pubid:iala:annex",
            "annex_form": "Annex",
        }))
        .expect_err("no base must raise");
        assert!(err.to_string().contains("no base"));
        assert!(IalaIdentifier::from_hash(&json!({ "_type": "pubid:iala:other" })).is_err());
        assert!(crate::bound::check_type(
            &json!({"_type": "pubid:iala:annex"}),
            "pubid:iala:annex"
        )
        .is_ok());
        // The letterless hash round-trip of the bare annex.
        let id = IalaIdentifier::from_hash(&json!({
            "_type": "pubid:iala:annex",
            "base": { "_type": "pubid:iala:guideline", "number": "1045" },
            "annex_form": "Annex",
        }))
        .expect("parse");
        assert_eq!(id.to_human(), "IALA G1045 Annex");
        assert_eq!(
            id.to_urn(),
            Some("urn:mrn:iala:pub:g1045:annex".to_string())
        );
    }
}
