//! The cen_cenelec flavor: the CEN/CENELEC family — European norms
//! (with the prEN/FprEN draft stages), technical specifications and
//! reports, guides, workshop agreements, harmonization documents,
//! European specifications/prestandards, CEN reports, the amendment /
//! corrigendum / fragment supplements, consolidated editions, and the
//! adopted (EN ISO / EN IEC) documents.
//!
//! The artifact binds the flat document captures (`publisher`, `type`,
//! `type_with_stage`, `number`, `parts[]`, `version`, `year`, `month`,
//! `copublishers[]`) plus the supplement captures (`supplements[]` with
//! the `amd_sep_plus`/`amd_sep_slash` separator, `amd_number`,
//! `cor_number`, year/month), the `adopted_string` face, and the
//! fragment form (`amendment_number` + `fragment_number`). The flavor
//! layer does the reference's builder cast over them:
//!
//! - the publisher selects the class (EN → european-norm, CR →
//!   cen-report, CWA/HD/ES/ENV their own classes), the type token
//!   (`Guide`/`TR`/`TS`) or the draft stage (`prEN`/`FprEN`) refines it;
//! - slash supplements render standalone (`/A1`, `/AC:2003`), plus
//!   supplements consolidate (`+A1:2015`);
//! - the adopted string parses through the bounded ISO/IEC cast (ISO,
//!   ISO/TS, ISO/TR, ISO/IEC, IEC; the `Amd n:y` ISO amendment nests
//!   under its base; everything else stays a foreign reference);
//! - the `#` comment tail and the `(corrigendum)` parenthetical, the
//!   non-breaking/soft hyphens, and the `CEN-CLC`/`GUIDE` spellings
//!   normalize before the parse (the reference's parser normalization);
//! - an EN-published bare 60000–79999 number is an implicit IEC
//!   adoption (`EN 60335` → `EN IEC 60335`).
//!
//! The URN keeps the reference's faces: `urn:cen:<publisher>[:copubs]`
//! for documents, the adopted MR string (`iso.ts.21003-7.2008` with
//! dots/underscores mapped to colons) under the adopted publisher, and
//! the `plus`/`amd`/`cor`/`frag` supplement chains.

use serde_json::Value;

use crate::artifact::cen_cenelec_artifact;
use crate::bound::{capture_in, optional_at, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete cen_cenelec class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenKind {
    EuropeanNorm,
    TechnicalSpecification,
    TechnicalReport,
    Guide,
    CenWorkshopAgreement,
    HarmonizationDocument,
    EuropeanSpecification,
    CenReport,
    EuropeanPrestandard,
    Amendment,
    Corrigendum,
    Fragment,
    ConsolidatedIdentifier,
    AdoptedEuropeanNorm,
}

use CenKind::*;

impl CenKind {
    /// The shared `_type` prefix every cen_cenelec tag carries (the
    /// enum's dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:cencenelec:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            EuropeanNorm => "pubid:cencenelec:european-norm",
            TechnicalSpecification => "pubid:cencenelec:technical-specification",
            TechnicalReport => "pubid:cencenelec:technical-report",
            Guide => "pubid:cencenelec:guide",
            CenWorkshopAgreement => "pubid:cencenelec:cen-workshop-agreement",
            HarmonizationDocument => "pubid:cencenelec:harmonization-document",
            EuropeanSpecification => "pubid:cencenelec:european-specification",
            CenReport => "pubid:cencenelec:cen-report",
            EuropeanPrestandard => "pubid:cencenelec:european-prestandard",
            Amendment => "pubid:cencenelec:amendment",
            Corrigendum => "pubid:cencenelec:corrigendum",
            Fragment => "pubid:cencenelec:fragment",
            ConsolidatedIdentifier => "pubid:cencenelec:consolidated-identifier",
            AdoptedEuropeanNorm => "pubid:cencenelec:adopted-european-norm",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [
            EuropeanNorm,
            TechnicalSpecification,
            TechnicalReport,
            Guide,
            CenWorkshopAgreement,
            HarmonizationDocument,
            EuropeanSpecification,
            CenReport,
            EuropeanPrestandard,
            Amendment,
            Corrigendum,
            Fragment,
            ConsolidatedIdentifier,
            AdoptedEuropeanNorm,
        ]
        .into_iter()
        .find(|kind| kind.type_tag() == tag)
    }

    /// The publisher the class defaults to (only a different captured
    /// publisher lands on the wire).
    fn default_publisher(self) -> Option<&'static str> {
        match self {
            CenWorkshopAgreement => Some("CWA"),
            HarmonizationDocument => Some("HD"),
            EuropeanSpecification => Some("ES"),
            CenReport => Some("CR"),
            EuropeanPrestandard => Some("ENV"),
            _ => Some("EN"),
        }
    }

    /// The short word the class renders when no type token overrides it.
    fn class_short(self) -> &'static str {
        match self {
            EuropeanNorm => "EN",
            TechnicalSpecification => "TS",
            TechnicalReport => "TR",
            Guide => "Guide",
            CenWorkshopAgreement => "CWA",
            HarmonizationDocument => "HD",
            EuropeanSpecification => "ES",
            CenReport => "CR",
            EuropeanPrestandard => "ENV",
            _ => "EN",
        }
    }
}

/// The typed draft stages (`prEN` → proposal, `FprEN` → final
/// proposal); the published stages ride the class defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenStage {
    /// The wire `stage` code (`pren`, `fpren`).
    pub code: &'static str,
    /// The printed abbreviation (`prEN`, `FprEN`).
    pub abbr: &'static str,
    /// The URN stage segment (`stage.proposal`).
    pub stage_code: &'static str,
    /// The document type the stage belongs to (`en`, `ts`, ...).
    pub type_code: &'static str,
}

