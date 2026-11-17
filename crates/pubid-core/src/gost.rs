//! The gost flavor: interstate and national standards, harmonized
//! editions, and identical adoptions.
//!
//! The artifact binds the raw faces (`gost_word`, `scope_r`,
//! `prefix_text`, `raw`, `adopted_raw`, `adopted_reference_raw`); the
//! flavor layer does the reference's cast over them:
//!
//! - the main `raw` splits at the last dash into number + year when
//!   the trailing segment is a 2+ digit run (single-digit trailing
//!   segments are part of the number face: `ISO 17636-1` keeps
//!   `17636-1`), with the em-dash and spaced forms normalized first
//!   (`71039— 2023` renders `71039-2023`);
//! - the `prefix_text` carries the copublisher and its subtype word
//!   (`ISO Guide`, `ISO/IEC ISP`), with the Russian print forms mapped
//!   to their Latin faces (`МЭК` → `IEC`, `ИСО/МЭК МФС` →
//!   `ISO/IEC ISP`); unmapped spellings keep their captured face
//!   (`ИСО/ТС` stays, per the corpus);
//! - the kind dispatches off the adopted faces: the paren form is
//!   harmonized, the slash form an identical adoption, `scope_r` the
//!   national class, else interstate;
//! - the adopted reference (`(ISO 9902-2:2001)`, `/EN 1548:2007`)
//!   parses through the bounded adopted-reference cast (ISO/IEC/EN/
//!   ASTM faces and the CEN-CENELEC consolidated form; everything
//!   else — UNECE, OECD, IECEE — stays a foreign reference that
//!   renders its captured text). The `: YYYY` noise face (space after
//!   the colon) is ingestion noise the reference drops.
//!
//! The nested `base` mirrors the main number/year under its
//! interstate/national tag; the harmonized wire carries
//! `adopted_identifiers` as an array, the identical adoption a single
//! `adopted`.

use serde_json::Value;

use crate::artifact::gost_artifact;
use crate::bound::{bound_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete gost class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GostKind {
    Harmonized,
    IdenticalAdoption,
    InterstateStandard,
    NationalStandard,
}

use GostKind::*;

impl GostKind {
    /// The shared `_type` prefix every gost tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:gost:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            Harmonized => "pubid:gost:harmonized",
            IdenticalAdoption => "pubid:gost:identical-adoption",
            InterstateStandard => "pubid:gost:interstate-standard",
            NationalStandard => "pubid:gost:national-standard",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [
            Harmonized,
            IdenticalAdoption,
            InterstateStandard,
            NationalStandard,
        ]
        .into_iter()
        .find(|kind| kind.type_tag() == tag)
    }
}

/// An adopted reference: a parsed wire hash, or the captured text of a
/// foreign reference (the wire hash is the empty foreign tag; the
/// human still renders the captured text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adopted {
    /// A foreign reference kept by its captured text.
    Foreign(String),
    /// A parsed adopted identifier (ISO/IEC/EN/ASTM/consolidated).
    Hash(Value),
}

impl Adopted {
    /// The wire hash (the foreign tag carries no captured text).
    fn to_hash(&self) -> Value {
        match self {
            Adopted::Foreign(_) => serde_json::json!({"_type": FOREIGN_REFERENCE}),
            Adopted::Hash(hash) => hash.clone(),
        }
    }

    fn from_hash(hash: &Value) -> Self {
        if hash.get("_type").and_then(Value::as_str) == Some(FOREIGN_REFERENCE) {
            Adopted::Foreign(String::new())
        } else {
            Adopted::Hash(hash.clone())
        }
    }

    /// The canonical human face of the adopted reference.
    fn to_human(&self) -> String {
        match self {
            Adopted::Foreign(raw) => raw.clone(),
            Adopted::Hash(hash) => render_adopted_hash(hash),
        }
    }
}

/// The foreign-reference tag (an unparseable adopted origin).
pub const FOREIGN_REFERENCE: &str = "pubid:gost:foreign-reference";

