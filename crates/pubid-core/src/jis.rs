//! The jis flavor: the Japanese Industrial Standards (`JIS A 0001:1999`,
//! the TR/TS types, the `(E)`/`(J)` language faces, the `:1997R`
//! reaffirmations) and the WES welding-engineering standards that ride
//! the same flavor (`WES 7700-1:2019`, the `WES TR/TS` types, the
//! language-before-year form `WES 1112(E):2022`), plus the amendment /
//! corrigendum / explanation supplements.
//!
//! The artifact binds the flat captures (`series`, `number`,
//! `parts[]`, `year`, `reaffirmed`, `language`, `all_parts`,
//! `wes`, `wes_type`, `wes_language`, and the supplement's
//! `amd_*`/`corr_*`/`expl_*` faces). The builder dispatches on the
//! supplement captures first, then the WES marker; a WES id carries
//! one language, not two (`wes_language` is the before-year form,
//! recorded as `language_before_year`, which the hash carries and the
//! render honours). Years are INTEGERs on the wire — a document's
//! number stays a string (the leading zeros are the identity), a
//! supplement's number/year cast to integers. The URN keeps the
//! document code with its space (`urn:jis:A 0001:1999`) and appends
//! the supplement notation lowercased (`amd 1:2025`).

use serde_json::Value;

use crate::artifact::jis_artifact;
use crate::bound::{capture_in, optional_at, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete jis class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JisKind {
    JapaneseIndustrialStandard,
    TechnicalReport,
    TechnicalSpecification,
    Amendment,
    Corrigendum,
    Explanation,
    WeldingEngineeringStandard,
    WeldingEngineeringTechnicalReport,
    WeldingEngineeringTechnicalSpecification,
}

use JisKind::*;

impl JisKind {
    /// The shared `_type` prefix every jis tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:jis:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            JapaneseIndustrialStandard => "pubid:jis:japanese-industrial-standard",
            TechnicalReport => "pubid:jis:technical-report",
            TechnicalSpecification => "pubid:jis:technical-specification",
            Amendment => "pubid:jis:amendment",
            Corrigendum => "pubid:jis:corrigendum",
            Explanation => "pubid:jis:explanation",
            WeldingEngineeringStandard => "pubid:jis:welding-engineering-standard",
            WeldingEngineeringTechnicalReport => "pubid:jis:welding-engineering-technical-report",
            WeldingEngineeringTechnicalSpecification => {
                "pubid:jis:welding-engineering-technical-specification"
            }
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [
            JapaneseIndustrialStandard,
            TechnicalReport,
            TechnicalSpecification,
            Amendment,
            Corrigendum,
            Explanation,
            WeldingEngineeringStandard,
            WeldingEngineeringTechnicalReport,
            WeldingEngineeringTechnicalSpecification,
        ]
        .into_iter()
        .find(|kind| kind.type_tag() == tag)
    }

    fn is_supplement(self) -> bool {
        matches!(self, Amendment | Corrigendum | Explanation)
    }
}

/// The jis identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JisIdentifier {
    /// The concrete class (drives `_type`, the code root, and the
    /// supplement notation).
    pub kind: JisKind,
    /// The series letter (`A` in `JIS A 0001`); WES ids carry none.
    pub series: Option<String>,
    /// The document number (`0001`; a supplement's ordinal).
    pub number: String,
    /// The dash parts (`1`, `2` in `C 61000-3-2`).
    pub parts: Vec<String>,
    /// The year (INTEGER on the wire).
    pub year: Option<i64>,
    /// The language face (`E`, `J`).
    pub language: Option<String>,
    /// A language printed before the year (`WES 1112(E):2022`).
    pub language_before_year: bool,
    /// The (規格群) all-parts marker.
    pub all_parts: bool,
    /// The `R` reaffirmation.
    pub reaffirmed: bool,
    /// The SYMBOL clause value (`""` when the clause carries no value).
    pub symbol: Option<String>,
    /// A supplement's underlying document.
    pub base: Option<Box<JisIdentifier>>,
}

