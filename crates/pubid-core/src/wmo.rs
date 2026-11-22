//! The wmo flavor: the WMO-No. publication series and the
//! WMO-published S-100 product specifications (pubid#763).
//!
//! Two series share the flavor, per the IALA-precedent ruling (the
//! S-100 specs live under the publishing org's flavor — the lexical
//! shape is the IHO S-100 form, but WMO is the assigning authority):
//!
//!   WMO-No. 1364                       the WMO-No. publication series
//!   WMO-No. 1364, 2025 edition         calendar-year edition (comma face)
//!   WMO-No. 1364 2025 edition          cover-stack alias
//!   WMO-No. 1364:ed2025                compact alias
//!   S-411                              the S-100 product specifications
//!   S-411 1.2.1                        dotted x.y.z edition
//!   S-411:ed1.2.1                      compact alias
//!   S-411 Annex A                      annex wrapper (the DCEG face)
//!   S-411 ANNEX A                      parse-only uppercase alias
//!
//! The edition is ONE attribute carrying a type-driven face: the
//! calendar year on a publication, the dotted x.y.z on a standard or
//! annex. The MRN URNs follow the IALA pattern (`urn:mrn:wmo:pub:...`;
//! the namespace's rule is the fleet's proposed extension), and the
//! flavor is the first ported one whose aliases include URN spellings,
//! so `parse` dispatches on the `urn:` prefix like the reference's
//! `FormatDetector`.

use serde_json::Value;