/// The gost identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GostIdentifier {
    /// The concrete class (drives `_type`, the render shape, and the
    /// adopted wire key).
    pub kind: GostKind,
    /// Whether the Russian `R` scope was captured (`GOST R ...`); an
    /// identical adoption keeps its class and renders the scope.
    pub scope_r: bool,
    /// The document number (the raw's dash segments minus the year).
    pub number: String,
    /// The trailing year of the raw (`2024` in `1437-2024`).
    pub year: Option<String>,
    /// The copublisher face (`ISO`, `IEC`, `ИСО/ТС` as captured).
    pub copublisher: Option<String>,
    /// The subtype word after the copublisher (`Guide`, `ISP`).
    pub subtype: Option<String>,
    /// The adopted reference (harmonized/identical only).
    pub adopted: Option<Adopted>,
    /// The nested base hash (harmonized/identical only).
    pub base: Option<Value>,
}

impl GostIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// reference's cast over the raw captures.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = gost_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let (number, year) = split_raw(bound_str(&bound, "raw")?);
        let (copublisher, subtype) = match optional_bound_str(&bound, "prefix_text") {
            Some(prefix_text) => split_prefix(prefix_text),
            None => (None, None),
        };
        let scope_r = optional_bound_str(&bound, "scope_r").is_some();
        let adopted_raw = optional_bound_str(&bound, "adopted_raw");
        let adopted_reference_raw = optional_bound_str(&bound, "adopted_reference_raw");

        let (kind, adopted, base) = if let Some(raw) = adopted_raw {
            (
                IdenticalAdoption,
                Some(parse_adopted(raw)),
                Some(base_hash(scope_r, &number, year.as_deref())),
            )
        } else if let Some(raw) = adopted_reference_raw {
            (
                Harmonized,
                Some(parse_adopted(raw)),
                Some(base_hash(scope_r, &number, year.as_deref())),
            )
        } else if scope_r {
            (NationalStandard, None, None)
        } else {
            (InterstateStandard, None, None)
        };

        Ok(Self {
            kind,
            scope_r,
            number,
            year,
            copublisher,
            subtype,
            adopted,
            base,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(GostKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown gost _type {:?}", hash.get("_type")),
            })?;
        let base = hash.get("base").cloned().filter(|base| !base.is_null());
        let scope_r = kind == NationalStandard
            || base
                .as_ref()
                .map(|base| {
                    base.get("_type").and_then(Value::as_str) == Some(NationalStandard.type_tag())
                })
                .unwrap_or(false);
        let adopted = match kind {
            Harmonized => hash
                .get("adopted_identifiers")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .map(Adopted::from_hash),
            IdenticalAdoption => hash.get("adopted").map(Adopted::from_hash),
            _ => None,
        };
        Ok(Self {
            kind,
            scope_r,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            year: optional_hash_str(hash, "year").map(str::to_string),
            copublisher: optional_hash_str(hash, "copublisher").map(str::to_string),
            subtype: optional_hash_str(hash, "subtype").map(str::to_string),
            adopted,
            base,
        })
    }

    /// The canonical hash: `_type`, the main faces, then the adopted
    /// structures (harmonized carries the array, the identical
    /// adoption the single hash; both carry the nested base).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        if let Some(copublisher) = &self.copublisher {
            object.insert("copublisher".into(), copublisher.clone().into());
        }
        if let Some(subtype) = &self.subtype {
            object.insert("subtype".into(), subtype.clone().into());
        }
        if let Some(adopted) = &self.adopted {
            match self.kind {
                Harmonized => {
                    object.insert(
                        "adopted_identifiers".into(),
                        Value::Array(vec![adopted.to_hash()]),
                    );
                }
                IdenticalAdoption => {
                    object.insert("adopted".into(), adopted.to_hash());
                }
                _ => {}
            }
        }
        if let Some(base) = &self.base {
            object.insert("base".into(), base.clone());
        }
        Value::Object(object)
    }

    /// The canonical human: `GOST [R] [copublisher [subtype]] number
    /// [-year]` plus the adopted face (paren for harmonized, slash for
    /// the identical adoption).
    pub fn to_human(&self) -> String {
        let mut out = "GOST".to_string();
        if self.scope_r {
            out.push_str(" R");
        }
        if let Some(copublisher) = &self.copublisher {
            out.push(' ');
            out.push_str(copublisher);
            if let Some(subtype) = &self.subtype {
                out.push(' ');
                out.push_str(subtype);
            }
        }
        out.push(' ');
        out.push_str(&self.number);
        if let Some(year) = &self.year {
            out.push('-');
            out.push_str(year);
        }
        if let Some(adopted) = &self.adopted {
            let face = adopted.to_human();
            match self.kind {
                Harmonized => out.push_str(&format!(" ({face})")),
                IdenticalAdoption => {
                    out.push('/');
                    out.push_str(&face);
                }
                _ => {}
            }
        }
        out
    }

    /// The URN: `urn:gost:std[:r]:<number>[:<year>]`.
    pub fn to_urn(&self) -> String {
        let mut out = "urn:gost:std".to_string();
        if self.scope_r {
            out.push_str(":r");
        }
        out.push(':');
        out.push_str(&self.number);
        if let Some(year) = &self.year {
            out.push(':');
            out.push_str(year);
        }
        out
    }
}