impl JisIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// reference's builder cast over the bound captures.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = jis_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        let mut identifier = Self::build(&bound)?;
        // The SYMBOL clause attaches to the outermost identifier.
        if optional_at(&bound, "symbol_present").is_some() {
            identifier.symbol = Some(optional_at(&bound, "symbol_value").unwrap_or("").to_string());
        }
        Ok(identifier)
    }

    /// The builder cast over a bound hash.
    pub fn build(bound: &Value) -> Result<Self> {
        if optional_at(bound, "amd_number").is_some() {
            return build_supplement(bound, Amendment, "amd");
        }
        if optional_at(bound, "corr_number").is_some() {
            return build_supplement(bound, Corrigendum, "corr");
        }
        if optional_at(bound, "expl_type").is_some() {
            return build_explanation(bound);
        }
        build_single(bound)
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(JisKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown jis _type {:?}", hash.get("_type")),
            })?;
        let base = match hash.get("base").cloned().filter(|base| !base.is_null()) {
            Some(base) => Some(Box::new(JisIdentifier::from_hash(&base)?)),
            None => None,
        };
        // A document's number is a string face (the leading zeros are
        // the identity); a supplement's is an INTEGER ordinal.
        let number = match hash.get("number") {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Number(value)) => value.to_string(),
            _ => return Err(Error::MissingAttribute { capture: "number" }),
        };
        Ok(Self {
            kind,
            series: optional_hash_str(hash, "series").map(str::to_string),
            number,
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
            year: hash
                .get("year")
                .and_then(Value::as_i64),
            language: optional_hash_str(hash, "language").map(str::to_string),
            language_before_year: hash.get("language_before_year") == Some(&Value::Bool(true)),
            all_parts: hash.get("all_parts") == Some(&Value::Bool(true)),
            reaffirmed: hash.get("reaffirmed") == Some(&Value::Bool(true)),
            symbol: optional_hash_str(hash, "symbol").map(str::to_string),
            base,
        })
    }

    /// The canonical hash: `_type` first; a document carries its code
    /// faces, a supplement its ordinal year and base.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        if self.kind.is_supplement() {
            object.insert("number".into(), Value::Number(self.number.parse::<i64>().unwrap_or(0).into()));
            if let Some(year) = self.year {
                object.insert("year".into(), Value::Number(year.into()));
            }
            if self.reaffirmed {
                object.insert("reaffirmed".into(), Value::Bool(true));
            }
        } else {
            if let Some(series) = &self.series {
                object.insert("series".into(), series.clone().into());
            }
            object.insert("number".into(), self.number.clone().into());
            if !self.parts.is_empty() {
                object.insert(
                    "parts".into(),
                    Value::Array(
                        self.parts
                            .iter()
                            .map(|part| part.clone().into())
                            .collect(),
                    ),
                );
            }
            if let Some(year) = self.year {
                object.insert("year".into(), Value::Number(year.into()));
            }
            if let Some(language) = &self.language {
                object.insert("language".into(), language.clone().into());
            }
            if self.language_before_year {
                object.insert("language_before_year".into(), Value::Bool(true));
            }
            if self.all_parts {
                object.insert("all_parts".into(), Value::Bool(true));
            }
            if self.reaffirmed {
                object.insert("reaffirmed".into(), Value::Bool(true));
            }
            if let Some(symbol) = &self.symbol {
                object.insert("symbol".into(), symbol.clone().into());
            }
        }
        if let Some(base) = &self.base {
            object.insert("base".into(), base.to_hash());
        }
        Value::Object(object)
    }

    /// The canonical human.
    pub fn to_human(&self) -> String {
        if self.kind.is_supplement() {
            let base = self.base.as_deref().map(JisIdentifier::to_human).unwrap_or_default();
            let result = format!("{base}/{}", self.supplement_notation());
            return result + &self.symbol_suffix();
        }
        let mut result = match self.kind {
            WeldingEngineeringStandard | WeldingEngineeringTechnicalReport
            | WeldingEngineeringTechnicalSpecification => self.code().unwrap_or_default(),
            _ => {
                let prefix = match self.kind {
                    TechnicalReport => "JIS TR",
                    TechnicalSpecification => "JIS TS",
                    _ => "JIS",
                };
                format!("{prefix} {}", self.code().unwrap_or_default())
            }
        };
        result += &self.year_and_language();
        if self.all_parts {
            result.push_str("（規格群）");
        }
        result + &self.symbol_suffix()
    }

    /// The URN: `urn:jis:<code>[:year][:lang]["all"][:notation]`.
    pub fn to_urn(&self) -> String {
        let mut parts = vec!["urn".to_string(), "jis".to_string()];
        if let Some(code) = self.code() {
            parts.push(code);
        }
        if let Some(year) = self.year {
            parts.push(year.to_string());
        }
        if let Some(language) = &self.language {
            parts.push(language.to_lowercase());
        }
        if self.all_parts {
            parts.push("all".to_string());
        }
        if self.base.is_some() {
            let notation = self.supplement_notation();
            if !notation.is_empty() {
                parts.push(notation.to_lowercase());
            }
        }
        parts.join(":")
    }

    /// The rendered document code (`B 0205-1`, `WES TR 2032`).
    fn code(&self) -> Option<String> {
        if let Some(base) = &self.base {
            return base.code();
        }
        match self.kind {
            WeldingEngineeringStandard | WeldingEngineeringTechnicalReport
            | WeldingEngineeringTechnicalSpecification => {
                let root = match self.kind {
                    WeldingEngineeringTechnicalReport => "WES TR",
                    WeldingEngineeringTechnicalSpecification => "WES TS",
                    _ => "WES",
                };
                let mut code = format!("{root} {}", self.number);
                for part in &self.parts {
                    code.push('-');
                    code.push_str(part);
                }
                Some(code)
            }
            _ => {
                let series = self.series.as_deref()?;
                let mut code = format!("{series} {}", self.number);
                for part in &self.parts {
                    code.push('-');
                    code.push_str(part);
                }
                Some(code)
            }
        }
    }

    /// The `:year[R]` and `(lang)` tail, in the order the source used.
    fn year_and_language(&self) -> String {
        let mut year_part = String::new();
        if let Some(year) = self.year {
            year_part = format!(":{year}{}", if self.reaffirmed { "R" } else { "" });
        }
        let lang_part = self
            .language
            .as_deref()
            .map(|language| format!("({language})"))
            .unwrap_or_default();
        if self.language_before_year {
            format!("{lang_part}{year_part}")
        } else {
            format!("{year_part}{lang_part}")
        }
    }

    /// The SYMBOL clause suffix.
    fn symbol_suffix(&self) -> String {
        match &self.symbol {
            None => String::new(),
            Some(value) if value.is_empty() => " SYMBOL".to_string(),
            Some(value) => format!(" SYMBOL {value}"),
        }
    }

    /// The supplement notation (`AMD 1:2025`, `CORRIGENDUM 1:2025`,
    /// `EXPL [n]`).
    fn supplement_notation(&self) -> String {
        let reaffirmed = if self.reaffirmed { "R" } else { "" };
        match self.kind {
            Amendment => Some(format!("AMD {}:{}{}", self.number, year_or_empty(self.year), reaffirmed)),
            Corrigendum => Some(format!(
                "CORRIGENDUM {}:{}{}",
                self.number,
                year_or_empty(self.year),
                reaffirmed
            )),
            Explanation => Some(match self.number.as_str() {
                "" => "EXPL".to_string(),
                number => format!("EXPL {number}"),
            }),
            _ => None,
        }
        .unwrap_or_default()
    }
}

