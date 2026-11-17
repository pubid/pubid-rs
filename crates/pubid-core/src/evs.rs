//! The evs flavor: the Estonian national family — plain standards
//! (`EVS 811:2018`), the TR/TS types, the direct ISO/IEC adoptions
//! (`EVS-IEC 62368-1:2020`), and the national adoptions over the
//! embedded CEN grammar (`EVS-EN 16798-1:2019`), plus the EVS-list
//! supplements (the `+NA` national annex, the `COR`/`AC` corrigendum,
//! the `AMD` amendment spelling), the `prEVS` draft prefix, and the
//! trailing language code.
//!
//! The artifact embeds the CEN identifier rules under the `adopted`
//! subtree (which builds through [`CenIdentifier::build`]); the ISO/
//! IEC and plain shapes carry their captures directly. The trailing
//! language code extracts before the parse (the CEN adopted spine is
//! greedy over plain text and would swallow it), and the space-
//! separated ` AC` corrigendum spelling binds through the EVS-specific
//! rule (the CEN spine consumes `+AC`). The separator face (`-` vs
//! ` `) preserves the printed form; the default dash stays off the
//! wire.

use serde_json::Value;

use crate::artifact::evs_artifact;
use crate::bound::{optional_at, optional_hash_str};
use crate::cen_cenelec::CenIdentifier;
use crate::error::Result;

/// What concrete evs class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvsKind {
    EstonianStandard,
    InternationalAdoption,
    NationalAdoption,
}

use EvsKind::*;

impl EvsKind {
    /// The shared `_type` prefix every evs tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:evs:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            EstonianStandard => "pubid:evs:estonian-standard",
            InternationalAdoption => "pubid:evs:international-adoption",
            NationalAdoption => "pubid:evs:national-adoption",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [EstonianStandard, InternationalAdoption, NationalAdoption]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }
}

/// The evs identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvsIdentifier {
    /// The concrete class (drives `_type` and the body shape).
    pub kind: EvsKind,
    /// The CEN adoption (national only).
    pub base: Option<CenIdentifier>,
    /// The printed separator (`-` default, `EVS 811` the space form).
    pub separator: Option<String>,
    /// The `prEVS` draft prefix.
    pub draft_prefix: bool,
    /// The trailing language code (`et-en`, `et`, `en`).
    pub language: Option<String>,
    /// The document number (the direct adoption's, the plain
    /// standard's).
    pub number: Option<String>,
    /// The direct adoption's part.
    pub part: Option<String>,
    /// The direct adoption's origin organization (`IEC`, `ISO`).
    pub intl_org: Option<String>,
    /// The year.
    pub year: Option<String>,
    /// The plain standard's type word (`TR`, `TS`).
    pub evs_type: Option<String>,
    /// The national annex: separator (`+`/`/`) and optional year.
    pub na_separator: Option<String>,
    pub na_year: Option<String>,
    /// The AMD amendment spelling.
    pub amd_separator: Option<String>,
    pub amd_kind: Option<String>,
    pub amd_number: Option<String>,
    pub amd_year: Option<String>,
    /// The COR/AC corrigendum spelling.
    pub cor_separator: Option<String>,
    pub cor_kind: Option<String>,
    pub cor_number: Option<String>,
    pub cor_year: Option<String>,
}

/// The trailing language tail (` (et-en|et|en)$`).
fn language_tail(input: &str) -> Option<(&str, &str)> {
    for code in ["et-en", "et", "en"] {
        let tail = format!(" {code}");
        if let Some(body) = input.strip_suffix(&tail) {
            return Some((body, code));
        }
    }
    None
}