/// The nested base hash: the main number/year under its scope tag.
fn base_hash(scope_r: bool, number: &str, year: Option<&str>) -> Value {
    let mut object = serde_json::Map::new();
    let tag = if scope_r {
        NationalStandard.type_tag()
    } else {
        InterstateStandard.type_tag()
    };
    object.insert("_type".into(), tag.into());
    object.insert("number".into(), number.into());
    if let Some(year) = year {
        object.insert("year".into(), year.into());
    }
    Value::Object(object)
}

/// The main `raw` cast: em-dash and spaced dash forms normalize, then
/// the trailing 2+ digit segment splits off as the year.
fn split_raw(raw: &str) -> (String, Option<String>) {
    let normalized = raw.replace('—', "-");
    let normalized = normalized.replace("- ", "-");
    match normalized.rsplit_once('-') {
        Some((number, year))
            if !year.is_empty() && year.len() >= 2 && year.chars().all(|c| c.is_ascii_digit()) =>
        {
            (number.to_string(), Some(year.to_string()))
        }
        _ => (normalized, None),
    }
}

/// The Russian print forms the reference maps to their Latin faces.
/// Unmapped spellings (`ИСО/ТС`) keep their captured face.
fn map_spelling(token: &str) -> &str {
    match token {
        "ЕН" => "EN",
        "ИСО" => "ISO",
        "МЭК" => "IEC",
        "ИСО/МЭК" => "ISO/IEC",
        "ИСО/ТО" => "ISO/TR",
        "ИСО/ТУ" => "ISO/TS",
        "МФС" => "ISP",
        other => other,
    }
}

/// The subtype word the prefix can carry (`ISO Guide`, `ISO/IEC ISP`).
fn is_subtype_word(token: &str) -> bool {
    matches!(token, "Guide" | "ISP" | "МФС")
}

/// The `prefix_text` cast: the subtype word splits off, the
/// copublisher maps through the spelling table.
fn split_prefix(prefix_text: &str) -> (Option<String>, Option<String>) {
    let mut words: Vec<&str> = prefix_text.split(' ').collect();
    let subtype = match words.last() {
        Some(last) if is_subtype_word(last) => Some(map_spelling(last).to_string()),
        _ => None,
    };
    if subtype.is_some() {
        words.pop();
    }
    let joined = words.join(" ");
    let copublisher = map_spelling(&joined);
    (
        (!copublisher.is_empty()).then(|| copublisher.to_string()),
        subtype,
    )
}

/// The adopted-reference ingestion: the `: YYYY` noise face (space
/// after the colon) is dropped, exactly like the reference.
fn normalize_adopted(raw: &str) -> String {
    match raw.find(": ") {
        Some(index) => raw[..index].trim_end().to_string(),
        None => raw.to_string(),
    }
}

/// The adopted-reference cast: origin token → face parser; unknown
/// origins stay foreign references (rendered from their captured
/// text).
fn parse_adopted(raw: &str) -> Adopted {
    let raw = normalize_adopted(raw);
    let (origin, rest) = match raw.split_once(' ') {
        Some((origin, rest)) => (map_spelling(origin), rest),
        None => (raw.as_str(), ""),
    };
    match origin {
        "ASTM" => parse_astm(rest).map_or_else(|| Adopted::Foreign(raw.clone()), Adopted::Hash),
        "ISO" | "ISO/TR" | "ISO/IEC" | "IEC" | "EN" => parse_iso_like(origin, rest)
            .map_or_else(|| Adopted::Foreign(raw.clone()), Adopted::Hash),
        _ => Adopted::Foreign(raw),
    }
}

