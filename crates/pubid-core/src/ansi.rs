//! The ansi flavor: ANSI standards and copublished editions.
//!
//! The artifact binds `publisher`, `number_with_part` (the whole
//! `802.3-2012` face), `copublishers[].copublisher`, and the
//! colon-form `date`; the flavor layer splits the dash-suffix like the
//! reference (relaton#45): a trailing 4-digit 19xx/20xx group is the
//! publication *year* (`ANSI 802.3-2012` carries `number: 802.3`,
//! `year: '2012'`, `date_separator: "-"`), a shorter all-digit group
//! stays the wire `part` (`ANSI/IEC 60601-1`), and the colon date
//! becomes a separator-less `year` (`ANSI/ISO 9899:1990`). The human
//! joins the authorities with slashes; the URN joins them with dashes
//! and keeps the dash-lead part segment (`urn:ansi:X9.8:-1:2007`).

use serde_json::Value;

use crate::artifact::ansi_artifact;
use crate::bound::{bound_str, check_type, optional_bound_str, optional_hash_str};
use crate::error::Result;

/// The ansi identifier (the flavor has one wire class).
#[derive(Debug, Clone, Eq)]
pub struct AnsiIdentifier {
    /// The publishing body (`ANSI`).
    pub publisher: String,
    /// The copublishing bodies, in print order (`ASME`, `IEC`, ...).
    pub copublishers: Vec<String>,
    /// The document number (`802.3`, `C135.14`).
    pub number: String,
    /// The dash-suffix part (`1` in `X9.8-1:2007`).
    pub part: Option<String>,
    /// The year (`2012` in `802.3-2012`, `1990` in `9899:1990`).
    pub year: Option<String>,
    /// How the input joined the year to the number: `-` for the
    /// `ANSI X9.62-2005` spelling, `None` for the colon of
    /// `ANSI/ISO 9899:1990`. Spelling only, so [`PartialEq`] skips it.
    pub date_separator: Option<String>,
}

impl PartialEq for AnsiIdentifier {
    /// The reference marks `date_separator` equality-ignored: the dash
    /// and the colon spelling are one document
    /// (`X9.62-2005 == X9.62:2005`).
    fn eq(&self, other: &Self) -> bool {
        self.publisher == other.publisher
            && self.copublishers == other.copublishers
            && self.number == other.number
            && self.part == other.part
            && self.year == other.year
    }
}

/// `19xx` / `20xx`: the trailing-dash year face
/// (the reference's `/\A(?:19|20)\d{2}\z/`).
fn is_dash_year(text: &str) -> bool {
    text.len() == 4
        && (text.starts_with("19") || text.starts_with("20"))
        && text.chars().all(|c| c.is_ascii_digit())
}