/// The stage table (the reference's TYPED_STAGES).
pub const TYPED_STAGES: &[CenStage] = &[
    CenStage {
        code: "pwien",
        abbr: "pWI EN",
        stage_code: "preliminary",
        type_code: "en",
    },
    CenStage {
        code: "pren",
        abbr: "prEN",
        stage_code: "proposal",
        type_code: "en",
    },
    CenStage {
        code: "fpren",
        abbr: "FprEN",
        stage_code: "final_proposal",
        type_code: "en",
    },
    CenStage {
        code: "fven",
        abbr: "FV prEN",
        stage_code: "formal_vote",
        type_code: "en",
    },
    CenStage {
        code: "ven",
        abbr: "vEN",
        stage_code: "vote",
        type_code: "en",
    },
    CenStage {
        code: "puben",
        abbr: "EN",
        stage_code: "published",
        type_code: "en",
    },
    CenStage {
        code: "rven",
        abbr: "rvEN",
        stage_code: "review",
        type_code: "en",
    },
    CenStage {
        code: "racen",
        abbr: "racEN",
        stage_code: "reactivation",
        type_code: "en",
    },
    CenStage {
        code: "wden",
        abbr: "wdEN",
        stage_code: "withdrawn",
        type_code: "en",
    },
    CenStage {
        code: "pubts",
        abbr: "TS",
        stage_code: "published",
        type_code: "ts",
    },
    CenStage {
        code: "prts",
        abbr: "prTS",
        stage_code: "proposal",
        type_code: "ts",
    },
    CenStage {
        code: "pubtr",
        abbr: "TR",
        stage_code: "published",
        type_code: "tr",
    },
    CenStage {
        code: "pubcwa",
        abbr: "CWA",
        stage_code: "published",
        type_code: "cwa",
    },
    CenStage {
        code: "pubguide",
        abbr: "Guide",
        stage_code: "published",
        type_code: "guide",
    },
    CenStage {
        code: "pubhd",
        abbr: "HD",
        stage_code: "published",
        type_code: "hd",
    },
    CenStage {
        code: "pubes",
        abbr: "ES",
        stage_code: "published",
        type_code: "es",
    },
    CenStage {
        code: "pubcr",
        abbr: "CR",
        stage_code: "published",
        type_code: "cr",
    },
    CenStage {
        code: "pubenv",
        abbr: "ENV",
        stage_code: "published",
        type_code: "env",
    },
];

/// Locate a stage by its printed abbreviation (case-insensitive).
fn locate_stage(abbr: &str) -> Option<&'static CenStage> {
    TYPED_STAGES
        .iter()
        .find(|stage| stage.abbr.eq_ignore_ascii_case(abbr))
}

/// Locate a stage by its wire code.
fn locate_stage_by_code(code: &str) -> Option<&'static CenStage> {
    TYPED_STAGES.iter().find(|stage| stage.code == code)
}

/// The class for a stage's type code.
fn class_for_type_code(type_code: &str) -> Option<CenKind> {
    match type_code {
        "en" => Some(EuropeanNorm),
        "ts" => Some(TechnicalSpecification),
        "tr" => Some(TechnicalReport),
        "guide" => Some(Guide),
        "cwa" => Some(CenWorkshopAgreement),
        "hd" => Some(HarmonizationDocument),
        "es" => Some(EuropeanSpecification),
        "cr" => Some(CenReport),
        "env" => Some(EuropeanPrestandard),
        _ => None,
    }
}

/// The publisher word classes (CWA/HD/ES/CR/ENV carry their own word).
fn class_for_publisher(publisher: &str) -> Option<CenKind> {
    match publisher {
        "CWA" => Some(CenWorkshopAgreement),
        "HD" => Some(HarmonizationDocument),
        "ES" => Some(EuropeanSpecification),
        "CR" => Some(CenReport),
        "ENV" => Some(EuropeanPrestandard),
        _ => None,
    }
}

/// An adopted reference: a parsed ISO/IEC wire hash, or the captured
/// text of a foreign reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CenAdopted {
    /// A foreign reference kept by its captured text.
    Foreign(String),
    /// A parsed adopted identifier (ISO/IEC faces).
    Hash(Value),
}

impl CenAdopted {
    fn to_hash(&self) -> Value {
        match self {
            CenAdopted::Foreign(_) => serde_json::json!({"_type": FOREIGN_REFERENCE}),
            CenAdopted::Hash(hash) => hash.clone(),
        }
    }

    fn from_hash(hash: &Value) -> Self {
        if hash.get("_type").and_then(Value::as_str) == Some(FOREIGN_REFERENCE) {
            CenAdopted::Foreign(String::new())
        } else {
            CenAdopted::Hash(hash.clone())
        }
    }

    fn to_human(&self) -> String {
        match self {
            CenAdopted::Foreign(raw) => raw.clone(),
            CenAdopted::Hash(hash) => render_iso_iec_hash(hash),
        }
    }
}

/// The foreign-reference tag (an unparseable adopted origin).
pub const FOREIGN_REFERENCE: &str = "pubid:cencenelec:foreign-reference";

/// The cen_cenelec identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenIdentifier {
    /// The concrete class (drives `_type`, the render shape, and the
    /// publisher default).
    pub kind: CenKind,
    /// The document number (a supplement's ordinal for supplements).
    pub number: Option<String>,
    /// The dash-suffix part (`1` in `EN 1325-1`; may be `60-15`).
    pub part: Option<String>,
    /// The year (a supplement's own year when on a supplement).
    pub year: Option<String>,
    /// The corrigendum month (`11` in `AC:2016-11`).
    pub month: Option<String>,
    /// The ETSI-style version face (` V2.2.1`, leading space kept).
    pub version: Option<String>,
    /// The draft stage (prEN/FprEN forms).
    pub stage: Option<&'static CenStage>,
    /// The captured publisher (absent means the class default).
    pub publisher: Option<String>,
    /// The copublishers (`CLC`, `ETSI`).
    pub copublishers: Vec<String>,
    /// A supplement's / a fragment's underlying document.
    pub base: Option<Box<CenIdentifier>>,
    /// The adopted ISO/IEC reference (adopted-european-norm, ENV
    /// adoption).
    pub adopted: Option<CenAdopted>,
    /// The consolidated edition's members (base first).
    pub identifiers: Vec<CenIdentifier>,
}