/// The ASTM cast: `D129-18` → letter + number + 20xx year.
fn parse_astm(rest: &str) -> Option<Value> {
    let bytes = rest.as_bytes();
    let letter_end = bytes
        .iter()
        .position(|b| b.is_ascii_digit())
        .unwrap_or(bytes.len());
    let (letter, tail) = rest.split_at(letter_end);
    if letter.is_empty() || !letter.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let mut pieces = tail.split(['-', ':']);
    let number = pieces.next()?.to_string();
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut hash = serde_json::Map::new();
    hash.insert("_type".into(), "pubid:astm:standard".into());
    hash.insert("letter".into(), letter.into());
    hash.insert("number".into(), number.into());
    for piece in pieces {
        if piece.len() == 2 && piece.chars().all(|c| c.is_ascii_digit()) {
            hash.insert("year".into(), format!("20{piece}").into());
        } else if piece.len() == 4 && piece.chars().all(|c| c.is_ascii_digit()) {
            hash.insert("year".into(), piece.to_string().into());
        }
    }
    Some(Value::Object(hash))
}

/// The ISO/IEC/EN cast: `<origin> <number>[-part[-subpart]][:year]`,
/// plus the CEN-CENELEC consolidated `+A1:2020` form under `EN`.
fn parse_iso_like(origin: &str, rest: &str) -> Option<Value> {
    if origin == "EN" {
        if let Some((main, amendment)) = rest.split_once('+') {
            let main = parse_iso_like(origin, main)?;
            let amendment = parse_amendment(amendment)?;
            return Some(serde_json::json!({
                "_type": "pubid:cencenelec:consolidated-identifier",
                "identifiers": [main, amendment],
            }));
        }
        let (mut hash, year) = number_part_year(rest, "pubid:cencenelec:european-norm")?;
        if let Some(year) = year {
            hash.insert("year".into(), year.into());
        }
        return Some(Value::Object(hash));
    }
    let type_tag = match origin {
        "ISO/TR" => "pubid:iso:technical-report",
        "ISO" => "pubid:iso:international-standard",
        "ISO/IEC" => "pubid:iso:international-standard",
        "IEC" => "pubid:iec:international-standard",
        _ => return None,
    };
    let (mut hash, year) = number_part_year(rest, type_tag)?;
    if origin == "ISO/IEC" {
        hash.insert("copublishers".into(), Value::Array(vec!["IEC".into()]));
    }
    if let Some(year) = year {
        hash.insert("year".into(), year.into());
    }
    Some(Value::Object(hash))
}

/// The CEN-CENELEC amendment cast: `A1:2020`.
fn parse_amendment(rest: &str) -> Option<Value> {
    let (number, year) = rest.strip_prefix('A')?.split_once(':')?;
    Some(serde_json::json!({
        "_type": "pubid:cencenelec:amendment",
        "number": number,
        "year": year,
    }))
}

/// Split `<number>[-part[-subpart]][:year]` into the hash fields.
fn number_part_year(
    rest: &str,
    type_tag: &str,
) -> Option<(serde_json::Map<String, Value>, Option<String>)> {
    let (base, year) = match rest.split_once(':') {
        Some((base, year)) => (base, Some(year.to_string())),
        None => (rest, None),
    };
    let mut segments = base.split('-');
    let number = segments.next()?.to_string();
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut hash = serde_json::Map::new();
    hash.insert("_type".into(), type_tag.into());
    hash.insert("number".into(), number.into());
    if let Some(part) = segments.next() {
        hash.insert("part".into(), part.to_string().into());
    }
    if let Some(subpart) = segments.next() {
        hash.insert("subpart".into(), subpart.to_string().into());
    }
    Some((hash, year))
}

