//! The wmo flavor: the WMO-No. publication series and the
//! WMO-published S-100 product specifications (pubid#763, widened with
//! the volume, compound-edition and TD/JCOMM/GAW faces — grammar #142,
//! the gem's widening at pubid 2.0.49).
//!
//! The publication series share one type, per the widening ruling:
//!
//!   WMO-No. 1364                       the WMO-No. publication series
//!   WMO-No. 1364, 2025 edition         calendar-year edition (comma face)
//!   WMO-No. 1364 2025 edition          cover-stack alias
//!   WMO-No. 1364:ed2025                compact alias
//!   WMO-No. 8, Volume I                volume face (roman I..V)
//!   WMO-No. 306, Volume I.1            dotted-roman sub-volume
//!   WMO-No. 9, Volume D                letter volume
//!   WMO-No. 306, Volume I.1, 2020 edition   volume BEFORE the edition
//!                                           in the face order
//!   WMO-No. 8, 2021/2018 edition       compound year-pair edition (the
//!                                      pair IS the identity face)
//!   WMO-No. 1364, 2023 edition, updated in 2025   the updated-in year
//!   WMO-No. 8, Provisional 2026 edition           the status word
//!   WMO/TD-No. 196                     the Technical Document series
//!   JCOMM Technical Report No. 14      the joint WMO/IOC series
//!   GAW Report No. 312                 the GAW series (bare head)
//!   S-411                              the S-100 product specifications
//!   S-411 1.2.1                        dotted x.y.z edition
//!   S-411:ed1.2.1                      compact alias
//!   S-411 Annex A                      annex wrapper (the DCEG face)
//!   S-411 ANNEX A                      parse-only uppercase alias
//!
//! The edition is ONE attribute carrying a type-driven face: the
//! calendar year (or the year pair, "2021/2018") on a publication, the
//! dotted x.y.z on a standard or annex; the status word and the
//! updated-in year ride their own attributes beside it. The new series
//! follow the widening's PREFIX law: every head token names its
//! publisher, so the bare `TD-No.` spellings parse in no flavor — the
//! reject is the prefix law, not accidental grammar damage.
//!
//! The MRN URNs follow the IALA pattern (`urn:mrn:wmo:pub:...`; the
//! namespace's rule is the fleet's proposed extension), widened with
//! the series slugs (`wmo-td-`, `jcomm-tr-`, `gaw-`), the caseless
//! `vol-`/`status-`/`updated-` segments and the hyphenated year pair
//! (`ed2021-2018`; RFC 2141 has no slash in the trans set), and
//! `parse` dispatches on the `urn:` prefix like the reference's
//! `FormatDetector`.

use serde_json::Value;

use crate::artifact::wmo_artifact;
use crate::bound::{bound_str, check_type, hash_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete WMO class an identifier belongs to
/// (`Pubid::Wmo::Identifiers::*` in the reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoKind {
    /// The WMO-No. publication series (`WMO-No. 1364`), including the
    /// TD/JCOMM/GAW heads — one type, the series key discriminates.
    Publication,
    /// The WMO-published S-100 product specification (`S-411`).
    Standard,
    /// An annex to an S-100 specification (`S-411 Annex A`).
    Annex,
}

impl WmoKind {
    /// The shared `_type` prefix every wmo tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:wmo:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            WmoKind::Publication => "pubid:wmo:publication",
            WmoKind::Standard => "pubid:wmo:standard",
            WmoKind::Annex => "pubid:wmo:annex",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        match tag {
            "pubid:wmo:publication" => Some(WmoKind::Publication),
            "pubid:wmo:standard" => Some(WmoKind::Standard),
            "pubid:wmo:annex" => Some(WmoKind::Annex),
            _ => None,
        }
    }
}

/// The publication series beyond the WMO-No. default (the widening
/// prefix law: every head token names its publisher, so the model
/// carries the short key and derives the citation face and the MRN
/// slug prefix from this table — `Pubid::Wmo::Identifier::SERIES`).
/// The default row stays out of the table: series is `None` for
/// WMO-No., so existing rows serialize without the key.
const SERIES: &[(&str, &str, &str)] = &[
    // (key, citation face, MRN slug prefix)
    ("TD", "WMO/TD-No.", "wmo-td"),
    ("JCOMM", "JCOMM Technical Report No.", "jcomm-tr"),
    ("GAW", "GAW Report No.", "gaw"),
];

/// The WMO-No. series itself: the face the grammar matches and the MRN
/// slug prefix of the default series (series `None`).
const DEFAULT_SERIES: (&str, &str) = ("WMO-No.", "wmo");

/// The series config — the citation face and the MRN slug prefix — of
/// a series key, defaulting to the WMO-No. row (the reference's
/// `Identifier.series_config`, which `SERIES.fetch`es with the
/// default).
fn series_config(key: Option<&str>) -> (&'static str, &'static str) {
    match key {
        Some(key) => SERIES
            .iter()
            .find(|(name, _, _)| *name == key)
            .map(|(_, face, mrn)| (*face, *mrn))
            .unwrap_or(DEFAULT_SERIES),
        None => DEFAULT_SERIES,
    }
}