impl EvsIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// reference's builder cast over the bound captures.
    pub fn parse(input: &str) -> Result<Self> {
        let (body, language) = match language_tail(input) {
            Some((body, code)) => (body, Some(code.to_string())),
            None => (input, None),
        };
        let artifact = evs_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, body)?;
        let mut identifier = Self::build(&bound)?;
        identifier.language = language;
        Ok(identifier)
    }

    /// The builder cast over a bound hash.
    pub fn build(bound: &Value) -> Result<Self> {
        let draft_prefix = optional_at(bound, "draft_prefix") == Some("pr");
        let separator = optional_at(bound, "evs_separator").map(str::to_string);

        let mut identifier = if let Some(adopted) = bound.get("adopted") {
            // The adopted TS face carries no TS marker ("EVS-EN TS
            // 16798-1:2019"): the TS-ness of an EN adoption is
            // metadata, not identity — strip the type capture before
            // the CEN builder routes on it.
            let cen_tree = if optional_at(adopted, "type") == Some("TS")
                && optional_at(adopted, "publisher") == Some("EN")
            {
                let mut stripped = adopted.clone();
                if let Some(object) = stripped.as_object_mut() {
                    object.remove("type");
                }
                stripped
            } else {
                adopted.clone()
            };
            Self {
                base: Some(CenIdentifier::build(&cen_tree)?),
                separator,
                draft_prefix,
                ..empty(NationalAdoption)
            }
        } else if let Some(intl) = bound.get("intl") {
            let read = |key: &'static str| optional_at(intl, key).map(str::to_string);
            Self {
                number: read("number"),
                part: read("part"),
                intl_org: read("intl_org"),
                year: read("year"),
                draft_prefix,
                ..empty(InternationalAdoption)
            }
        } else {
            let plain = bound.get("plain").cloned().unwrap_or(Value::Null);
            let read = |key: &'static str| optional_at(&plain, key).map(str::to_string);
            Self {
                number: read("number"),
                year: read("year"),
                evs_type: read("evs_type"),
                separator,
                draft_prefix,
                ..empty(EstonianStandard)
            }
        };

        if let Some(na) = bound.get("na") {
            identifier.na_separator = Some(supplement_separator(na).unwrap_or("+").to_string());
            identifier.na_year = optional_at(na, "na_year").map(str::to_string);
        }
        if let Some(amd) = bound.get("evs_amd") {
            identifier.amd_separator =
                Some(supplement_separator(amd).unwrap_or("+").to_string());
            identifier.amd_kind = Some("AMD".to_string());
            identifier.amd_number = optional_at(amd, "amd_number").map(str::to_string);
            identifier.amd_year = optional_at(amd, "amd_year").map(str::to_string);
        }
        if let Some(cor) = bound.get("evs_cor") {
            identifier.cor_separator =
                Some(supplement_separator(cor).unwrap_or("+").to_string());
            identifier.cor_kind = Some(
                optional_at(cor, "cor_kind").unwrap_or("COR").to_string(),
            );
            identifier.cor_number = optional_at(cor, "cor_number").map(str::to_string);
            identifier.cor_year = optional_at(cor, "cor_year").map(str::to_string);
        }
        Ok(identifier)
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(EvsKind::from_type_tag)
            .ok_or_else(|| crate::error::Error::InvalidHash {
                detail: format!("unknown evs _type {:?}", hash.get("_type")),
            })?;
        let read = |field: &'static str| optional_hash_str(hash, field).map(str::to_string);
        let base = match hash.get("base").cloned().filter(|base| !base.is_null()) {
            Some(base) => Some(CenIdentifier::from_hash(&base)?),
            None => None,
        };
        Ok(Self {
            kind,
            base,
            separator: read("separator"),
            draft_prefix: hash.get("draft_prefix") == Some(&Value::Bool(true)),
            language: read("language"),
            number: read("number"),
            part: read("part"),
            intl_org: read("intl_org"),
            year: read("year"),
            evs_type: read("evs_type"),
            na_separator: read("na_separator"),
            na_year: read("na_year"),
            amd_separator: read("amd_separator"),
            amd_kind: read("amd_kind"),
            amd_number: read("amd_number"),
            amd_year: read("amd_year"),
            cor_separator: read("cor_separator"),
            cor_kind: read("cor_kind"),
            cor_number: read("cor_number"),
            cor_year: read("cor_year"),
        })
    }

    /// The canonical hash: `_type` first, then the present attributes.
    /// The default dash separator stays off the wire; the draft prefix
    /// only carries when true.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        if let Some(base) = &self.base {
            object.insert("base".into(), base.to_hash());
        }
        if let Some(number) = &self.number {
            object.insert("number".into(), number.clone().into());
        }
        if let Some(part) = &self.part {
            object.insert("part".into(), part.clone().into());
        }
        if let Some(intl_org) = &self.intl_org {
            object.insert("intl_org".into(), intl_org.clone().into());
        }
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        if let Some(evs_type) = &self.evs_type {
            object.insert("evs_type".into(), evs_type.clone().into());
        }
        if let Some(separator) = &self.separator {
            if separator != "-" {
                object.insert("separator".into(), separator.clone().into());
            }
        }
        if let Some(language) = &self.language {
            object.insert("language".into(), language.clone().into());
        }
        for (key, value) in [
            ("na_separator", &self.na_separator),
            ("na_year", &self.na_year),
            ("amd_separator", &self.amd_separator),
            ("amd_kind", &self.amd_kind),
            ("amd_number", &self.amd_number),
            ("amd_year", &self.amd_year),
            ("cor_separator", &self.cor_separator),
            ("cor_kind", &self.cor_kind),
            ("cor_number", &self.cor_number),
            ("cor_year", &self.cor_year),
        ] {
            if let Some(value) = value {
                object.insert(key.into(), value.clone().into());
            }
        }
        if self.draft_prefix {
            object.insert("draft_prefix".into(), Value::Bool(true));
        }
        Value::Object(object)
    }

    /// The canonical human: `pr?EVS<separator>body` plus the
    /// supplements and the language tail.
    pub fn to_human(&self) -> String {
        let mut result = String::new();
        if self.draft_prefix {
            result.push_str("pr");
        }
        result.push_str("EVS");
        result.push_str(self.separator.as_deref().unwrap_or("-"));
        result.push_str(&self.body());
        if let Some(separator) = &self.amd_separator {
            result.push_str(separator);
            result.push_str("AMD");
            result.push_str(self.amd_number.as_deref().unwrap_or(""));
            if let Some(year) = &self.amd_year {
                result.push(':');
                result.push_str(year);
            }
        }
        if let Some(separator) = &self.cor_separator {
            result.push_str(separator);
            result.push_str(self.cor_kind.as_deref().unwrap_or("COR"));
            result.push_str(self.cor_number.as_deref().unwrap_or(""));
            if let Some(year) = &self.cor_year {
                result.push(':');
                result.push_str(year);
            }
        }
        if let Some(separator) = &self.na_separator {
            result.push_str(separator);
            result.push_str("NA");
            if let Some(year) = &self.na_year {
                result.push(':');
                result.push_str(year);
            }
        }
        if let Some(language) = &self.language {
            result.push(' ');
            result.push_str(language);
        }
        result
    }

    /// The URN: `urn:evs:` over the shape's segments, then the
    /// supplements and the language.
    pub fn to_urn(&self) -> String {
        let mut segments: Vec<String> = Vec::new();
        if self.draft_prefix {
            segments.push("pr".to_string());
        }
        if let Some(base) = &self.base {
            let adopted_urn = base.to_urn();
            const CEN_NAMESPACE: &str = "urn:cen:";
            if let Some(rest) = adopted_urn.strip_prefix(CEN_NAMESPACE) {
                segments.extend(rest.split(':').map(str::to_string));
            } else {
                segments.push(adopted_urn);
            }
        } else if self.kind == InternationalAdoption {
            segments.push(self.intl_org.as_deref().unwrap_or("").to_lowercase());
            let number = self.number.clone().unwrap_or_default();
            segments.push(match &self.part {
                Some(part) => format!("{number}-{part}"),
                None => number,
            });
            if let Some(year) = &self.year {
                segments.push(year.clone());
            }
        } else {
            if let Some(evs_type) = &self.evs_type {
                segments.push(evs_type.to_lowercase());
            }
            segments.push(self.number.clone().unwrap_or_default());
            if let Some(year) = &self.year {
                segments.push(year.clone());
            }
        }
        if self.na_separator.is_some() {
            segments.push("na".to_string());
            if let Some(year) = &self.na_year {
                segments.push(year.clone());
            }
        }
        if self.amd_separator.is_some() {
            if self.amd_separator.as_deref() == Some("+") {
                segments.push("plus".to_string());
            }
            segments.push("amd".to_string());
            if let Some(number) = &self.amd_number {
                segments.push(number.clone());
            }
            if let Some(year) = &self.amd_year {
                segments.push(year.clone());
            }
        }
        if self.cor_separator.is_some() {
            if self.cor_separator.as_deref() == Some("+") {
                segments.push("plus".to_string());
            }
            segments.push("cor".to_string());
            if let Some(number) = &self.cor_number {
                segments.push(number.clone());
            }
            if let Some(year) = &self.cor_year {
                segments.push(year.clone());
            }
        }
        if let Some(language) = &self.language {
            segments.push(language.clone());
        }
        format!("urn:evs:{}", segments.join(":"))
    }

    /// The shape's body between the separator and the supplements.
    fn body(&self) -> String {
        match self.kind {
            NationalAdoption => self
                .base
                .as_ref()
                .map(CenIdentifier::to_human)
                .unwrap_or_default(),
            InternationalAdoption => {
                let mut body = format!(
                    "{} {}",
                    self.intl_org.as_deref().unwrap_or(""),
                    self.number.as_deref().unwrap_or("")
                );
                if let Some(part) = &self.part {
                    body.push('-');
                    body.push_str(part);
                }
                if let Some(year) = &self.year {
                    body.push(':');
                    body.push_str(year);
                }
                body
            }
            EstonianStandard => {
                let mut body = String::new();
                if let Some(evs_type) = &self.evs_type {
                    body.push_str(evs_type);
                    body.push(' ');
                }
                body.push_str(self.number.as_deref().unwrap_or(""));
                if let Some(year) = &self.year {
                    body.push(':');
                    body.push_str(year);
                }
                body
            }
        }
    }
}