/// Render a parsed adopted hash back to its human face.
fn render_adopted_hash(hash: &Value) -> String {
    let type_tag = hash.get("_type").and_then(Value::as_str).unwrap_or("");
    match type_tag {
        "pubid:astm:standard" => {
            let mut out = format!(
                "ASTM {}{}",
                optional_hash_str(hash, "letter").unwrap_or(""),
                optional_hash_str(hash, "number").unwrap_or(""),
            );
            if let Some(year) = optional_hash_str(hash, "year") {
                if year.len() == 4 {
                    out.push_str(&format!("-{}", &year[2..]));
                }
            }
            out
        }
        "pubid:cencenelec:consolidated-identifier" => hash
            .get("identifiers")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(render_adopted_hash)
                    .collect::<Vec<_>>()
                    .join("+")
            })
            .unwrap_or_default(),
        "pubid:cencenelec:amendment" => format!(
            "A{}:{}",
            optional_hash_str(hash, "number").unwrap_or(""),
            optional_hash_str(hash, "year").unwrap_or(""),
        ),
        "pubid:iso:technical-report" => render_origin_hash(hash, "ISO/TR"),
        "pubid:iso:international-standard" => {
            let mut head = "ISO".to_string();
            if let Some(copublishers) = hash.get("copublishers").and_then(Value::as_array) {
                for copublisher in copublishers {
                    if let Some(body) = copublisher.as_str() {
                        head.push('/');
                        head.push_str(body);
                    }
                }
            }
            render_origin_hash(hash, &head)
        }
        "pubid:iec:international-standard" => render_origin_hash(hash, "IEC"),
        "pubid:cencenelec:european-norm" => render_origin_hash(hash, "EN"),
        _ => String::new(),
    }
}