/// The series head the grammar matched -> the model's series key
/// (`Builder::SERIES_HEADS`): the widening prefix law, and the head
/// face is the grammar's verbatim capture, so the key mapping lives on
/// the one construction site.
const SERIES_HEADS: &[(&str, Option<&str>)] = &[
    ("WMO-No.", None),
    ("WMO/TD-No.", Some("TD")),
    ("JCOMM Technical Report No.", Some("JCOMM")),
    ("GAW Report No.", Some("GAW")),
];

/// The wmo identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WmoIdentifier {
    /// The concrete class (drives `_type`, the edition face, and the
    /// URN code segment).
    pub kind: WmoKind,
    /// The document number (`1364`, `411`). On an annex this is the
    /// BASE specification's number — the grammar binds one `doc_number`
    /// per match, and the reference hoists it into the base.
    pub number: String,
    /// The publication series key: `None` (the WMO-No. default), or
    /// `"TD"` / `"JCOMM"` / `"GAW"`. The citation face and the MRN
    /// slug prefix derive from [`SERIES`].
    pub series: Option<String>,
    /// The edition, with the face keyed off the kind: the calendar
    /// year — or the year pair, `"2021/2018"` — on a publication, the
    /// dotted x.y.z on a standard or annex (`"1.2.1"`).
    pub edition: Option<String>,
    /// The publication volume face (`"I"`, `"I.1"`, `"D"`);
    /// publication-only. An identity discriminator (Volume I and
    /// Volume II of WMO-No. 8 are two documents), printed before the
    /// edition face.
    pub volume: Option<String>,
    /// The edition status word (`"Provisional"`); publication-only.
    pub edition_status: Option<String>,
    /// The year the edition was updated in (`"2025"`);
    /// publication-only.
    pub updated_in: Option<String>,
    /// Annex only: the marker as spelled (`"Annex"` / `"ANNEX"`),
    /// preserved verbatim for round-trip fidelity.
    pub annex_form: Option<String>,
    /// Annex only: the letter (`"A"`); absent on the bare form.
    pub letter: Option<String>,
}