/// The EVS-list supplement separator capture (`supp_plus` / `supp_slash`
/// / `supp_space`) mapped to its printed face.
fn supplement_separator(node: &Value) -> Option<&str> {
    for (key, face) in [
        ("supp_plus", "+"),
        ("supp_slash", "/"),
        ("supp_space", " "),
    ] {
        if node.get(key).is_some() {
            return Some(face);
        }
    }
    None
}

fn empty(kind: EvsKind) -> EvsIdentifier {
    EvsIdentifier {
        kind,
        base: None,
        separator: None,
        draft_prefix: false,
        language: None,
        number: None,
        part: None,
        intl_org: None,
        year: None,
        evs_type: None,
        na_separator: None,
        na_year: None,
        amd_separator: None,
        amd_kind: None,
        amd_number: None,
        amd_year: None,
        cor_separator: None,
        cor_kind: None,
        cor_number: None,
        cor_year: None,
    }
}

#[cfg(test)]
mod tests {
    use super::EvsIdentifier;
    use serde_json::json;

    #[test]
    fn the_plain_standard_parses_and_keeps_the_separator_off_the_wire() {
        let id = EvsIdentifier::parse("EVS 906:2026").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:evs:estonian-standard",
            "number": "906",
            "year": "2026",
        }));
        assert_eq!(id.to_human(), "EVS 906:2026");
        assert_eq!(id.to_urn(), "urn:evs:906:2026");

        let spaced = EvsIdentifier::parse("EVS 811:2018 et-en").expect("parse");
        assert_eq!(spaced.to_hash(), json!({
            "_type": "pubid:evs:estonian-standard",
            "number": "811",
            "separator": " ",
            "language": "et-en",
            "year": "2018",
        }));
        assert_eq!(spaced.to_human(), "EVS 811:2018 et-en");
        assert_eq!(spaced.to_urn(), "urn:evs:811:2018:et-en");
    }

    #[test]
    fn the_tr_and_ts_types_use_their_faces() {
        let tr = EvsIdentifier::parse("EVS-TR 1:2019").expect("parse");
        assert_eq!(tr.to_hash(), json!({
            "_type": "pubid:evs:estonian-standard",
            "number": "1",
            "year": "2019",
            "evs_type": "TR",
        }));
        assert_eq!(tr.to_human(), "EVS-TR 1:2019");
        assert_eq!(tr.to_urn(), "urn:evs:tr:1:2019");
    }

    #[test]
    fn the_international_adoption_carries_the_org() {
        let id = EvsIdentifier::parse("EVS-IEC 62368-1:2020").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:evs:international-adoption",
            "number": "62368",
            "part": "1",
            "intl_org": "IEC",
            "year": "2020",
        }));
        assert_eq!(id.to_human(), "EVS-IEC 62368-1:2020");
        assert_eq!(id.to_urn(), "urn:evs:iec:62368-1:2020");
    }

    #[test]
    fn the_national_adoption_builds_the_cen_base() {
        let id = EvsIdentifier::parse("EVS-EN 16798-1:2019").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:evs:national-adoption",
            "base": {
                "_type": "pubid:cencenelec:european-norm",
                "number": "16798",
                "part": "1",
                "year": "2019",
            },
        }));
        assert_eq!(id.to_human(), "EVS-EN 16798-1:2019");
        assert_eq!(id.to_urn(), "urn:evs:en:16798-1:2019");

        // The TS alias is metadata: the same document, no TS marker.
        let alias = EvsIdentifier::parse("EVS-EN TS 16798-1:2019").expect("parse");
        assert_eq!(alias.to_hash(), id.to_hash());
    }

    #[test]
    fn the_supplements_render_and_chain_the_urn() {
        let na = EvsIdentifier::parse("EVS-EN 1993-1-1:2005+NA").expect("parse");
        assert_eq!(na.to_hash()["na_separator"], json!("+"));
        assert_eq!(na.to_human(), "EVS-EN 1993-1-1:2005+NA");
        assert_eq!(na.to_urn(), "urn:evs:en:1993-1-1:2005:na");

        let cor = EvsIdentifier::parse("EVS-EN 50581:2012 AC:2015").expect("parse");
        assert_eq!(cor.to_human(), "EVS-EN 50581:2012 AC:2015");
        assert_eq!(cor.to_urn(), "urn:evs:en:50581:2012:cor:2015");

        let amd = EvsIdentifier::parse("EVS-EN 71-1:2011 AMD1:2014").expect("parse");
        assert_eq!(amd.to_hash()["amd_separator"], json!(" "));
        assert_eq!(amd.to_human(), "EVS-EN 71-1:2011 AMD1:2014");
        assert_eq!(amd.to_urn(), "urn:evs:en:71-1:2011:amd:1:2014");
    }

    #[test]
    fn the_consolidated_base_slices_the_cen_urn() {
        let id = EvsIdentifier::parse("EVS-EN 50581:2012+COR:2015").expect("parse");
        assert_eq!(id.to_human(), "EVS-EN 50581:2012+COR:2015");
        assert_eq!(id.to_urn(), "urn:evs:en:50581:2012:plus:cor:2015");
    }

    #[test]
    fn the_iso_iec_copublisher_face_renders_and_urns() {
        let id = EvsIdentifier::parse("EVS-EN ISO/IEC 27017:2026").expect("parse");
        assert_eq!(id.to_human(), "EVS-EN ISO/IEC 27017:2026");
        assert_eq!(id.to_urn(), "urn:evs:en:iso-iec:27017:2026");
    }

    #[test]
    fn the_draft_prefix_leads_every_face() {
        let id = EvsIdentifier::parse("prEVS-EN 100:2026").expect("parse");
        assert_eq!(id.to_hash()["draft_prefix"], json!(true));
        assert_eq!(id.to_human(), "prEVS-EN 100:2026");
        assert_eq!(id.to_urn(), "urn:evs:pr:en:100:2026");
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "EVS 811:2018 et-en",
            "EVS-TR 1:2019",
            "EVS-EN 16798-1:2019",
            "EVS-EN 1993-1-1:2005+NA:2010 et",
            "prEVS-EN 100:2026",
        ] {
            let id = EvsIdentifier::parse(spelling).expect(spelling);
            let rebuilt = EvsIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{spelling}");
        }
    }
}