impl CenIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// reference's builder cast over the bound captures.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = cen_cenelec_artifact()?;
        let normalized = preprocess(input);
        let bound = artifact.parse_and_bind(Self::ENTRY, &normalized)?;
        Self::build(&bound)
    }

    /// The builder cast over a bound hash (shared with the evs flavor,
    /// which embeds the cen grammar under its `adopted` subtree).
    pub fn build(bound: &Value) -> Result<Self> {
        if optional_at(bound, "fragment_number").is_some() {
            return build_fragment(bound);
        }
        if optional_at(bound, "type") == Some("TR") && optional_at(bound, "publisher") == Some("EN")
        {
            return Err(Error::InvalidHash {
                detail: "\"EN\" cannot be \"TR\": a technical report is \"CEN/TR\" or \"CLC/TR\""
                    .to_string(),
            });
        }
        let base = build_base_document(bound)?;
        apply_supplements(base, bound)
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(CenKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown cen_cenelec _type {:?}", hash.get("_type")),
            })?;
        let read = |field: &'static str| optional_hash_str(hash, field).map(str::to_string);
        let base = match hash.get("base").cloned().filter(|base| !base.is_null()) {
            Some(base) => Some(Box::new(CenIdentifier::from_hash(&base)?)),
            None => None,
        };
        let adopted = hash
            .get("adopted")
            .cloned()
            .filter(|a| !a.is_null())
            .map(|adopted| CenAdopted::from_hash(&adopted));
        let mut identifiers = Vec::new();
        if let Some(items) = hash.get("identifiers").and_then(Value::as_array) {
            for item in items {
                identifiers.push(CenIdentifier::from_hash(item)?);
            }
        }
        Ok(Self {
            kind,
            number: read("number"),
            part: read("part"),
            year: read("year"),
            month: read("month"),
            version: read("version"),
            stage: read("stage").and_then(|code| locate_stage_by_code(&code)),
            publisher: read("publisher"),
            copublishers: hash
                .get("copublishers")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            base,
            adopted,
            identifiers,
        })
    }

    /// The canonical hash: `_type` first, then the present attributes.
    /// A publisher equal to the class default stays off the wire.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        if let Some(number) = &self.number {
            object.insert("number".into(), number.clone().into());
        }
        if let Some(part) = &self.part {
            object.insert("part".into(), part.clone().into());
        }
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        if let Some(month) = &self.month {
            object.insert("month".into(), month.clone().into());
        }
        if let Some(version) = &self.version {
            object.insert("version".into(), version.clone().into());
        }
        if let Some(stage) = &self.stage {
            object.insert("stage".into(), stage.code.into());
        }
        let default_publisher = self.kind.default_publisher();
        if let Some(publisher) = &self.publisher {
            if Some(publisher.as_str()) != default_publisher {
                object.insert("publisher".into(), publisher.clone().into());
            }
        }
        if !self.copublishers.is_empty() {
            object.insert(
                "copublishers".into(),
                Value::Array(
                    self.copublishers
                        .iter()
                        .map(|copublisher| copublisher.clone().into())
                        .collect(),
                ),
            );
        }
        if let Some(base) = &self.base {
            object.insert("base".into(), base.to_hash());
        }
        if let Some(adopted) = &self.adopted {
            object.insert("adopted".into(), adopted.to_hash());
        }
        if !self.identifiers.is_empty() {
            object.insert(
                "identifiers".into(),
                Value::Array(
                    self.identifiers
                        .iter()
                        .map(CenIdentifier::to_hash)
                        .collect(),
                ),
            );
        }
        Value::Object(object)
    }

    /// The canonical human.
    pub fn to_human(&self) -> String {
        match self.kind {
            Fragment => {
                let base = self
                    .base
                    .as_deref()
                    .map(CenIdentifier::to_human)
                    .unwrap_or_default();
                format!("{base} FRAG{}", optional_str(&self.number))
            }
            Amendment | Corrigendum => {
                let base = self
                    .base
                    .as_deref()
                    .map(CenIdentifier::to_human)
                    .unwrap_or_default();
                format!("{base}/{}", supplement_token(self))
            }
            ConsolidatedIdentifier => {
                let mut iter = self.identifiers.iter();
                let mut result = iter.next().map(CenIdentifier::to_human).unwrap_or_default();
                for supplement in iter {
                    result.push('+');
                    result.push_str(&supplement_token(supplement));
                }
                result
            }
            AdoptedEuropeanNorm => render_adopted(self),
            EuropeanPrestandard if self.adopted.is_some() => {
                let adopted = self
                    .adopted
                    .as_ref()
                    .map(CenAdopted::to_human)
                    .unwrap_or_default();
                format!("ENV {adopted}")
            }
            _ => render_single(self),
        }
    }

    /// The URN.
    pub fn to_urn(&self) -> String {
        urn_for(self)
    }
}

fn optional_str(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("")
}

// ---------------------------------------------------------------------------
// Builder cast
// ---------------------------------------------------------------------------

/// The parser-side normalization (the reference's `Parser.parse`):
/// non-breaking/soft hyphens to dashes, the `#` comment tail and the
/// `(corrigendum)` parenthetical off, the `CEN-CLC` and `GUIDE`
/// spellings to their canonical faces.
fn preprocess(input: &str) -> String {
    let normalized = input
        .replace(['\u{2011}', '\u{00AD}'], "-")
        .replace("CEN-CLC", "CEN/CLC")
        .replace("CLC-CEN", "CLC/CEN")
        .replace("GUIDE", "Guide");
    let stripped = match normalized.find('#') {
        Some(index) => &normalized[..index],
        None => normalized.as_str(),
    }
    .trim();
    drop_parenthetical(stripped).to_string()
}

/// Strip a trailing `(...)` only when it mentions "corrigendum".
fn drop_parenthetical(text: &str) -> &str {
    let mut result = text;
    while let Some(open) = result.rfind('(') {
        let Some(close) = result[open..].find(')') else {
            break;
        };
        let inner = result[open + 1..open + close].to_lowercase();
        if inner.contains("corrigendum") {
            result = result[..open].trim_end();
        } else {
            break;
        }
    }
    result
}

/// The adopted-document fields: only the publisher row and the stage
/// ride along (the reference builder copies no document faces — the
/// year the grammar swallowed beside the adopted string never reaches
/// the wire).
fn adopted_fields(bound: &Value) -> Result<CenIdentifier> {
    let kind = if optional_at(bound, "publisher") == Some("ENV") {
        EuropeanPrestandard
    } else {
        AdoptedEuropeanNorm
    };
    let mut fields = plain_fields(bound, kind)?;
    fields.kind = kind;
    fields.number = None;
    fields.part = None;
    fields.year = None;
    fields.month = None;
    fields.version = None;
    Ok(fields)
}