fn year_or_empty(year: Option<i64>) -> String {
    year.map(|year| year.to_string()).unwrap_or_default()
}

/// The single-document cast: WES marker or JIS code.
fn build_single(bound: &Value) -> Result<JisIdentifier> {
    let read = |key: &'static str| optional_at(bound, key).map(str::to_string);
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
    let year = read("year").and_then(|year| year.parse::<i64>().ok());
    if optional_at(bound, "wes").is_some() {
        let kind = match read("wes_type").as_deref() {
            Some("TR") => WeldingEngineeringTechnicalReport,
            Some("TS") => WeldingEngineeringTechnicalSpecification,
            _ => WeldingEngineeringStandard,
        };
        // A WES id carries one language, not two: `wes_language` is
        // the before-year form.
        let (language, language_before_year) = match (
            read("wes_language"),
            read("language"),
        ) {
            (Some(wes_language), Some(_)) => {
                return Err(Error::InvalidHash {
                    detail: "JIS: a WES id carries one language, not two".to_string(),
                });
            }
            (Some(wes_language), None) => (Some(wes_language), true),
            (None, language) => (language, false),
        };
        return Ok(JisIdentifier {
            kind,
            series: None,
            number: read("number").unwrap_or_default(),
            parts,
            year,
            language,
            language_before_year,
            all_parts: optional_at(bound, "all_parts").is_some(),
            reaffirmed: optional_at(bound, "reaffirmed").is_some(),
            symbol: None,
            base: None,
        });
    }
    let kind = match read("type").as_deref() {
        Some("TR") => TechnicalReport,
        Some("TS") => TechnicalSpecification,
        _ => JapaneseIndustrialStandard,
    };
    Ok(JisIdentifier {
        kind,
        series: read("series"),
        number: read("number").ok_or(Error::MissingAttribute { capture: "number" })?,
        parts,
        year,
        language: read("language"),
        language_before_year: false,
        all_parts: optional_at(bound, "all_parts").is_some(),
        reaffirmed: optional_at(bound, "reaffirmed").is_some(),
        symbol: None,
        base: None,
    })
}