impl AnsiIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:ansi:standard";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and split the
    /// number face exactly like the reference's wire hash.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = ansi_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let number_with_part = bound_str(&bound, "number_with_part")?;
        // The reference splits on the first dash: the group after it is
        // the part — unless it is a 4-digit 19xx/20xx group and no
        // colon year follows, which makes it the year instead
        // ("ANSI X9.62-2005", relaton#45).
        let bound_year = optional_bound_str(&bound, "date").map(str::to_string);
        let (number, part, year, date_separator) = match number_with_part.split_once('-') {
            Some((left, right))
                if !right.is_empty() && right.chars().all(|c| c.is_ascii_digit()) =>
            {
                if bound_year.is_none() && is_dash_year(right) {
                    (left, None, Some(right.to_string()), Some("-".to_string()))
                } else {
                    (left, Some(right.to_string()), bound_year, None)
                }
            }
            _ => (number_with_part, None, bound_year, None),
        };
        let copublishers = match bound.get("copublishers") {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|item| item.get("copublisher").and_then(Value::as_str))
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        };

        Ok(Self {
            publisher: bound_str(&bound, "publisher")?.to_string(),
            copublishers,
            number: number.to_string(),
            part,
            year,
            date_separator,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record;
    /// the publisher/copublishers carry nested `body` objects).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        check_type(hash, Self::TYPE)?;
        let publisher = hash
            .get("publisher")
            .and_then(|publisher| publisher.get("body"))
            .and_then(Value::as_str)
            .ok_or(crate::error::Error::MissingAttribute {
                capture: "publisher.body",
            })?;
        let copublishers = match hash.get("copublishers") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.get("body")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .ok_or(crate::error::Error::InvalidHash {
                            detail: "copublishers entry has no body".to_string(),
                        })
                })
                .collect::<Result<Vec<_>>>()?,
            Some(other) => {
                return Err(crate::error::Error::InvalidHash {
                    detail: format!("copublishers is {other:?}"),
                })
            }
        };
        Ok(Self {
            publisher: publisher.to_string(),
            copublishers,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            part: optional_hash_str(hash, "part").map(str::to_string),
            year: optional_hash_str(hash, "year").map(str::to_string),
            date_separator: optional_hash_str(hash, "date_separator").map(str::to_string),
        })
    }

    /// The canonical hash: `_type`, number, part, year, the nested
    /// publisher/copublisher bodies, then the date separator (absent
    /// optionals omitted — the reference's attribute order).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(part) = &self.part {
            object.insert("part".into(), part.clone().into());
        }
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        object.insert(
            "publisher".into(),
            serde_json::json!({ "body": self.publisher }),
        );
        if !self.copublishers.is_empty() {
            object.insert(
                "copublishers".into(),
                Value::Array(
                    self.copublishers
                        .iter()
                        .map(|body| serde_json::json!({ "body": body }))
                        .collect(),
                ),
            );
        }
        if let Some(date_separator) = &self.date_separator {
            object.insert("date_separator".into(), date_separator.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human: authorities joined with slashes, then the
    /// number, dash-part, and the year under its input separator (the
    /// colon is the default).
    pub fn to_human(&self) -> String {
        let mut authorities = vec![self.publisher.clone()];
        authorities.extend(self.copublishers.iter().cloned());
        let mut out = authorities.join("/");
        out.push(' ');
        out.push_str(&self.number);
        if let Some(part) = &self.part {
            out.push('-');
            out.push_str(part);
        }
        if let Some(year) = &self.year {
            out.push_str(self.date_separator.as_deref().unwrap_or(":"));
            out.push_str(year);
        }
        out
    }

    /// The URN: `urn:<pub>[-<copub>]*:<number>[:-<part>][:<year>]` —
    /// the part segment keeps its dash-lead, downcased authorities.
    pub fn to_urn(&self) -> String {
        let mut authorities = vec![self.publisher.to_lowercase()];
        authorities.extend(self.copublishers.iter().map(|body| body.to_lowercase()));
        let mut out = format!("urn:{}:{}", authorities.join("-"), self.number);
        if let Some(part) = &self.part {
            out.push_str(&format!(":-{part}"));
        }
        if let Some(year) = &self.year {
            out.push_str(&format!(":{year}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::AnsiIdentifier;
    use serde_json::json;

    #[test]
    fn a_trailing_dash_year_is_the_year_not_a_part() {
        let id = AnsiIdentifier::parse("ANSI 802.3-2012").expect("parse");
        assert_eq!(id.number, "802.3");
        assert_eq!(id.part, None);
        assert_eq!(id.year.as_deref(), Some("2012"));
        assert_eq!(id.date_separator.as_deref(), Some("-"));
        assert_eq!(id.publisher, "ANSI");
        assert!(id.copublishers.is_empty());
        assert_eq!(id.to_human(), "ANSI 802.3-2012");
        // The year carries no dash-lead in the URN.
        assert_eq!(id.to_urn(), "urn:ansi:802.3:2012");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ansi:standard",
                "number": "802.3",
                "year": "2012",
                "publisher": {"body": "ANSI"},
                "date_separator": "-",
            })
        );
    }

    #[test]
    fn copublishers_join_with_slashes_in_the_human_and_dashes_in_the_urn() {
        let id = AnsiIdentifier::parse("ANSI/IEC 60601-1").expect("parse");
        assert_eq!(id.copublishers, vec!["IEC".to_string()]);
        // A short dash group stays a part.
        assert_eq!(id.part.as_deref(), Some("1"));
        assert_eq!(id.to_human(), "ANSI/IEC 60601-1");
        assert_eq!(id.to_urn(), "urn:ansi-iec:60601:-1");
    }

    #[test]
    fn the_colon_date_becomes_the_year() {
        let id = AnsiIdentifier::parse("ANSI/ISO 9899:1990").expect("parse");
        assert_eq!(id.year.as_deref(), Some("1990"));
        assert!(id.part.is_none());
        assert_eq!(id.date_separator, None);
        assert_eq!(id.to_human(), "ANSI/ISO 9899:1990");
        assert_eq!(id.to_urn(), "urn:ansi-iso:9899:1990");
        // nil means the colon: the hash carries no separator.
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:ansi:standard",
                "number": "9899",
                "year": "1990",
                "publisher": {"body": "ANSI"},
                "copublishers": [{"body": "ISO"}],
            })
        );
    }

    #[test]
    fn a_true_part_stays_a_part_before_a_colon_year() {
        let id = AnsiIdentifier::parse("ANSI X9.8-1:2007").expect("parse");
        assert_eq!(id.number, "X9.8");
        assert_eq!(id.part.as_deref(), Some("1"));
        assert_eq!(id.year.as_deref(), Some("2007"));
        assert_eq!(id.date_separator, None);
        assert_eq!(id.to_human(), "ANSI X9.8-1:2007");
        assert_eq!(id.to_urn(), "urn:ansi:X9.8:-1:2007");
    }

    #[test]
    fn a_non_year_dash_group_stays_a_part() {
        let id = AnsiIdentifier::parse("ANSI C63.4-1").expect("parse");
        assert_eq!(id.part.as_deref(), Some("1"));
        assert_eq!(id.year, None);
    }

    #[test]
    fn the_dash_and_the_colon_spelling_are_one_document() {
        let dashed = AnsiIdentifier::parse("ANSI X9.62-2005").expect("parse");
        let colon = AnsiIdentifier::parse("ANSI X9.62:2005").expect("parse");
        assert_eq!(dashed, colon);
        // The spelling still lives in the hash (and only there).
        assert!(dashed.to_hash() != colon.to_hash());
    }

    #[test]
    fn an_open_copublisher_list_parses_any_uppercase_acronym() {
        for (input, copublishers) in [
            ("ANSI/UL 94-2023", vec!["UL"]),
            ("ANSI/NISO Z39.88-2004", vec!["NISO"]),
            ("ANSI/INCITS 359-2004", vec!["INCITS"]),
            ("ANSI/ASHRAE/IES 90.1-2019", vec!["ASHRAE", "IES"]),
            ("ANSI/ASC X9.95-2022", vec!["ASC"]),
            ("ANSI/AAMI ST79:2017", vec!["AAMI"]),
        ] {
            let id = AnsiIdentifier::parse(input).expect(input);
            assert_eq!(
                id.copublishers,
                copublishers
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<_>>(),
                "{input}"
            );
            assert_eq!(id.to_human(), input);
        }
    }

    #[test]
    fn a_lowercase_copublisher_is_rejected() {
        assert!(AnsiIdentifier::parse("ANSI/ul 94-2023").is_err());
    }

    #[test]
    fn an_undated_number_stays_whole() {
        let id = AnsiIdentifier::parse("ANSI/ASME B16.5").expect("parse");
        assert_eq!(id.number, "B16.5");
        assert_eq!(id.to_human(), "ANSI/ASME B16.5");
        assert_eq!(id.to_urn(), "urn:ansi-asme:B16.5");
    }

    #[test]
    fn round_trips_through_the_hash() {
        for input in ["ANSI C135.14-1979", "ANSI X9.62-2005", "ANSI X9.8-1:2007"] {
            let id = AnsiIdentifier::parse(input).expect("parse");
            let rebuilt = AnsiIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id);
            assert_eq!(rebuilt.to_hash(), id.to_hash());
            assert_eq!(rebuilt.to_human(), input);
        }
    }
}