/// The plain-document cast: kind from publisher / stage / type, then
/// the shared fields.
fn build_base_document(bound: &Value) -> Result<CenIdentifier> {
    if optional_at(bound, "adopted_string").is_some() {
        let mut identifier = adopted_fields(bound)?;
        identifier.adopted = Some(parse_adopted_string(
            optional_at(bound, "adopted_string").expect("checked above"),
        ));
        return Ok(identifier);
    }

    // The implicit IEC adoption: an EN-published bare 60000–79999
    // number with no parts and no supplements.
    let publisher = optional_at(bound, "publisher");
    let has_parts = bound
        .get("parts")
        .and_then(Value::as_array)
        .is_some_and(|parts| !parts.is_empty());
    let has_supplements = bound
        .get("supplements")
        .and_then(Value::as_array)
        .is_some_and(|supplements| !supplements.is_empty());
    if publisher == Some("EN") && !has_parts && !has_supplements {
        if let Some(number) = optional_at(bound, "number") {
            if let Ok(value) = number.parse::<u32>() {
                if (60_000..=79_999).contains(&value) {
                    let adopted = parse_adopted_string(&format!("IEC {number}"));
                    return Ok(CenIdentifier {
                        adopted: Some(adopted),
                        ..adopted_fields(bound)?
                    });
                }
            }
        }
    }

    let kind = document_kind(bound);
    let mut identifier = plain_fields(bound, kind)?;
    identifier.kind = kind;
    Ok(identifier)
}

/// The document kind from the publisher / stage / type captures.
fn document_kind(bound: &Value) -> CenKind {
    if let Some(publisher) = optional_at(bound, "publisher") {
        if let Some(kind) = class_for_publisher(publisher) {
            return kind;
        }
    }
    let type_or_stage = optional_at(bound, "type_with_stage")
        .or_else(|| optional_at(bound, "type"))
        .unwrap_or("EN");
    locate_stage(type_or_stage)
        .and_then(|stage| class_for_type_code(stage.type_code))
        .unwrap_or(EuropeanNorm)
}

/// The shared document fields off the bound hash.
fn plain_fields(bound: &Value, kind: CenKind) -> Result<CenIdentifier> {
    let read = |key: &'static str| optional_at(bound, key).map(str::to_string);
    let stage = optional_at(bound, "type_with_stage").and_then(locate_stage);
    let copublishers = bound
        .get("copublishers")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| capture_in(item, "copublisher").map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let publisher = read("publisher").or_else(|| kind.default_publisher().map(str::to_string));
    Ok(CenIdentifier {
        kind,
        number: read("number"),
        part: bound
            .get("parts")
            .and_then(Value::as_array)
            .and_then(|parts| parts.first())
            .and_then(|part| capture_in(part, "part").map(str::to_string)),
        year: read("year"),
        month: read("month"),
        version: read("version"),
        stage,
        publisher,
        copublishers,
        base: None,
        adopted: None,
        identifiers: Vec::new(),
    })
}

/// One supplement's cast data off its bound entry.
#[derive(Debug, Clone)]
struct SupplementData {
    amendment: bool,
    number: Option<String>,
    year: Option<String>,
    month: Option<String>,
}

fn supplement_data(entry: &Value) -> SupplementData {
    let inner = entry.get("supplement").unwrap_or(entry);
    let read = |key: &'static str| optional_at(inner, key).map(str::to_string);
    let amendment = read("amd_number");
    SupplementData {
        amendment: amendment.is_some(),
        number: amendment.or_else(|| read("cor_number")),
        year: read("amd_year").or_else(|| read("year")),
        month: read("month"),
    }
}

/// Wrap the base document with its supplements: slash separators make
/// the standalone supplement (amendment `/A1`, corrigendum `/AC`),
/// plus separators consolidate (`+A1:2015`).
fn apply_supplements(base: CenIdentifier, bound: &Value) -> Result<CenIdentifier> {
    let entries = bound
        .get("supplements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return Ok(base);
    }
    let data: Vec<SupplementData> = entries.iter().map(supplement_data).collect();
    if let Some(first) = data.first() {
        if entries
            .first()
            .and_then(|entry| {
                entry
                    .get("supplement")
                    .unwrap_or(entry)
                    .get("amd_sep_slash")
            })
            .is_some()
        {
            let kind = if first.amendment {
                Amendment
            } else {
                Corrigendum
            };
            return Ok(CenIdentifier {
                kind,
                number: first.number.clone(),
                year: first.year.clone(),
                month: first.month.clone(),
                base: Some(Box::new(base)),
                ..empty_identifier(kind)
            });
        }
    }
    let mut identifiers = vec![base];
    for supplement in data {
        let kind = if supplement.amendment {
            Amendment
        } else {
            Corrigendum
        };
        identifiers.push(CenIdentifier {
            kind,
            number: supplement.number,
            year: supplement.year,
            month: supplement.month,
            ..empty_identifier(kind)
        });
    }
    Ok(CenIdentifier {
        kind: ConsolidatedIdentifier,
        identifiers,
        ..empty_identifier(ConsolidatedIdentifier)
    })
}

/// The fragment cast: the plain base document (the reference builder
/// routes fragments through the plain path only — no adoption), its
/// `/A1` (or `AMD1`) amendment, then the fragment ordinal.
fn build_fragment(bound: &Value) -> Result<CenIdentifier> {
    let mut base_data = bound.clone();
    if let Some(object) = base_data.as_object_mut() {
        for key in ["amendment_number", "fragment_number", "amendment_year"] {
            object.remove(key);
        }
    }
    let kind = document_kind(&base_data);
    let base = plain_fields(&base_data, kind)?;
    let amendment = CenIdentifier {
        kind: Amendment,
        number: optional_at(bound, "amendment_number").map(str::to_string),
        year: optional_at(bound, "amendment_year").map(str::to_string),
        base: Some(Box::new(base)),
        ..empty_identifier(Amendment)
    };
    Ok(CenIdentifier {
        kind: Fragment,
        number: optional_at(bound, "fragment_number").map(str::to_string),
        base: Some(Box::new(amendment)),
        ..empty_identifier(Fragment)
    })
}