/// The supplement cast: the base document, then the ordinal year and
/// the `R` reaffirmation.
fn build_supplement(bound: &Value, kind: JisKind, prefix: &'static str) -> Result<JisIdentifier> {
    let mut base_data = bound.clone();
    if let Some(object) = base_data.as_object_mut() {
        for key in [
            "amd_type", "amd_number", "amd_year", "amd_reaffirmed",
            "corr_type", "corr_number", "corr_year", "corr_reaffirmed",
            "expl_type", "expl_number",
        ] {
            object.remove(key);
        }
    }
    let base = build_single(&base_data)?;
    let number_key: &'static str = match prefix {
        "amd" => "amd_number",
        _ => "corr_number",
    };
    let year_key: &'static str = match prefix {
        "amd" => "amd_year",
        _ => "corr_year",
    };
    let reaffirmed_key: &'static str = match prefix {
        "amd" => "amd_reaffirmed",
        _ => "corr_reaffirmed",
    };
    Ok(JisIdentifier {
        kind,
        number: optional_at(bound, number_key)
            .ok_or(Error::MissingAttribute { capture: number_key })?
            .to_string(),
        year: optional_at(bound, year_key).and_then(|year| year.parse::<i64>().ok()),
        reaffirmed: optional_at(bound, reaffirmed_key).is_some(),
        base: Some(Box::new(base)),
        series: None,
        parts: Vec::new(),
        language: None,
        language_before_year: false,
        all_parts: false,
        symbol: None,
    })
}

/// The explanation cast: the base document, then the optional ordinal
/// (the base's year rides along).
fn build_explanation(bound: &Value) -> Result<JisIdentifier> {
    let mut base_data = bound.clone();
    if let Some(object) = base_data.as_object_mut() {
        for key in [
            "amd_type", "amd_number", "amd_year", "amd_reaffirmed",
            "corr_type", "corr_number", "corr_year", "corr_reaffirmed",
            "expl_type", "expl_number",
        ] {
            object.remove(key);
        }
    }
    let base = build_single(&base_data)?;
    Ok(JisIdentifier {
        kind: Explanation,
        number: optional_at(bound, "expl_number").unwrap_or("").to_string(),
        year: base.year,
        base: Some(Box::new(base)),
        series: None,
        parts: Vec::new(),
        language: None,
        language_before_year: false,
        all_parts: false,
        reaffirmed: false,
        symbol: None,
    })
}