/// The origin body render: `head number[-part][-subpart][:year]`.
fn render_origin_hash(hash: &Value, head: &str) -> String {
    let mut out = format!(
        "{} {}",
        head,
        optional_hash_str(hash, "number").unwrap_or("")
    );
    if let Some(part) = optional_hash_str(hash, "part") {
        out.push('-');
        out.push_str(part);
    }
    if let Some(subpart) = optional_hash_str(hash, "subpart") {
        out.push('-');
        out.push_str(subpart);
    }
    if let Some(year) = optional_hash_str(hash, "year") {
        out.push(':');
        out.push_str(year);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{GostIdentifier, GostKind};
    use serde_json::json;

    #[test]
    fn the_interstate_standard_splits_number_and_year() {
        let id = GostIdentifier::parse("GOST 10052-2006").expect("parse");
        assert_eq!(id.kind, GostKind::InterstateStandard);
        assert_eq!(id.number, "10052");
        assert_eq!(id.year.as_deref(), Some("2006"));
        assert_eq!(id.to_human(), "GOST 10052-2006");
        assert_eq!(id.to_urn(), "urn:gost:std:10052:2006");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:gost:interstate-standard",
                "number": "10052",
                "year": "2006",
            })
        );
    }

    #[test]
    fn a_single_digit_trailing_segment_stays_in_the_number() {
        let id = GostIdentifier::parse("GOST ISO 17636-1").expect("parse");
        assert_eq!(id.number, "17636-1");
        assert!(id.year.is_none());
        assert_eq!(id.copublisher.as_deref(), Some("ISO"));
        assert_eq!(id.to_human(), "GOST ISO 17636-1");
    }

    #[test]
    fn the_em_dash_alias_renders_the_canonical_face() {
        for spelling in ["ГОСТ Р 71039— 2023", "ГОСТ Р 71039—2023"] {
            let id = GostIdentifier::parse(spelling).expect(spelling);
            assert_eq!(id.to_human(), "GOST R 71039-2023", "{spelling}");
            assert_eq!(id.to_urn(), "urn:gost:std:r:71039:2023");
        }
    }

    #[test]
    fn the_prefix_carries_copublisher_and_subtype() {
        let guide = GostIdentifier::parse("GOST ISO Guide 30-2019").expect("parse");
        assert_eq!(guide.copublisher.as_deref(), Some("ISO"));
        assert_eq!(guide.subtype.as_deref(), Some("Guide"));
        assert_eq!(guide.to_human(), "GOST ISO Guide 30-2019");

        let isp = GostIdentifier::parse("ГОСТ Р ИСО/МЭК МФС 10609-9-95").expect("parse");
        assert_eq!(isp.copublisher.as_deref(), Some("ISO/IEC"));
        assert_eq!(isp.subtype.as_deref(), Some("ISP"));
        assert_eq!(isp.to_human(), "GOST R ISO/IEC ISP 10609-9-95");
        assert_eq!(isp.to_urn(), "urn:gost:std:r:10609-9:95");
    }

    #[test]
    fn unmapped_spellings_keep_their_captured_face() {
        let id = GostIdentifier::parse("GOST R ИСО/ТС 10303-439-2016").expect("parse");
        assert_eq!(id.copublisher.as_deref(), Some("ИСО/ТС"));
        assert_eq!(id.to_human(), "GOST R ИСО/ТС 10303-439-2016");
    }

    #[test]
    fn the_harmonized_paren_form_parses_the_adopted_reference() {
        let id = GostIdentifier::parse("GOST 1437-2024 (ASTM D129-18)").expect("parse");
        assert_eq!(id.kind, GostKind::Harmonized);
        assert_eq!(id.to_human(), "GOST 1437-2024 (ASTM D129-18)");
        assert_eq!(id.to_urn(), "urn:gost:std:1437:2024");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:gost:harmonized",
                "number": "1437",
                "year": "2024",
                "adopted_identifiers": [{
                    "_type": "pubid:astm:standard",
                    "letter": "D",
                    "number": "129",
                    "year": "2018",
                }],
                "base": {
                    "_type": "pubid:gost:interstate-standard",
                    "number": "1437",
                    "year": "2024",
                },
            })
        );
    }

    #[test]
    fn the_identical_adoption_carries_a_single_adopted_hash() {
        let id = GostIdentifier::parse("GOST 31425.2-2025/ISO 9902-2:2001").expect("parse");
        assert_eq!(id.kind, GostKind::IdenticalAdoption);
        assert_eq!(id.to_human(), "GOST 31425.2-2025/ISO 9902-2:2001");
        assert_eq!(
            id.to_hash()["adopted"],
            json!({
                "_type": "pubid:iso:international-standard",
                "number": "9902",
                "part": "2",
                "year": "2001",
            })
        );
    }

    #[test]
    fn unknown_adopted_origins_stay_foreign_and_render_their_text() {
        let id =
            GostIdentifier::parse("GOST 32786-2014 (UNECE STANDARD FFV-19:2010)").expect("parse");
        assert_eq!(
            id.to_human(),
            "GOST 32786-2014 (UNECE STANDARD FFV-19:2010)"
        );
        assert_eq!(
            id.to_hash()["adopted_identifiers"],
            json!([{"_type": "pubid:gost:foreign-reference"}])
        );
    }

    #[test]
    fn the_colon_space_year_noise_drops_from_the_adopted_face() {
        let id = GostIdentifier::parse("ГОСТ 35294.2-2025 (ISO 8124-2: 2023)").expect("parse");
        assert_eq!(id.to_human(), "GOST 35294.2-2025 (ISO 8124-2)");
    }

    #[test]
    fn the_consolidated_en_form_nests_the_amendment() {
        let id = GostIdentifier::parse("GOST 35292-2025 (EN 14988:2017+A1:2020)").expect("parse");
        assert_eq!(id.to_human(), "GOST 35292-2025 (EN 14988:2017+A1:2020)");
        assert_eq!(
            id.to_hash()["adopted_identifiers"],
            json!([{
                "_type": "pubid:cencenelec:consolidated-identifier",
                "identifiers": [
                    {"_type": "pubid:cencenelec:european-norm", "number": "14988", "year": "2017"},
                    {"_type": "pubid:cencenelec:amendment", "number": "1", "year": "2020"},
                ],
            }])
        );
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "GOST 12.1.004",
            "GOST R 1.0-2015",
            "GOST R ИСО/ТС 10303-439-2016",
            "GOST 1437-2024 (ASTM D129-18)",
            "GOST R 57415-2017/EN 1548:2007",
        ] {
            let id = GostIdentifier::parse(spelling).expect(spelling);
            let rebuilt = GostIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
        }
    }
}