fn empty_identifier(kind: CenKind) -> CenIdentifier {
    CenIdentifier {
        kind,
        number: None,
        part: None,
        year: None,
        month: None,
        version: None,
        stage: None,
        publisher: None,
        copublishers: Vec::new(),
        base: None,
        adopted: None,
        identifiers: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// The adopted-string cast (bounded ISO/IEC)
// ---------------------------------------------------------------------------

/// The adopted-string cast: origin token → the bounded ISO/IEC faces.
/// Unknown origins stay foreign references (rendered from their
/// captured text).
fn parse_adopted_string(raw: &str) -> CenAdopted {
    let raw = raw.trim();
    let (iso_like, copublishers, rest) = if let Some(rest) = raw.strip_prefix("ISO/TS") {
        (IsoLike::TechnicalSpecification, Vec::new(), rest)
    } else if let Some(rest) = raw.strip_prefix("ISO/TR") {
        (IsoLike::TechnicalReport, Vec::new(), rest)
    } else if let Some(rest) = raw.strip_prefix("ISO/IEC") {
        (IsoLike::International, vec!["IEC".to_string()], rest)
    } else if let Some(rest) = raw.strip_prefix("ISO") {
        (IsoLike::International, Vec::new(), rest)
    } else if let Some(rest) = raw.strip_prefix("IEC") {
        (IsoLike::Iec, Vec::new(), rest)
    } else {
        return CenAdopted::Foreign(raw.to_string());
    };
    let rest = rest.trim_start();
    // The ISO amendment tail (`/Amd 1:2016`) nests under its base.
    if iso_like == IsoLike::International && copublishers.is_empty() {
        if let Some((base, amendment)) = rest.split_once("/Amd ") {
            if let Some(mut base_hash) = number_part_year(base.trim()) {
                let (number, year) = amendment.split_once(':').map_or_else(
                    || (amendment.trim(), None),
                    |(number, year)| (number.trim(), Some(year.trim())),
                );
                base_hash.insert("_type".into(), IsoLike::International.type_tag().into());
                let mut hash = serde_json::Map::new();
                hash.insert("_type".into(), "pubid:iso:amendment".into());
                hash.insert("number".into(), number.into());
                if let Some(year) = year {
                    hash.insert("year".into(), year.into());
                }
                hash.insert("base".into(), Value::Object(base_hash));
                return CenAdopted::Hash(Value::Object(hash));
            }
        }
    }
    match number_part_year(rest) {
        Some(mut hash) => {
            hash.insert("_type".into(), iso_like.type_tag().into());
            if !copublishers.is_empty() {
                hash.insert(
                    "copublishers".into(),
                    Value::Array(copublishers.iter().map(|c| c.clone().into()).collect()),
                );
            }
            CenAdopted::Hash(Value::Object(hash))
        }
        None => CenAdopted::Foreign(raw.to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IsoLike {
    International,
    TechnicalSpecification,
    TechnicalReport,
    Iec,
}

impl IsoLike {
    fn type_tag(self) -> &'static str {
        match self {
            IsoLike::International => "pubid:iso:international-standard",
            IsoLike::TechnicalSpecification => "pubid:iso:technical-specification",
            IsoLike::TechnicalReport => "pubid:iso:technical-report",
            IsoLike::Iec => "pubid:iec:international-standard",
        }
    }
}

/// Split `<number>[-part[-subpart]][:year]` into the hash fields.
fn number_part_year(rest: &str) -> Option<serde_json::Map<String, Value>> {
    let (base, year) = match rest.split_once(':') {
        Some((base, year)) => (base, Some(year)),
        None => (rest, None),
    };
    let mut segments = base.split('-');
    let number = segments.next()?;
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut hash = serde_json::Map::new();
    hash.insert("number".into(), number.into());
    if let Some(part) = segments.next() {
        hash.insert("part".into(), part.into());
    }
    if let Some(subpart) = segments.next() {
        hash.insert("subpart".into(), subpart.into());
    }
    if let Some(year) = year {
        hash.insert("year".into(), year.into());
    }
    Some(hash)
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// The supplement token: `A`/`AC` + number + `:year[-month]`.
fn supplement_token(supplement: &CenIdentifier) -> String {
    let prefix = match supplement.kind {
        Corrigendum => "AC",
        _ => "A",
    };
    let mut result = format!("{prefix}{}", optional_str(&supplement.number));
    if let Some(year) = &supplement.year {
        result.push(':');
        result.push_str(year);
        if let Some(month) = &supplement.month {
            result.push('-');
            result.push_str(month);
        }
    }
    result
}

/// The draft stage word (`prEN`/`FprEN`) when the stage renders as a
/// draft.
fn draft_abbr(identifier: &CenIdentifier) -> Option<&'static str> {
    match identifier.stage.map(|stage| stage.abbr) {
        Some("prEN") | Some("FprEN") => identifier.stage.map(|stage| stage.abbr),
        _ => None,
    }
}

/// Whether the type word is one of the publisher words (CWA/HD/ES/CR/
/// ENV render as their own word).
fn is_publisher_type(short: &str) -> bool {
    matches!(short, "CWA" | "HD" | "ES" | "CR" | "ENV")
}

/// The single-document render (`EN 1325-1`, `CEN/TR 12101-5:2005`,
/// `FprEN 16114:2011`, `EN 302 326-2 V2.2.1:2026`).
fn render_single(identifier: &CenIdentifier) -> String {
    let mut parts: Vec<String> = Vec::new();
    let is_draft = draft_abbr(identifier).is_some();
    let type_short = if is_draft {
        identifier
            .stage
            .map(|stage| stage.type_code.to_ascii_uppercase())
            .unwrap_or_else(|| identifier.kind.class_short().to_string())
    } else {
        identifier.kind.class_short().to_string()
    };

    let mut use_slash_before_type = false;
    if is_publisher_type(&type_short) {
        let word = match identifier.stage {
            Some(stage) if stage.abbr != type_short => stage.abbr.to_string(),
            _ => type_short.clone(),
        };
        parts.push(word);
    } else if let Some(draft) = draft_abbr(identifier) {
        parts.push(draft.to_string());
    } else {
        parts.push(render_publisher(identifier));
        use_slash_before_type = true;
    }

    if !identifier.copublishers.is_empty() {
        let copub = identifier.copublishers.join("/");
        if let Some(last) = parts.last_mut() {
            last.push('/');
            last.push_str(&copub);
        } else {
            parts.push(copub);
        }
    }

    if type_short != "EN" && type_short != "Guide" && !is_publisher_type(&type_short) {
        if use_slash_before_type && !parts.is_empty() {
            if let Some(last) = parts.last_mut() {
                last.push('/');
                last.push_str(&type_short);
            }
        } else {
            parts.push(type_short.clone());
        }
    } else if type_short == "Guide" {
        parts.push("Guide".to_string());
    }

    if let Some(number) = &identifier.number {
        let mut face = match &identifier.part {
            Some(part) => format!("{number}-{part}"),
            None => number.clone(),
        };
        if let Some(version) = &identifier.version {
            face.push_str(version);
        }
        parts.push(face);
    }

    let mut result = String::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 && !part.starts_with('/') {
            result.push(' ');
        }
        result.push_str(part);
    }

    if let Some(year) = &identifier.year {
        result.push(':');
        result.push_str(year);
        if let Some(month) = &identifier.month {
            result.push('-');
            result.push_str(month);
        }
    }
    result
}

/// The publisher word a single-document render leads with (the captured
/// publisher, else the class default — a `from_hash` rebuild carries no
/// default publisher on the wire).
fn render_publisher(identifier: &CenIdentifier) -> String {
    identifier
        .publisher
        .clone()
        .unwrap_or_else(|| identifier.kind.class_short().to_string())
}

/// The adopted-document render (`EN ISO 13485:2016`, `CEN ISO/TS
/// 21003-7`, `HD IEC 60364-8-81`): the publisher row, then the
/// adopted's human face.
fn render_adopted(identifier: &CenIdentifier) -> String {
    let mut publishers: Vec<String> = Vec::new();
    if let Some(publisher) = &identifier.publisher {
        publishers.push(publisher.clone());
    } else {
        publishers.push("EN".to_string());
    }
    publishers.extend(identifier.copublishers.iter().cloned());
    if let Some(draft) = draft_abbr(identifier) {
        if let Some(first) = publishers.first_mut() {
            *first = draft.to_string();
        }
    }
    let mut result = publishers.join("/");
    if let Some(adopted) = &identifier.adopted {
        result.push(' ');
        result.push_str(&adopted.to_human());
    }
    result
}

/// The ISO/IEC adopted hash's human face (`ISO 13485:2016`,
/// `ISO/TS 21003-7`, `IEC 61000-4-30`, the `Amd` amendment nesting).
fn render_iso_iec_hash(hash: &Value) -> String {
    let type_tag = hash.get("_type").and_then(Value::as_str).unwrap_or("");
    if type_tag == "pubid:iso:amendment" {
        let base = hash
            .get("base")
            .map(render_iso_iec_hash)
            .unwrap_or_default();
        return format!(
            "{base}/Amd {}:{}",
            optional_hash_str(hash, "number").unwrap_or(""),
            optional_hash_str(hash, "year").unwrap_or(""),
        );
    }
    let mut head = match type_tag {
        "pubid:iso:technical-specification" => "ISO/TS".to_string(),
        "pubid:iso:technical-report" => "ISO/TR".to_string(),
        "pubid:iec:international-standard" => "IEC".to_string(),
        _ => {
            let mut head = "ISO".to_string();
            if let Some(copublishers) = hash.get("copublishers").and_then(Value::as_array) {
                for copublisher in copublishers {
                    if let Some(body) = copublisher.as_str() {
                        head.push('/');
                        head.push_str(body);
                    }
                }
            }
            head
        }
    };
    head.push(' ');
    head.push_str(&identity_face(hash));
    head
}

/// `number[-part][-subpart][:year]` off an adopted hash.
fn identity_face(hash: &Value) -> String {
    let mut face = String::new();
    face.push_str(optional_hash_str(hash, "number").unwrap_or(""));
    for field in ["part", "subpart"] {
        if let Some(value) = optional_hash_str(hash, field) {
            face.push('-');
            face.push_str(value);
        }
    }
    if let Some(year) = optional_hash_str(hash, "year") {
        face.push(':');
        face.push_str(year);
    }
    face
}

// ---------------------------------------------------------------------------
// URN
// ---------------------------------------------------------------------------

/// The URN for one identifier (the reference's CenUrnGenerator).
fn urn_for(identifier: &CenIdentifier) -> String {
    match identifier.kind {
        Amendment | Corrigendum | Fragment => {
            let base = identifier
                .base
                .as_deref()
                .map(urn_for)
                .unwrap_or_else(|| "urn:cen".to_string());
            let mut segments = vec![base];
            segments.extend(supplement_segments(identifier));
            segments.join(":")
        }
        ConsolidatedIdentifier => {
            let mut iter = identifier.identifiers.iter();
            let mut segments = vec![iter.next().map(urn_for).unwrap_or_else(|| "urn:cen".into())];
            for supplement in iter {
                segments.push("plus".to_string());
                segments.extend(supplement_segments(supplement));
            }
            segments.join(":")
        }
        AdoptedEuropeanNorm => adopted_urn(identifier),
        EuropeanPrestandard if identifier.adopted.is_some() => adopted_urn(identifier),
        _ => base_urn(identifier),
    }
}

/// The adopted-document URN: the publisher's lowercase word, then the
/// adopted MR string, dots/underscores mapped to colons, plus the
/// stage segment.
fn adopted_urn(identifier: &CenIdentifier) -> String {
    let publisher = identifier
        .publisher
        .as_deref()
        .unwrap_or_else(|| identifier.kind.default_publisher().unwrap_or("EN"))
        .to_lowercase();
    let mr = identifier
        .adopted
        .as_ref()
        .map(|adopted| adopted_mr(&adopted.to_hash()))
        .unwrap_or_default();
    let body = if mr.is_empty() {
        publisher
    } else {
        format!("{publisher}.{mr}")
    }
    .replace(['.', '_'], ":");
    let mut segments = vec![format!("urn:cen:{body}")];
    if let Some(stage) = identifier.stage {
        if stage.stage_code != "published" {
            segments.push(format!("stage.{}", stage.stage_code));
        }
    }
    segments.join(":")
}

/// The document base URN (`urn:cen:<publisher>[-copubs]:<number>...`).
fn base_urn(identifier: &CenIdentifier) -> String {
    let publisher = identifier
        .publisher
        .as_deref()
        .unwrap_or_else(|| identifier.kind.default_publisher().unwrap_or("EN"))
        .to_lowercase();
    let mut head = publisher;
    if !identifier.copublishers.is_empty() {
        head.push('-');
        head.push_str(
            &identifier
                .copublishers
                .iter()
                .map(|copublisher| copublisher.to_lowercase())
                .collect::<Vec<_>>()
                .join("-"),
        );
    }
    let mut parts = vec!["urn".to_string(), "cen".to_string(), head];
    if let Some(number) = &identifier.number {
        parts.push(match &identifier.part {
            Some(part) => format!("{number}-{part}"),
            None => number.clone(),
        });
    }
    if let Some(year) = &identifier.year {
        let mut date = year.clone();
        if let Some(month) = &identifier.month {
            date.push('-');
            date.push_str(month);
        }
        parts.push(date);
    }
    if let Some(stage) = identifier.stage {
        if stage.stage_code != "published" {
            parts.push(format!("stage.{}", stage.stage_code));
        }
    }
    parts.join(":")
}

/// The supplement URN segments (`amd`/`cor`/`frag` chains).
fn supplement_segments(supplement: &CenIdentifier) -> Vec<String> {
    match supplement.kind {
        Fragment => vec![
            "frag".to_string(),
            optional_str(&supplement.number).to_string(),
        ]
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect(),
        Corrigendum => {
            let mut segments = vec!["cor".to_string()];
            if let Some(number) = &supplement.number {
                segments.push(number.clone());
            }
            if let Some(year) = &supplement.year {
                let mut date = year.clone();
                if let Some(month) = &supplement.month {
                    date.push('-');
                    date.push_str(month);
                }
                segments.push(date);
            }
            segments
        }
        _ => {
            let mut segments = vec!["amd".to_string()];
            if let Some(number) = &supplement.number {
                segments.push(number.clone());
            }
            if let Some(year) = &supplement.year {
                segments.push(year.clone());
            }
            segments
        }
    }
}

/// The adopted document's MR string (`iso.ts.21003-7.2008`, the iso
/// amendment recursion `_amd.n.y`) — dots and underscores map to
/// colons in the URN body.
fn adopted_mr(hash: &Value) -> String {
    let type_tag = hash.get("_type").and_then(Value::as_str).unwrap_or("");
    let mut segments: Vec<String> = Vec::new();
    if type_tag.starts_with("pubid:iso") {
        let copublishers = hash
            .get("copublishers")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|copublisher| copublisher.to_lowercase())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        segments.push(format!(
            "iso{}",
            copublishers
                .iter()
                .fold(String::new(), |head, copublisher| format!(
                    "{head}-{copublisher}"
                ))
        ));
    } else if type_tag.starts_with("pubid:iec") {
        segments.push("iec".to_string());
    }
    let type_suffix = type_tag.split(':').nth(2).unwrap_or("");
    let type_segment = match type_suffix {
        "technical-specification" => Some("ts"),
        "technical-report" => Some("tr"),
        _ => None,
    };
    if let Some(segment) = type_segment {
        segments.push(segment.to_string());
    }
    let identity = ["number", "part", "subpart"]
        .iter()
        .filter_map(|field| optional_hash_str(hash, field))
        .collect::<Vec<_>>()
        .join("-");
    if !identity.is_empty() {
        segments.push(identity);
    }
    if let Some(year) = optional_hash_str(hash, "year") {
        segments.push(year.to_string());
    }

    if let Some(base) = hash
        .get("base")
        .filter(|base| base.is_object() && !base.is_null())
    {
        let base_mr = adopted_mr(base);
        let suffix_tag = if type_suffix.ends_with("amendment") {
            "amd"
        } else {
            "cor"
        };
        let mut suffix = String::new();
        for (index, field) in ["number", "year"].iter().enumerate() {
            if let Some(value) = optional_hash_str(hash, field) {
                if index > 0 {
                    suffix.push('.');
                }
                suffix.push_str(value);
            }
        }
        return if suffix.is_empty() {
            format!("{base_mr}_{suffix_tag}")
        } else {
            format!("{base_mr}_{suffix_tag}.{suffix}")
        };
    }
    segments.join(".")
}

#[cfg(test)]
mod tests {
    use super::CenIdentifier;
    use serde_json::json;

    #[test]
    fn the_european_norm_parses_number_part_year() {
        let id = CenIdentifier::parse("EN 13250:2014").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:cencenelec:european-norm",
                "number": "13250",
                "year": "2014",
            })
        );
        assert_eq!(id.to_human(), "EN 13250:2014");
        assert_eq!(id.to_urn(), "urn:cen:en:13250:2014");
    }

    #[test]
    fn the_part_and_version_faces_compose() {
        let id = CenIdentifier::parse("EN 302 326-2 V2.2.1:2026").expect("parse");
        assert_eq!(id.number.as_deref(), Some("302 326"));
        assert_eq!(id.part.as_deref(), Some("2"));
        assert_eq!(id.version.as_deref(), Some(" V2.2.1"));
        assert_eq!(id.to_human(), "EN 302 326-2 V2.2.1:2026");
        assert_eq!(id.to_urn(), "urn:cen:en:302 326-2:2026");
    }

    #[test]
    fn the_draft_stages_render_their_words_and_urn_stages() {
        let pren = CenIdentifier::parse("prEN 10160").expect("parse");
        assert_eq!(pren.to_human(), "prEN 10160");
        assert_eq!(pren.to_urn(), "urn:cen:en:10160:stage.proposal");
        assert_eq!(pren.to_hash()["stage"], json!("pren"));

        let fpren = CenIdentifier::parse("FprEN 16114:2011").expect("parse");
        assert_eq!(fpren.to_human(), "FprEN 16114:2011");
        assert_eq!(fpren.to_urn(), "urn:cen:en:16114:2011:stage.final_proposal");
    }

    #[test]
    fn the_publisher_classes_render_their_words() {
        for (input, human, urn) in [
            ("CR 10313:2000", "CR 10313:2000", "urn:cen:cr:10313:2000"),
            (
                "CWA 14050-21:2000",
                "CWA 14050-21:2000",
                "urn:cen:cwa:14050-21:2000",
            ),
            ("ENV 1613:1995", "ENV 1613:1995", "urn:cen:env:1613:1995"),
            (
                "ES 59008-1:2000",
                "ES 59008-1:2000",
                "urn:cen:es:59008-1:2000",
            ),
            ("HD 1215-2:1988", "HD 1215-2:1988", "urn:cen:hd:1215-2:1988"),
        ] {
            let id = CenIdentifier::parse(input).expect(input);
            assert_eq!(id.to_human(), human, "{input}");
            assert_eq!(id.to_urn(), urn, "{input}");
        }
    }

    #[test]
    fn guides_and_copublishers_render_the_rows() {
        let cen = CenIdentifier::parse("CEN Guide 18:2021").expect("parse");
        assert_eq!(cen.to_human(), "CEN Guide 18:2021");
        assert_eq!(cen.to_urn(), "urn:cen:cen:18:2021");

        let clc = CenIdentifier::parse("CLC Guide 1").expect("parse");
        assert_eq!(clc.to_human(), "CLC Guide 1");
        assert_eq!(clc.to_urn(), "urn:cen:clc:1");

        let joint = CenIdentifier::parse("CEN/CLC Guide 25:2023").expect("parse");
        assert_eq!(joint.to_hash()["copublishers"], json!(["CLC"]));
        assert_eq!(joint.to_human(), "CEN/CLC Guide 25:2023");
        assert_eq!(joint.to_urn(), "urn:cen:cen-clc:25:2023");

        let trio = CenIdentifier::parse("CEN/CLC/ETSI/TR 101551:2026").expect("parse");
        assert_eq!(trio.to_hash()["copublishers"], json!(["CLC", "ETSI"]));
        assert_eq!(trio.to_human(), "CEN/CLC/ETSI/TR 101551:2026");
        assert_eq!(trio.to_urn(), "urn:cen:cen-clc-etsi:101551:2026");
    }

    #[test]
    fn the_adopted_forms_render_the_iso_iec_faces() {
        for (input, human, urn) in [
            (
                "CEN ISO/TS 21003-7:2008",
                "CEN ISO/TS 21003-7:2008",
                "urn:cen:cen:iso:ts:21003-7:2008",
            ),
            (
                "EN IEC 55014-2:2021",
                "EN IEC 55014-2:2021",
                "urn:cen:en:iec:55014-2:2021",
            ),
            (
                "EN ISO 13485:2016/Amd 1:2016",
                "EN ISO 13485:2016/Amd 1:2016",
                "urn:cen:en:iso:13485:2016:amd:1:2016",
            ),
            (
                "HD IEC 60364-8-81",
                "HD IEC 60364-8-81",
                "urn:cen:hd:iec:60364-8-81",
            ),
        ] {
            let id = CenIdentifier::parse(input).expect(input);
            assert_eq!(id.to_human(), human, "{input}");
            assert_eq!(id.to_urn(), urn, "{input}");
        }
    }

    #[test]
    fn the_supplements_render_standalone_and_consolidated() {
        let amendment = CenIdentifier::parse("EN 60335-1:2012/A1").expect("parse");
        assert_eq!(amendment.to_human(), "EN 60335-1:2012/A1");
        assert_eq!(amendment.to_urn(), "urn:cen:en:60335-1:2012:amd:1");

        let corrigendum = CenIdentifier::parse("EN 13254:2000/AC:2003").expect("parse");
        assert_eq!(corrigendum.to_human(), "EN 13254:2000/AC:2003");
        assert_eq!(corrigendum.to_urn(), "urn:cen:en:13254:2000:cor:2003");

        let month = CenIdentifier::parse("EN 61375-2-3:2015/AC:2016-11").expect("parse");
        assert_eq!(month.to_human(), "EN 61375-2-3:2015/AC:2016-11");
        assert_eq!(month.to_urn(), "urn:cen:en:61375-2-3:2015:cor:2016-11");

        let consolidated = CenIdentifier::parse("EN 13250:2014+A1:2015").expect("parse");
        assert_eq!(consolidated.to_human(), "EN 13250:2014+A1:2015");
        assert_eq!(
            consolidated.to_urn(),
            "urn:cen:en:13250:2014:plus:amd:1:2015"
        );
        assert_eq!(
            consolidated.to_hash()["identifiers"][1],
            json!({"_type": "pubid:cencenelec:amendment", "number": "1", "year": "2015"})
        );
    }

    #[test]
    fn the_fragment_form_nests_the_amendment() {
        let id = CenIdentifier::parse("EN 60038/A1 FRAG2").expect("parse");
        assert_eq!(id.to_human(), "EN 60038/A1 FRAG2");
        assert_eq!(id.to_urn(), "urn:cen:en:60038:amd:1:frag:2");
    }

    #[test]
    fn the_comment_and_parenthetical_tails_normalize() {
        let id =
            CenIdentifier::parse("EN ISO 13485:2016/AC:2016# (first corrigendum)").expect("parse");
        assert_eq!(id.to_human(), "EN ISO 13485:2016/AC:2016");

        let guide = CenIdentifier::parse("CEN-CLC Guide 32:2016").expect("parse");
        assert_eq!(guide.to_human(), "CEN/CLC Guide 32:2016");
    }

    #[test]
    fn en_tr_is_rejected() {
        assert!(CenIdentifier::parse("EN TR 100").is_err());
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "EN 1325-1:1996",
            "FprEN 50600-2-4",
            "EN ISO 13485:2016/Amd 1:2016",
            "EN 60038/A1 FRAG2",
            "CLC/TR 62125:2008",
            "EN 14891:2007/AC2:2009",
        ] {
            let id = CenIdentifier::parse(spelling).expect(spelling);
            let rebuilt = CenIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{spelling}");
        }
    }
}