#[cfg(test)]
mod tests {
    use super::JisIdentifier;
    use serde_json::json;

    #[test]
    fn the_industrial_standard_keeps_leading_zeros() {
        let id = JisIdentifier::parse("JIS A 0001:1999").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:jis:japanese-industrial-standard",
            "series": "A",
            "number": "0001",
            "year": 1999,
        }));
        assert_eq!(id.to_human(), "JIS A 0001:1999");
        assert_eq!(id.to_urn(), "urn:jis:A 0001:1999");
    }

    #[test]
    fn the_parts_dash_onto_the_code() {
        let id = JisIdentifier::parse("JIS C 61000-3-2:2005").expect("parse");
        assert_eq!(id.parts, vec!["3", "2"]);
        assert_eq!(id.to_human(), "JIS C 61000-3-2:2005");
        assert_eq!(id.to_urn(), "urn:jis:C 61000-3-2:2005");
    }

    #[test]
    fn the_reaffirmed_r_rides_the_year() {
        let id = JisIdentifier::parse("JIS X 0208:1997R").expect("parse");
        assert_eq!(id.to_hash()["reaffirmed"], json!(true));
        assert_eq!(id.to_human(), "JIS X 0208:1997R");
        assert_eq!(id.to_urn(), "urn:jis:X 0208:1997");
    }

    #[test]
    fn the_wes_standard_rides_the_flavor() {
        let id = JisIdentifier::parse("WES 7700-1:2019").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:jis:welding-engineering-standard",
            "number": "7700",
            "parts": ["1"],
            "year": 2019,
        }));
        assert_eq!(id.to_human(), "WES 7700-1:2019");
        assert_eq!(id.to_urn(), "urn:jis:WES 7700-1:2019");

        let plain = JisIdentifier::parse("WES 7301").expect("parse");
        assert_eq!(plain.to_urn(), "urn:jis:WES 7301");
    }

    #[test]
    fn the_wes_language_is_the_before_year_form() {
        let id = JisIdentifier::parse("WES 1112(E):2022").expect("parse");
        assert_eq!(id.to_hash(), json!({
            "_type": "pubid:jis:welding-engineering-standard",
            "number": "1112",
            "year": 2022,
            "language": "E",
            "language_before_year": true,
        }));
        assert_eq!(id.to_human(), "WES 1112(E):2022");
        assert_eq!(id.to_urn(), "urn:jis:WES 1112:2022:e");
    }

    #[test]
    fn the_wes_types_and_the_amendment_nest() {
        let tr = JisIdentifier::parse("WES TR 2032:2024").expect("parse");
        assert_eq!(tr.to_hash()["_type"], json!("pubid:jis:welding-engineering-technical-report"));
        assert_eq!(tr.to_human(), "WES TR 2032:2024");

        let amended = JisIdentifier::parse("WES 3003(E):1995/AMD 1:2025").expect("parse");
        assert_eq!(amended.to_hash(), json!({
            "_type": "pubid:jis:amendment",
            "number": 1,
            "year": 2025,
            "base": {
                "_type": "pubid:jis:welding-engineering-standard",
                "number": "3003",
                "year": 1995,
                "language": "E",
                "language_before_year": true,
            },
        }));
        assert_eq!(amended.to_human(), "WES 3003(E):1995/AMD 1:2025");
        assert_eq!(amended.to_urn(), "urn:jis:WES 3003:2025:amd 1:2025");
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "JIS A 0001:1999",
            "JIS C 61000-3-2:2005",
            "WES 1112(E):2022",
            "WES 7301:2020R",
            "WES TS 2816(E):2015",
        ] {
            let id = JisIdentifier::parse(spelling).expect(spelling);
            let rebuilt = JisIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{spelling}");
        }
    }
}