impl WmoIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// The MRN namespace the URNs live under
    /// (`Pubid::Wmo::UrnParser::PREFIX`).
    const MRN_PREFIX: &'static str = "urn:mrn:wmo:pub:";

    /// Parse an input spelling — a human form or an MRN URN — into an
    /// identifier. The `urn:` prefix dispatches to the URN grammar
    /// (the reference's `FormatDetector`, case-insensitive like its
    /// `/\Aurn:/i`).
    pub fn parse(input: &str) -> Result<Self> {
        if input.len() >= 4 && input.as_bytes()[..4].eq_ignore_ascii_case(b"urn:") {
            Self::parse_urn(input)
        } else {
            Self::parse_human(input)
        }
    }

    /// Parse a human spelling through the vendored artifact and
    /// dispatch the class off the resolved captures: `annex_marker`
    /// marks an annex, `series_head` a publication, the bare `s_code`
    /// a standard (the grammar's ordered choice, and the builder's
    /// dispatch order).
    fn parse_human(input: &str) -> Result<Self> {
        let artifact = wmo_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Self::from_bound(&bound)
    }

    /// Build from the artifact's flat bound map.
    fn from_bound(bound: &Value) -> Result<Self> {
        let number = bound_str(bound, "doc_number")?.to_string();
        let edition = optional_bound_str(bound, "edition_value").map(str::to_string);
        if let Some(annex_form) = optional_bound_str(bound, "annex_marker") {
            Ok(Self {
                kind: WmoKind::Annex,
                number,
                series: None,
                edition,
                volume: None,
                edition_status: None,
                updated_in: None,
                annex_form: Some(annex_form.to_string()),
                letter: optional_bound_str(bound, "annex_letter").map(str::to_string),
            })
        } else if let Some(head) = optional_bound_str(bound, "series_head") {
            let series = match SERIES_HEADS.iter().find(|(token, _)| *token == head) {
                Some((_, key)) => *key,
                None => {
                    return Err(Error::InvalidHash {
                        detail: format!("unknown wmo series head {head:?}"),
                    })
                }
            };
            Ok(Self {
                kind: WmoKind::Publication,
                number,
                series: series.map(str::to_string),
                edition,
                volume: optional_bound_str(bound, "volume").map(str::to_string),
                edition_status: optional_bound_str(bound, "edition_status").map(str::to_string),
                updated_in: optional_bound_str(bound, "updated_in").map(str::to_string),
                annex_form: None,
                letter: None,
            })
        } else {
            Ok(Self {
                kind: WmoKind::Standard,
                number,
                series: None,
                edition,
                volume: None,
                edition_status: None,
                updated_in: None,
                annex_form: None,
                letter: None,
            })
        }
    }

    /// Parse an MRN URN (`Pubid::Wmo::UrnParser`): the namespace
    /// prefix, one code segment, then at most one of each optional
    /// segment per class. The parser validates the segments strictly
    /// and rebuilds the human form, which re-parses through the
    /// artifact — the reference's exact shape (one construction path).
    /// Segment matching is caseless in the MRN; the rebuild picks the
    /// attested faces (`Volume`/`Annex` re-cased, the comma edition
    /// face), whose renders are canonical.
    fn parse_urn(urn: &str) -> Result<Self> {
        // The reference checks the prefix case-insensitively and
        // parses the body; every extracted value re-cases
        // deterministically, so the lowercased body is equivalent.
        let lowered = urn.to_lowercase();
        let body = lowered
            .strip_prefix(Self::MRN_PREFIX)
            .ok_or_else(|| urn_error(format!("invalid WMO URN: {urn:?}")))?;
        let parts: Vec<&str> = body.split(':').collect();
        let code = parts[0];
        let segments = &parts[1..];

        // Longest slug first: "wmo-td-" also starts with "wmo-"
        // (the reference's ordered regex dispatch).
        if let Some(number) = code.strip_prefix("wmo-td-") {
            Self::urn_publication(Some("TD"), number, code, segments)
        } else if let Some(number) = code.strip_prefix("wmo-") {
            Self::urn_publication(None, number, code, segments)
        } else if let Some(number) = code.strip_prefix("jcomm-tr-") {
            Self::urn_publication(Some("JCOMM"), number, code, segments)
        } else if let Some(number) = code.strip_prefix("gaw-") {
            Self::urn_publication(Some("GAW"), number, code, segments)
        } else if let Some(number) = code.strip_prefix("s-") {
            Self::urn_standard(number, code, segments)
        } else {
            Err(urn_error(format!("invalid WMO URN code segment: {code:?}")))
        }
    }

    /// The publication path: the optional volume, status and
    /// updated-in segments plus the single `ed<year>` edition (the
    /// year pair rides hyphenated and restores its slash), each at
    /// most once, rebuilt as the canonical comma faces.
    fn urn_publication(
        series: Option<&str>,
        number: &str,
        code: &str,
        segments: &[&str],
    ) -> Result<Self> {
        if !is_digits(number) {
            return Err(urn_error(format!("invalid WMO URN code segment: {code:?}")));
        }
        let mut volume: Option<String> = None;
        let mut status: Option<String> = None;
        let mut edition: Option<String> = None;
        let mut updated_in: Option<String> = None;
        for segment in segments {
            if let Some(rest) = segment.strip_prefix("vol-") {
                if volume.is_some() {
                    return Err(duplicate_segment("volume", segment));
                }
                if !is_volume(rest) {
                    return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
                }
                volume = Some(rest.to_ascii_uppercase());
            } else if let Some(rest) = segment.strip_prefix("status-") {
                if status.is_some() {
                    return Err(duplicate_segment("status", segment));
                }
                if !is_word(rest) {
                    return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
                }
                status = Some(capitalize(rest));
            } else if let Some(rest) = segment.strip_prefix("ed") {
                if edition.is_some() {
                    return Err(duplicate_segment("edition", segment));
                }
                if !is_year_pair(rest) {
                    return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
                }
                edition = Some(rest.replace('-', "/"));
            } else if let Some(rest) = segment.strip_prefix("updated-") {
                if updated_in.is_some() {
                    return Err(duplicate_segment("updated", segment));
                }
                if !is_year(rest) {
                    return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
                }
                updated_in = Some(rest.to_string());
            } else {
                return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
            }
        }

        let (face, _) = series_config(series);
        let mut text = format!("{face} {number}");
        if let Some(volume) = &volume {
            text.push_str(&format!(" Volume {volume}"));
        }
        if let Some(edition) = &edition {
            match &status {
                Some(status) => text.push_str(&format!(", {status} {edition} edition")),
                None => text.push_str(&format!(", {edition} edition")),
            }
        }
        if let Some(updated_in) = &updated_in {
            text.push_str(&format!(", updated in {updated_in}"));
        }
        Self::parse_human(&text)
    }

    /// The standard and annex path: an optional annex segment (at most
    /// one), then the optional dotted edition, rebuilt as the human
    /// forms (the attested `Annex` face — the uppercase `ANNEX` form
    /// stays a parse-only human alias whose URN re-renders to it).
    fn urn_standard(number: &str, code: &str, segments: &[&str]) -> Result<Self> {
        if !is_digits(number) {
            return Err(urn_error(format!("invalid WMO URN code segment: {code:?}")));
        }
        let mut letter: Option<String> = None;
        let mut edition: Option<String> = None;
        let mut has_annex = false;
        for segment in segments {
            match annex_segment(segment) {
                Some(annex_letter) => {
                    if has_annex {
                        return Err(duplicate_segment("annex", segment));
                    }
                    has_annex = true;
                    letter = annex_letter;
                }
                None => {
                    if edition.is_some() {
                        return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
                    }
                    edition = Some(Self::urn_edition(segment)?);
                }
            }
        }
        let mut text = format!("S-{number}");
        if has_annex {
            text.push_str(" Annex");
        }
        if let Some(letter) = &letter {
            text.push_str(&format!(" {letter}"));
        }
        if let Some(edition) = &edition {
            text.push(' ');
            text.push_str(edition);
        }
        Self::parse_human(&text)
    }

    /// The value of an `ed<value>` segment on the S-100 shapes,
    /// validated against the dotted shape — the reference's
    /// per-segment strictness. The segment is already lowercased by
    /// [`WmoIdentifier::parse_urn`].
    fn urn_edition(segment: &str) -> Result<String> {
        let value = segment
            .strip_prefix("ed")
            .ok_or_else(|| urn_error(format!("invalid WMO URN segment: {segment:?}")))?;
        if is_dotted(value) {
            Ok(value.to_string())
        } else {
            Err(urn_error(format!("invalid WMO URN segment: {segment:?}")))
        }
    }

    /// Build from a canonical hash (`_type`-tagged attribute record;
    /// the annex nests its base specification).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(WmoKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown wmo _type {:?}", hash.get("_type")),
            })?;
        if kind == WmoKind::Annex {
            let base = hash.get("base").ok_or_else(|| Error::InvalidHash {
                detail: "annex has no base".to_string(),
            })?;
            check_type(base, WmoKind::Standard.type_tag())?;
            Ok(Self {
                kind,
                number: hash_str(base, "number")?.to_string(),
                series: None,
                edition: optional_hash_str(hash, "edition").map(str::to_string),
                volume: None,
                edition_status: None,
                updated_in: None,
                annex_form: Some(hash_str(hash, "annex_form")?.to_string()),
                letter: optional_hash_str(hash, "letter").map(str::to_string),
            })
        } else {
            Ok(Self {
                kind,
                number: hash_str(hash, "number")?.to_string(),
                series: optional_hash_str(hash, "series").map(str::to_string),
                edition: optional_hash_str(hash, "edition").map(str::to_string),
                volume: optional_hash_str(hash, "volume").map(str::to_string),
                edition_status: optional_hash_str(hash, "edition_status").map(str::to_string),
                updated_in: optional_hash_str(hash, "updated_in").map(str::to_string),
                annex_form: None,
                letter: None,
            })
        }
    }

    /// The canonical hash. The publisher default (`WMO`) never
    /// serializes — `WMO-No.` and the S-number are designations, not
    /// publisher tokens — and neither does the nil series of the
    /// WMO-No. default. The annex nests the base specification and
    /// carries the annex's own edition; absent optionals drop.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        if self.kind == WmoKind::Annex {
            object.insert(
                "base".into(),
                serde_json::json!({
                    "_type": WmoKind::Standard.type_tag(),
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
            for (name, value) in [
                ("series", &self.series),
                ("volume", &self.volume),
                ("edition_status", &self.edition_status),
                ("updated_in", &self.updated_in),
            ] {
                if let Some(value) = value {
                    object.insert(name.into(), value.clone().into());
                }
            }
        }
        if let Some(edition) = &self.edition {
            object.insert("edition".into(), edition.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human (the reference's `Renderer`; the faces
    /// carry no publisher token). The publication face order is the
    /// widening ruling's: volume first, then the edition face (the
    /// status word inside it), then the updated-in year. The annex
    /// marker keeps its input case verbatim.
    pub fn to_human(&self) -> String {
        let mut out = match self.kind {
            WmoKind::Publication => {
                let (face, _) = series_config(self.series.as_deref());
                let mut out = format!("{face} {}", self.number);
                if let Some(volume) = &self.volume {
                    out.push_str(&format!(", Volume {volume}"));
                }
                out
            }
            WmoKind::Standard => format!("S-{}", self.number),
            WmoKind::Annex => {
                let mut out = format!(
                    "S-{} {}",
                    self.number,
                    self.annex_form.as_deref().unwrap_or("Annex")
                );
                if let Some(letter) = &self.letter {
                    out.push(' ');
                    out.push_str(letter);
                }
                out
            }
        };
        if let Some(edition) = &self.edition {
            match self.kind {
                WmoKind::Publication => {
                    match &self.edition_status {
                        Some(status) => out.push_str(&format!(", {status} {edition} edition")),
                        None => out.push_str(&format!(", {edition} edition")),
                    }
                    if let Some(updated_in) = &self.updated_in {
                        out.push_str(&format!(", updated in {updated_in}"));
                    }
                }
                _ => {
                    out.push(' ');
                    out.push_str(edition);
                }
            }
        }
        out
    }

    /// The MRN URN (`Pubid::Wmo::UrnGenerator`): the publication
    /// series codes via their [`SERIES`] slug (`wmo-<number>`,
    /// `wmo-td-<number>`, `jcomm-tr-<number>`, `gaw-<number>`), the
    /// S-100 specs keep their lexical form lowercased (`s-<number>`),
    /// an annex adds the `annex` or `annex-<letter>` segment. The
    /// segment order mirrors the human face order (volume, status,
    /// edition, updated-in); the volume and status values are caseless
    /// (downcased), and the year pair keeps its slash out of the NSS —
    /// RFC 2141 has no slash in the trans set — as a hyphen
    /// (`ed2021-2018`).
    pub fn to_urn(&self) -> String {
        let mut out = match self.kind {
            WmoKind::Publication => {
                let (_, mrn) = series_config(self.series.as_deref());
                format!("{}{mrn}-{}", Self::MRN_PREFIX, self.number)
            }
            _ => format!("{}s-{}", Self::MRN_PREFIX, self.number),
        };
        if self.kind == WmoKind::Publication {
            if let Some(volume) = &self.volume {
                out.push_str(&format!(":vol-{}", volume.to_lowercase()));
            }
            if let Some(status) = &self.edition_status {
                out.push_str(&format!(":status-{}", status.to_lowercase()));
            }
        }
        if self.kind == WmoKind::Annex {
            match &self.letter {
                Some(letter) => out.push_str(&format!(":annex-{}", letter.to_lowercase())),
                None => out.push_str(":annex"),
            }
        }
        if let Some(edition) = &self.edition {
            if self.kind == WmoKind::Publication {
                out.push_str(&format!(":ed{}", edition.replace('/', "-")));
            } else {
                out.push_str(&format!(":ed{edition}"));
            }
        }
        if self.kind == WmoKind::Publication {
            if let Some(updated_in) = &self.updated_in {
                out.push_str(&format!(":updated-{updated_in}"));
            }
        }
        out
    }
}

/// All-ASCII-digit, non-empty.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// A calendar year (`\d{4}`).
fn is_year(text: &str) -> bool {
    text.len() == 4 && is_digits(text)
}

/// The year-pair edition shape (`\d{4}(?:-\d{4})?`): one year, or the
/// pair hyphenated for the MRN charset.
fn is_year_pair(text: &str) -> bool {
    match text.split_once('-') {
        Some((first, second)) => is_year(first) && is_year(second),
        None => is_year(text),
    }
}

/// The dotted x.y[.z] shape (`\A\d+(\.\d+)+\z`): at least two
/// non-empty all-digit groups.
fn is_dotted(text: &str) -> bool {
    let groups: Vec<&str> = text.split('.').collect();
    groups.len() >= 2 && groups.iter().all(|group| is_digits(group))
}

/// The volume segment value (`[a-z]+(?:\.\d+)*`): a roman/letter head,
/// then optional dotted digit groups (the dotted-roman sub-volume).
fn is_volume(text: &str) -> bool {
    let mut groups = text.split('.');
    let head = groups.next().unwrap_or_default();
    !head.is_empty() && head.bytes().all(|b| b.is_ascii_lowercase()) && groups.all(is_digits)
}

/// A single lowercase word (`[a-z]+`, the status segment).
fn is_word(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase())
}

/// The reference's `String#capitalize`: the first letter upcased, the
/// rest downcased.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
        }
        None => String::new(),
    }
}