use crate::artifact::wmo_artifact;
use crate::bound::{bound_str, check_type, hash_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete WMO class an identifier belongs to
/// (`Pubid::Wmo::Identifiers::*` in the reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoKind {
    /// The WMO-No. publication series (`WMO-No. 1364`).
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
    /// The edition, with the face keyed off the kind: the calendar
    /// year on a publication (`"2025"`), the dotted x.y.z on a
    /// standard or annex (`"1.2.1"`).
    pub edition: Option<String>,
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
    /// dispatch the class off the resolved captures: `wmomarker` marks
    /// a publication, `annex_marker` an annex, the bare `s_code` a
    /// standard (the grammar's ordered choice).
    fn parse_human(input: &str) -> Result<Self> {
        let artifact = wmo_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Self::from_bound(&bound)
    }

    /// Build from the artifact's flat bound map.
    fn from_bound(bound: &Value) -> Result<Self> {
        let number = bound_str(bound, "doc_number")?.to_string();
        let edition = optional_bound_str(bound, "edition_value").map(str::to_string);
        if optional_bound_str(bound, "wmomarker").is_some() {
            Ok(Self {
                kind: WmoKind::Publication,
                number,
                edition,
                annex_form: None,
                letter: None,
            })
        } else if let Some(annex_form) = optional_bound_str(bound, "annex_marker") {
            Ok(Self {
                kind: WmoKind::Annex,
                number,
                edition,
                annex_form: Some(annex_form.to_string()),
                letter: optional_bound_str(bound, "annex_letter").map(str::to_string),
            })
        } else {
            Ok(Self {
                kind: WmoKind::Standard,
                number,
                edition,
                annex_form: None,
                letter: None,
            })
        }
    }

    /// Parse an MRN URN (`Pubid::Wmo::UrnParser`): the namespace
    /// prefix, one code segment (`wmo-<number>` / `s-<number>`),
    /// then at most one annex segment and one edition segment. The
    /// parser validates the segments strictly and rebuilds the human
    /// form, which re-parses through the artifact — the reference's
    /// exact shape. Segment matching is caseless in the MRN; the
    /// rebuild picks the attested faces (`Annex`, the compact edition
    /// alias), whose renders are canonical.
    fn parse_urn(urn: &str) -> Result<Self> {
        // The reference checks the prefix case-insensitively and
        // parses the body; every extracted value is digits or a letter
        // that gets upcased, so the lowercased body is equivalent.
        let lowered = urn.to_lowercase();
        let body = lowered
            .strip_prefix(Self::MRN_PREFIX)
            .ok_or_else(|| urn_error(format!("invalid WMO URN: {urn:?}")))?;
        let parts: Vec<&str> = body.split(':').collect();
        let code = parts[0];
        let segments = &parts[1..];

        if let Some(number) = code.strip_prefix("wmo-") {
            Self::urn_publication(number, code, segments)
        } else if let Some(number) = code.strip_prefix("s-") {
            Self::urn_standard(number, code, segments)
        } else {
            Err(urn_error(format!("invalid WMO URN code segment: {code:?}")))
        }
    }

    /// The publication path: the single `ed<year>` segment, rebuilt as
    /// the compact year form (the text grammar normalizes it to the
    /// comma face).
    fn urn_publication(number: &str, code: &str, segments: &[&str]) -> Result<Self> {
        if !is_digits(number) {
            return Err(urn_error(format!("invalid WMO URN code segment: {code:?}")));
        }
        let mut edition: Option<String> = None;
        for segment in segments {
            if edition.is_some() {
                return Err(urn_error(format!(
                    "invalid WMO URN segment (duplicate edition): {segment:?}"
                )));
            }
            edition = Some(Self::urn_edition(segment, true)?);
        }
        let mut text = format!("WMO-No. {number}");
        if let Some(edition) = &edition {
            text.push_str(&format!(":ed{edition}"));
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
                        return Err(urn_error(format!(
                            "invalid WMO URN segment (duplicate annex): {segment:?}"
                        )));
                    }
                    has_annex = true;
                    letter = annex_letter;
                }
                None => {
                    if edition.is_some() {
                        return Err(urn_error(format!("invalid WMO URN segment: {segment:?}")));
                    }
                    edition = Some(Self::urn_edition(segment, false)?);
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

    /// The value of an `ed<value>` segment, validated against the
    /// shape (a year for a publication, dotted for a standard or
    /// annex) — the reference's per-segment strictness. The segment is
    /// already lowercased by [`WmoIdentifier::parse_urn`].
    fn urn_edition(segment: &str, year: bool) -> Result<String> {
        let value = segment
            .strip_prefix("ed")
            .ok_or_else(|| urn_error(format!("invalid WMO URN segment: {segment:?}")))?;
        let valid = if year {
            value.len() == 4 && is_digits(value)
        } else {
            is_dotted(value)
        };
        if valid {
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
                edition: optional_hash_str(hash, "edition").map(str::to_string),
                annex_form: Some(hash_str(hash, "annex_form")?.to_string()),
                letter: optional_hash_str(hash, "letter").map(str::to_string),
            })
        } else {
            Ok(Self {
                kind,
                number: hash_str(hash, "number")?.to_string(),
                edition: optional_hash_str(hash, "edition").map(str::to_string),
                annex_form: None,
                letter: None,
            })
        }
    }

    /// The canonical hash. The publisher default (`WMO`) never
    /// serializes — `WMO-No.` and the S-number are designations, not
    /// publisher tokens. The annex nests the base specification and
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
        }
        if let Some(edition) = &self.edition {
            object.insert("edition".into(), edition.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human (the reference's `Renderer`; the faces
    /// carry no publisher token). The annex marker keeps its input
    /// case verbatim.
    pub fn to_human(&self) -> String {
        let mut out = match self.kind {
            WmoKind::Publication => format!("WMO-No. {}", self.number),
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
                WmoKind::Publication => out.push_str(&format!(", {edition} edition")),
                _ => {
                    out.push(' ');
                    out.push_str(edition);
                }
            }
        }
        out
    }

    /// The MRN URN (`Pubid::Wmo::UrnGenerator`): the WMO-No. series
    /// codes as `wmo-<number>`, the S-100 specs keep their lexical
    /// form lowercased (`s-<number>`), an annex adds the `annex` or
    /// `annex-<letter>` segment, and the edition rides as `ed<value>`.
    pub fn to_urn(&self) -> String {
        let mut out = match self.kind {
            WmoKind::Publication => format!("{}wmo-{}", Self::MRN_PREFIX, self.number),
            _ => format!("{}s-{}", Self::MRN_PREFIX, self.number),
        };
        if self.kind == WmoKind::Annex {
            match &self.letter {
                Some(letter) => out.push_str(&format!(":annex-{}", letter.to_lowercase())),
                None => out.push_str(":annex"),
            }
        }
        if let Some(edition) = &self.edition {
            out.push_str(&format!(":ed{edition}"));
        }
        out
    }
}

/// All-ASCII-digit, non-empty.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// The dotted x.y[.z] shape (`\A\d+(\.\d+)+\z`): at least two
/// non-empty all-digit groups.
fn is_dotted(text: &str) -> bool {
    let groups: Vec<&str> = text.split('.').collect();
    groups.len() >= 2 && groups.iter().all(|group| is_digits(group))
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
    fn urn_segments_are_caseless() {
        let id = WmoIdentifier::parse("URN:MRN:WMO:PUB:S-411:ANNEX-A:ED1.2.0").expect("parse");
        assert_eq!(id.to_human(), "S-411 Annex A 1.2.0");
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
        ] {
            let id = WmoIdentifier::parse(input).expect("parse");
            let rebuilt = WmoIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id, "{input}");
            assert_eq!(rebuilt.to_hash(), id.to_hash());
            assert_eq!(rebuilt.to_human(), input);
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
}