/// An `annex` / `annex-<letter>` segment, lowercased like the URN
/// body: `Some(None)` is the bare form, `Some(Some(letter))` the
/// lettered form (the letter upcased for the attested face), `None`
/// when the segment is not an annex at all (`annex-` with no letter
/// falls through to the edition path, as in the reference's regex).
fn annex_segment(segment: &str) -> Option<Option<String>> {
    let rest = segment.strip_prefix("annex")?;
    match rest {
        "" => Some(None),
        _ => {
            let letter = rest.strip_prefix('-')?;
            if letter.len() == 1 && letter.bytes().all(|b| b.is_ascii_lowercase()) {
                Some(Some(letter.to_ascii_uppercase()))
            } else {
                None
            }
        }
    }
}

fn urn_error(detail: impl Into<String>) -> Error {
    Error::Urn(detail.into())
}

/// The duplicate-segment rejection (the reference's
/// `raise_segment!(seg, "duplicate ...")`).
fn duplicate_segment(name: &str, segment: &str) -> Error {
    urn_error(format!(
        "invalid WMO URN segment (duplicate {name}): {segment:?}"
    ))
}

#[cfg(test)]
mod tests {
    use super::WmoIdentifier;
    use crate::wmo::WmoKind;
    use serde_json::json;

    #[test]
    fn parses_the_publication_series() {
        let id = WmoIdentifier::parse("WMO-No. 1364").expect("parse");
        assert_eq!(id.kind, WmoKind::Publication);
        assert_eq!(id.number, "1364");
        assert_eq!(id.series, None);
        assert_eq!(id.edition, None);
        assert_eq!(id.to_human(), "WMO-No. 1364");
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:wmo-1364");
        assert_eq!(
            id.to_hash(),
            json!({ "_type": "pubid:wmo:publication", "number": "1364" })
        );
    }

    #[test]
    fn the_year_edition_aliases_normalize_to_the_comma_face() {
        for input in [
            "WMO-No. 1364, 2025 edition",
            "WMO-No. 1364 2025 edition",
            "WMO-No. 1364:ed2025",
        ] {
            let id = WmoIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, WmoKind::Publication);
            assert_eq!(id.edition.as_deref(), Some("2025"));
            assert_eq!(id.to_human(), "WMO-No. 1364, 2025 edition", "{input}");
        }
        assert_eq!(
            WmoIdentifier::parse("WMO-No. 1364, 2025 edition")
                .expect("parse")
                .to_urn(),
            "urn:mrn:wmo:pub:wmo-1364:ed2025"
        );
    }

    #[test]
    fn parses_the_s100_specifications() {
        let id = WmoIdentifier::parse("S-411 1.2.1").expect("parse");
        assert_eq!(id.kind, WmoKind::Standard);
        assert_eq!(id.number, "411");
        assert_eq!(id.edition.as_deref(), Some("1.2.1"));
        assert_eq!(id.to_human(), "S-411 1.2.1");
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:s-411:ed1.2.1");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:wmo:standard",
                "number": "411",
                "edition": "1.2.1",
            })
        );
        let bare = WmoIdentifier::parse("S-411").expect("parse");
        assert_eq!(
            bare.to_hash(),
            json!({ "_type": "pubid:wmo:standard", "number": "411" })
        );
    }

    #[test]
    fn the_edition_face_is_type_driven() {
        // A dotted edition is a publication parse error; a bare
        // 4-digit run is a standard parse error (dotted only).
        assert!(WmoIdentifier::parse("WMO-No. 1364, 1.2 edition").is_err());
        assert!(WmoIdentifier::parse("S-411 1234").is_err());
    }

    #[test]
    fn parses_the_volume_faces() {
        // Roman I..V (the IMOP Guide page), a repeated roman
        // (record 35631), a letter (Resolution 10 (SERCOM-1)) and the
        // dotted-roman sub-volume (record 35713).
        for (input, volume) in [
            ("WMO-No. 8, Volume I", "I"),
            ("WMO-No. 8, Volume V", "V"),
            ("WMO-No. 49, Volume III", "III"),
            ("WMO-No. 9, Volume D", "D"),
            ("WMO-No. 306, Volume I.1", "I.1"),
        ] {
            let id = WmoIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, WmoKind::Publication);
            assert_eq!(id.volume.as_deref(), Some(volume), "{input}");
            assert_eq!(id.to_human(), input);
        }
        let id = WmoIdentifier::parse("WMO-No. 306, Volume I.1").expect("parse");
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:wmo-306:vol-i.1");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:wmo:publication",
                "number": "306",
                "volume": "I.1",
            })
        );
    }

    #[test]
    fn the_volume_precedes_the_edition_in_the_face_order() {
        let id = WmoIdentifier::parse("WMO-No. 306, Volume I.1, 2020 edition").expect("parse");
        assert_eq!(id.volume.as_deref(), Some("I.1"));
        assert_eq!(id.edition.as_deref(), Some("2020"));
        assert_eq!(id.to_human(), "WMO-No. 306, Volume I.1, 2020 edition");
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:wmo-306:vol-i.1:ed2020");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:wmo:publication",
                "number": "306",
                "edition": "2020",
                "volume": "I.1",
            })
        );
    }

    #[test]
    fn rejects_an_arabic_volume() {
        // The attested faces are roman and letters.
        assert!(WmoIdentifier::parse("WMO-No. 8, Volume 2").is_err());
    }

    #[test]
    fn parses_the_compound_editions() {
        // The year pair rides ONE edition string (the pair IS the
        // identity face).
        let pair = WmoIdentifier::parse("WMO-No. 8, 2021/2018 edition").expect("parse");
        assert_eq!(pair.edition.as_deref(), Some("2021/2018"));
        assert_eq!(pair.to_human(), "WMO-No. 8, 2021/2018 edition");
        // The slash stays out of the MR charset as a hyphen.
        assert_eq!(pair.to_urn(), "urn:mrn:wmo:pub:wmo-8:ed2021-2018");
        assert_eq!(
            pair.to_hash(),
            json!({
                "_type": "pubid:wmo:publication",
                "number": "8",
                "edition": "2021/2018",
            })
        );

        // The updated-in year rides its own attribute.
        let updated =
            WmoIdentifier::parse("WMO-No. 1364, 2023 edition, updated in 2025").expect("parse");
        assert_eq!(updated.edition.as_deref(), Some("2023"));
        assert_eq!(updated.updated_in.as_deref(), Some("2025"));
        assert_eq!(
            updated.to_human(),
            "WMO-No. 1364, 2023 edition, updated in 2025"
        );
        assert_eq!(
            updated.to_urn(),
            "urn:mrn:wmo:pub:wmo-1364:ed2023:updated-2025"
        );
        assert_eq!(
            updated.to_hash(),
            json!({
                "_type": "pubid:wmo:publication",
                "number": "1364",
                "edition": "2023",
                "updated_in": "2025",
            })
        );

        // The status word rides its own attribute.
        let provisional =
            WmoIdentifier::parse("WMO-No. 8, Provisional 2026 edition").expect("parse");
        assert_eq!(provisional.edition_status.as_deref(), Some("Provisional"));
        assert_eq!(provisional.edition.as_deref(), Some("2026"));
        assert_eq!(
            provisional.to_human(),
            "WMO-No. 8, Provisional 2026 edition"
        );
        assert_eq!(
            provisional.to_urn(),
            "urn:mrn:wmo:pub:wmo-8:status-provisional:ed2026"
        );
        assert_eq!(
            provisional.to_hash(),
            json!({
                "_type": "pubid:wmo:publication",
                "number": "8",
                "edition": "2026",
                "edition_status": "Provisional",
            })
        );
    }

    #[test]
    fn parses_the_new_series_heads() {
        // The widening prefix law: the head names its publisher.
        for (input, series, mrn) in [
            ("WMO/TD-No. 196", "TD", "wmo-td"),
            ("JCOMM Technical Report No. 14", "JCOMM", "jcomm-tr"),
            ("GAW Report No. 312", "GAW", "gaw"),
        ] {
            let id = WmoIdentifier::parse(input).expect(input);
            assert_eq!(id.kind, WmoKind::Publication, "{input}");
            assert_eq!(id.series.as_deref(), Some(series), "{input}");
            assert_eq!(id.to_human(), input);
            assert_eq!(id.to_urn(), format!("urn:mrn:wmo:pub:{mrn}-{}", id.number));
            assert_eq!(
                id.to_hash(),
                json!({
                    "_type": "pubid:wmo:publication",
                    "number": id.number,
                    "series": series,
                })
            );
        }
    }

    #[test]
    fn rejects_the_prefix_law_rows() {
        // Each fails because no series head matches — the parser's
        // failure names the four heads and "S-" — not by accidental
        // grammar damage. The bare TD-No. spellings name no publisher;
        // IMO/TD-No. 259 is an OCR misread (no IMO TD series exists);
        // WMO/GAW Report No. 285 is third-party citation only; the
        // spaced cover variant is deliberately outside the grammar.
        for input in [
            "TD-No. 1265",
            "TD No. 196",
            "IMO/TD-No. 259",
            "WMO/GAW Report No. 285",
            "WMO/TD - No. 1469",
        ] {
            let err = WmoIdentifier::parse(input).expect_err(input);
            let message = err.to_string();
            assert!(message.contains("WMO/TD-No."), "{input}: {message}");
            assert!(message.contains("GAW Report No."), "{input}: {message}");
            assert!(
                message.contains("JCOMM Technical Report No."),
                "{input}: {message}"
            );
        }
    }

    #[test]
    fn parses_the_annex_wrapper() {
        let id = WmoIdentifier::parse("S-411 Annex A").expect("parse");
        assert_eq!(id.kind, WmoKind::Annex);
        assert_eq!(id.number, "411");
        assert_eq!(id.annex_form.as_deref(), Some("Annex"));
        assert_eq!(id.letter.as_deref(), Some("A"));
        assert_eq!(id.edition, None);
        assert_eq!(id.to_human(), "S-411 Annex A");
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:s-411:annex-a");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:wmo:annex",
                "base": { "_type": "pubid:wmo:standard", "number": "411" },
                "annex_form": "Annex",
                "letter": "A",
            })
        );
    }

    #[test]
    fn the_annex_edition_belongs_to_the_annex() {
        let id = WmoIdentifier::parse("S-411 Annex A 1.2.0").expect("parse");
        assert_eq!(id.edition.as_deref(), Some("1.2.0"));
        assert_eq!(id.to_human(), "S-411 Annex A 1.2.0");
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:s-411:annex-a:ed1.2.0");
        // The base never carries an edition.
        assert_eq!(id.to_hash()["base"]["_type"], "pubid:wmo:standard");
        assert!(id.to_hash()["base"].get("edition").is_none());
    }

    #[test]
    fn the_marker_case_is_preserved_verbatim() {
        let id = WmoIdentifier::parse("S-411 ANNEX A").expect("parse");
        assert_eq!(id.annex_form.as_deref(), Some("ANNEX"));
        assert_eq!(id.to_human(), "S-411 ANNEX A");
        // The URN re-renders to the attested face.
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:s-411:annex-a");
    }

    #[test]
    fn parses_the_mrn_urn_forms() {
        for (urn, human) in [
            ("urn:mrn:wmo:pub:wmo-1364", "WMO-No. 1364"),
            (
                "urn:mrn:wmo:pub:wmo-1364:ed2025",
                "WMO-No. 1364, 2025 edition",
            ),
            ("urn:mrn:wmo:pub:s-411", "S-411"),
            ("urn:mrn:wmo:pub:s-411:ed1.2.1", "S-411 1.2.1"),
            ("urn:mrn:wmo:pub:s-411:annex-a", "S-411 Annex A"),
            ("urn:mrn:wmo:pub:s-411:annex", "S-411 Annex"),
            (
                "urn:mrn:wmo:pub:s-411:annex-a:ed1.2.0",
                "S-411 Annex A 1.2.0",
            ),
            // the widening
            ("urn:mrn:wmo:pub:wmo-td-196", "WMO/TD-No. 196"),
            ("urn:mrn:wmo:pub:wmo-td-1086", "WMO/TD-No. 1086"),
            (
                "urn:mrn:wmo:pub:jcomm-tr-14",
                "JCOMM Technical Report No. 14",
            ),
            ("urn:mrn:wmo:pub:gaw-312", "GAW Report No. 312"),
            ("urn:mrn:wmo:pub:wmo-306:vol-i.1", "WMO-No. 306, Volume I.1"),
            ("urn:mrn:wmo:pub:wmo-9:vol-d", "WMO-No. 9, Volume D"),
            (
                "urn:mrn:wmo:pub:wmo-306:vol-i.1:ed2020",
                "WMO-No. 306, Volume I.1, 2020 edition",
            ),
            (
                "urn:mrn:wmo:pub:wmo-8:ed2021-2018",
                "WMO-No. 8, 2021/2018 edition",
            ),
            (
                "urn:mrn:wmo:pub:wmo-8:status-provisional:ed2026",
                "WMO-No. 8, Provisional 2026 edition",
            ),
            (
                "urn:mrn:wmo:pub:wmo-1364:ed2023:updated-2025",
                "WMO-No. 1364, 2023 edition, updated in 2025",
            ),
        ] {
            let id = WmoIdentifier::parse(urn).expect(urn);
            assert_eq!(id.to_human(), human, "{urn}");
            // Every MRN form round-trips exactly.
            assert_eq!(
                WmoIdentifier::parse(&id.to_urn()).expect("round"),
                id,
                "{urn}"
            );
        }
    }

    #[test]
    fn the_td_slug_stays_off_the_bare_wmo_code() {
        // Longest match first: "wmo-td-" also starts with "wmo-".
        assert_eq!(
            WmoIdentifier::parse("urn:mrn:wmo:pub:wmo-td-196")
                .expect("parse")
                .series
                .as_deref(),
            Some("TD")
        );
        assert_eq!(
            WmoIdentifier::parse("urn:mrn:wmo:pub:wmo-196")
                .expect("parse")
                .series,
            None
        );
    }

    #[test]
    fn urn_segments_are_caseless() {
        let id = WmoIdentifier::parse("URN:MRN:WMO:PUB:S-411:ANNEX-A:ED1.2.0").expect("parse");
        assert_eq!(id.to_human(), "S-411 Annex A 1.2.0");
        let widened =
            WmoIdentifier::parse("URN:MRN:WMO:PUB:WMO-306:VOL-I.1:ED2020").expect("parse");
        assert_eq!(widened.to_human(), "WMO-No. 306, Volume I.1, 2020 edition");
        let status =
            WmoIdentifier::parse("urn:mrn:wmo:pub:wmo-8:STATUS-PROVISIONAL:ED2026").expect("parse");
        assert_eq!(status.to_human(), "WMO-No. 8, Provisional 2026 edition");
    }

    #[test]
    fn rejects_the_bad_urns() {
        for urn in [
            "urn:mrn:wmo:pub:x-1",                   // unknown code segment
            "urn:mrn:wmo:pub:s-411:junk",            // unknown segment
            "urn:mrn:wmo:pub:s-411:ed9",             // not dotted
            "urn:mrn:wmo:pub:wmo-1364:ed1.2.1",      // not a year
            "urn:mrn:wmo:pub:s-411:annex-a:annex-b", // duplicate annex
            "urn:mrn:wmo:pub:s-411:ed1.2:ed1.3",     // duplicate edition
            "urn:wmo:s-411",                         // not the mrn namespace
            "urn:mrn:wmo:lib:s-411",                 // not the pub sub-namespace
            "urn:mrn:wmo:pub:wmo-13x4",              // non-digit number
            // the widening rejects
            "urn:mrn:wmo:pub:wmo-td-196:vol-3", // arabic volume
            "urn:mrn:wmo:pub:wmo-td-196:vol",   // valueless volume
            "urn:mrn:wmo:pub:wmo-1364:ed2021-2018-x", // malformed pair
            "urn:mrn:wmo:pub:wmo-1364:updated-25", // not a year
            "urn:mrn:wmo:pub:wmo-td-196:vol-a:vol-b", // duplicate volume
            "urn:mrn:wmo:pub:wmo-8:status-provisional:status-final",
            "urn:mrn:wmo:pub:wmo-8:ed2026:ed2027", // duplicate edition
            "urn:mrn:wmo:pub:wmo-1364:ed2023:updated-2025:updated-2026",
            "urn:mrn:wmo:pub:td-196", // unprefixed TD slug
        ] {
            assert!(WmoIdentifier::parse(urn).is_err(), "{urn} must be rejected");
        }
    }

    #[test]
    fn rejects_the_unattested_human_shapes() {
        for input in [
            "WMO 1364",        // the series marker is not optional
            "S411",            // the S-code carries the dash
            "WMO-No. edition", // no number
            "S-411:ed",        // no edition value
            "nonsense",
        ] {
            assert!(
                WmoIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn round_trips_through_the_hash() {
        for input in [
            "WMO-No. 1364",
            "WMO-No. 1364, 2025 edition",
            "S-411",
            "S-411 1.2.1",
            "S-411 Annex A",
            "WMO-No. 306, Volume I.1, 2020 edition",
            "WMO-No. 8, 2021/2018 edition",
            "WMO-No. 1364, 2023 edition, updated in 2025",
            "WMO-No. 8, Provisional 2026 edition",
            "WMO/TD-No. 196",
            "JCOMM Technical Report No. 14",
            "GAW Report No. 312",
        ] {
            let id = WmoIdentifier::parse(input).expect("parse");
            let rebuilt = WmoIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id, "{input}");
            assert_eq!(rebuilt.to_hash(), id.to_hash());
            assert_eq!(rebuilt.to_human(), input);
            assert_eq!(rebuilt.to_urn(), id.to_urn());
        }
    }

    #[test]
    fn a_bare_annex_has_no_letter_in_the_hash() {
        let id = WmoIdentifier::parse("S-411 Annex").expect("parse");
        assert_eq!(id.letter, None);
        assert_eq!(id.to_urn(), "urn:mrn:wmo:pub:s-411:annex");
        let rebuilt = WmoIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt.to_human(), "S-411 Annex");
    }

    #[test]
    fn every_generated_urn_rebuilds_its_identifier() {
        // The widening faces over every shape that carries them: the
        // generator's form re-parses to the same identifier and the
        // same face (the reference's fixtures URN round-trip spec).
        for input in [
            "WMO-No. 8, Volume I",
            "WMO-No. 8, Volume II",
            "WMO-No. 49, Volume III",
            "WMO-No. 9, Volume D",
            "WMO-No. 306, Volume I.1",
            "WMO-No. 306, Volume I.1, 2020 edition",
            "WMO-No. 8, 2021/2018 edition",
            "WMO-No. 1364, 2023 edition, updated in 2025",
            "WMO-No. 8, Provisional 2026 edition",
            "WMO/TD-No. 196",
            "WMO/TD-No. 1086",
            "WMO/TD-No. 1215",
            "WMO/TD-No. 1466",
            "JCOMM Technical Report No. 14",
            "JCOMM Technical Report No. 24",
            "JCOMM Technical Report No. 47",
            "GAW Report No. 312",
            "GAW Report No. 285",
        ] {
            let id = WmoIdentifier::parse(input).expect(input);
            let urn = id.to_urn();
            let round = WmoIdentifier::parse(&urn).expect(&urn);
            assert_eq!(round, id, "{input}");
            assert_eq!(round.to_human(), id.to_human(), "{input}");
        }
    }
}
