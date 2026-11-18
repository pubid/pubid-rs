//! The eu flavor: EU legal acts — regulations, directives, decisions
//! (pubid#752, relaton/relaton#44).
//!
//! The artifact binds the four input faces: the canonical pair
//! (`prefix`/`type_word`/`first`/`second`), the Official Journal long
//! form (`family`, `no`, `family_suffix`), the CELEX number
//! (`celex_letter`/`year`/`number`) and the ELI URL (`eli_type`). The
//! flavor layer is the gem builder's cast over them:
//!
//! - CELEX and ELI fix the order (year and number by position); the
//!   canonical and long forms give two numbers and the order is not in
//!   the grammar — [`number_first_for`] reads the pair by the print
//!   rule (only one plausible year: that one; both: a first value from
//!   2015 is the year, otherwise a regulation prints
//!   `<number>/<year>` and the other acts `<year>/<number>`; neither:
//!   the parse fails). A `No` marks `<number>/<year>` outright.
//! - the year is identity (`1227/2011` and `1227/2012` are two acts),
//!   so it must pass the plausibility gate (4 digits, 1952-2099 — the
//!   first Official Journal acts, the ECSC ones, date from 1952);
//! - the number drops the CELEX zero padding (`0679` → `679`) and
//!   rejects the bare `0`;
//! - the legal family (`EU`/`EC`/`EEC`/`Euratom`/`CFSP`) comes from the
//!   bracket or the suffix; both spellings together must agree
//!   (`.../EC` after `(EU)` is a parse failure). The canonical hash
//!   carries it, but the reference's `===` treats it as a wildcard
//!   (spelling, not identity).
//!
//! The renderer prints `EU <Type> <a>/<b>` in the reference's
//! `number_first?` order; when the plain pair would read back as
//! another act (two acts share one plain print: `32015R2014` and
//! `32014R2015` both give `2015/2014`), it prints
//! `No <number>/<year>` instead — `No` always marks
//! `<number>/<year>`, so every print parses back to its own act.

use serde_json::Value;

use crate::artifact::eu_artifact;
use crate::bound::{bound_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete EU act class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EuKind {
    /// An EU regulation (CELEX letter `R`, ELI segment `reg`).
    Regulation,
    /// An EU directive (CELEX letter `L`, ELI segment `dir`).
    Directive,
    /// An EU decision (CELEX letter `D`, ELI segment `dec`).
    Decision,
}

use EuKind::*;

impl EuKind {
    /// The shared `_type` prefix every eu tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:eu:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            Regulation => "pubid:eu:regulation",
            Directive => "pubid:eu:directive",
            Decision => "pubid:eu:decision",
        }
    }

    /// The spelled act type (`Regulation` / `Directive` / `Decision`),
    /// the word the canonical human prints.
    fn title(self) -> &'static str {
        match self {
            Regulation => "Regulation",
            Directive => "Directive",
            Decision => "Decision",
        }
    }

    /// The ELI segment (`reg` / `dir` / `dec`), the URN slot.
    fn eli(self) -> &'static str {
        match self {
            Regulation => "reg",
            Directive => "dir",
            Decision => "dec",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [Regulation, Directive, Decision]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }

    /// The `type_word` capture (`Regulation` / `Directive` / `Decision`).
    fn from_type_word(word: &str) -> Option<Self> {
        match word {
            "Regulation" => Some(Regulation),
            "Directive" => Some(Directive),
            "Decision" => Some(Decision),
            _ => None,
        }
    }

    /// The CELEX type letter (`R` / `L` / `D`).
    fn from_celex_letter(letter: &str) -> Option<Self> {
        match letter {
            "R" => Some(Regulation),
            "L" => Some(Directive),
            "D" => Some(Decision),
            _ => None,
        }
    }

    /// The ELI type segment (`reg` / `dir` / `dec`).
    fn from_eli_type(eli: &str) -> Option<Self> {
        match eli {
            "reg" => Some(Regulation),
            "dir" => Some(Directive),
            "dec" => Some(Decision),
            _ => None,
        }
    }
}

/// The eu identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EuIdentifier {
    /// The concrete act class (drives `_type`, the human title, and
    /// the CELEX/ELI slots).
    pub kind: EuKind,
    /// The act number (the CELEX zero padding stripped: `0679` → `679`).
    pub number: String,
    /// The act year — identity, not an edition.
    pub year: Option<String>,
    /// The legal family from the long form (`EU`, `EC`, `Euratom`,
    /// `EEC`, `CFSP`); the canonical reference omits it.
    pub family: Option<String>,
    /// The print order of the two numbers, kept only when the input
    /// used the other order than the render rule (`None` applies the
    /// default rule). Spelling, not identity.
    pub number_first: Option<bool>,
}

impl EuIdentifier {
    /// The eu publisher, carried as the reference's constant.
    pub const PUBLISHER: &'static str = "EU";

    /// From 2015 every act prints `<year>/<number>`; before 2015 a
    /// regulation printed `<number>/<year>`.
    pub const FIXED_ORDER_YEAR: i64 = 2015;

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// reference builder's cast over the raw captures.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = eu_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        // CELEX and ELI fix the order; the canonical and long forms
        // give the two numbers and the pair rule reads them.
        let (kind, year, raw_number, pair_order) =
            if let Some(letter) = optional_bound_str(&bound, "celex_letter") {
                let kind = EuKind::from_celex_letter(letter).ok_or_else(|| Error::InvalidHash {
                    detail: format!("unknown CELEX letter {letter:?}"),
                })?;
                (
                    kind,
                    bound_str(&bound, "year")?.to_string(),
                    bound_str(&bound, "number")?.to_string(),
                    None,
                )
            } else if let Some(eli) = optional_bound_str(&bound, "eli_type") {
                let kind = EuKind::from_eli_type(eli).ok_or_else(|| Error::InvalidHash {
                    detail: format!("unknown ELI type {eli:?}"),
                })?;
                (
                    kind,
                    bound_str(&bound, "year")?.to_string(),
                    bound_str(&bound, "number")?.to_string(),
                    None,
                )
            } else {
                let kind =
                    EuKind::from_type_word(bound_str(&bound, "type_word")?).ok_or_else(|| {
                        Error::InvalidHash {
                            detail: "unknown EU act type word".to_string(),
                        }
                    })?;
                let first = bound_str(&bound, "first")?;
                let second = bound_str(&bound, "second")?;
                // "No" marks <number>/<year>; otherwise the pair rule
                // reads the order, and no plausible year fails the parse.
                let number_first = match optional_bound_str(&bound, "no") {
                    Some(_) => true,
                    None => {
                        number_first_for(kind, first, second).ok_or_else(|| Error::InvalidHash {
                            detail: format!("no year in {first}/{second}"),
                        })?
                    }
                };
                let (year, number) = if number_first {
                    (second, first)
                } else {
                    (first, second)
                };
                (
                    kind,
                    year.to_string(),
                    number.to_string(),
                    Some(number_first),
                )
            };

        // The year is identity: an implausible one is a parse failure
        // (`93` and `0000` name no act).
        if !is_plausible_year(&year) {
            return Err(Error::InvalidHash {
                detail: format!("{year} is not a year"),
            });
        }
        let number = strip_zeros(&raw_number);
        if number == "0" {
            return Err(Error::InvalidHash {
                detail: "number 0".to_string(),
            });
        }

        // The bracketed and the suffixed family must agree.
        let bracket = optional_bound_str(&bound, "family");
        let suffix = optional_bound_str(&bound, "family_suffix");
        let family = match (bracket, suffix) {
            (Some(found), Some(other)) if found != other => {
                return Err(Error::InvalidHash {
                    detail: format!("two families, {found} and {other}"),
                });
            }
            (Some(found), _) => Some(found.to_string()),
            (None, Some(other)) => Some(other.to_string()),
            (None, None) => None,
        };

        // Keep the input order only when the render rule prints the
        // other one (the reference's keep_order).
        let number_first = match pair_order {
            Some(order) if order != default_number_first(kind, &year) => Some(order),
            _ => None,
        };

        Ok(Self {
            kind,
            number,
            year: Some(year),
            family,
            number_first,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(EuKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown eu _type {:?}", hash.get("_type")),
            })?;
        let number_first = match hash.get("number_first") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(flag)) => Some(*flag),
            Some(other) => {
                return Err(Error::InvalidHash {
                    detail: format!("eu number_first is {other:?}"),
                })
            }
        };
        Ok(Self {
            kind,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            year: optional_hash_str(hash, "year").map(str::to_string),
            family: optional_hash_str(hash, "family").map(str::to_string),
            number_first,
        })
    }

    /// The reference's `number_first?`: the stored flag, else the
    /// default print rule.
    fn number_first_effective(&self) -> bool {
        self.number_first
            .unwrap_or_else(|| default_number_first(self.kind, self.year.as_deref().unwrap_or("")))
    }

    /// The canonical hash: `_type`, number, year, then the spelling
    /// faces (the family; the order flag only when the input kept it).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(year) = &self.year {
            object.insert("year".into(), year.clone().into());
        }
        if let Some(family) = &self.family {
            object.insert("family".into(), family.clone().into());
        }
        if let Some(number_first) = self.number_first {
            object.insert("number_first".into(), Value::Bool(number_first));
        }
        Value::Object(object)
    }

    /// The canonical human: `EU <Type> <a>/<b>` in the reference's
    /// print order, or `No <number>/<year>` when the plain pair would
    /// read back as another act.
    pub fn to_human(&self) -> String {
        let number_first = self.number_first_effective();
        match &self.year {
            None => format!("{} {} {}", Self::PUBLISHER, self.kind.title(), self.number),
            Some(year) => {
                let pair = if number_first {
                    (self.number.as_str(), year.as_str())
                } else {
                    (year.as_str(), self.number.as_str())
                };
                if reads_back(self.kind, pair, number_first) {
                    format!(
                        "{} {} {}/{}",
                        Self::PUBLISHER,
                        self.kind.title(),
                        pair.0,
                        pair.1
                    )
                } else {
                    format!(
                        "{} {} No {}/{}",
                        Self::PUBLISHER,
                        self.kind.title(),
                        self.number,
                        year
                    )
                }
            }
        }
    }

    /// The CELEX number: sector 3, year, type letter, number padded
    /// to 4 (`32011R1227`).
    pub fn to_celex(&self) -> String {
        let celex_letter = match self.kind {
            Regulation => "R",
            Directive => "L",
            Decision => "D",
        };
        format!(
            "3{}{}{:0>4}",
            self.year.as_deref().unwrap_or(""),
            celex_letter,
            self.number
        )
    }

    /// The ELI of the act as published in the Official Journal.
    pub fn to_eli(&self) -> String {
        format!(
            "http://data.europa.eu/eli/{}/{}/{}/oj",
            self.kind.eli(),
            self.year.as_deref().unwrap_or(""),
            self.number
        )
    }

    /// The URN: `urn:eu:<eli>:<year>:<number>` — the act type stays in
    /// the slot, so a regulation and a decision with the same numbers
    /// do not collide.
    pub fn to_urn(&self) -> String {
        format!(
            "urn:eu:{}:{}:{}",
            self.kind.eli(),
            self.year.as_deref().unwrap_or(""),
            self.number
        )
    }
}

/// The default print rule: from 2015 every act prints
/// `<year>/<number>`; before 2015 a regulation printed
/// `<number>/<year>`, the other acts `<year>/<number>`.
fn default_number_first(kind: EuKind, year: &str) -> bool {
    kind == Regulation
        && year
            .parse::<i64>()
            .map(|value| value < EuIdentifier::FIXED_ORDER_YEAR)
            .unwrap_or(false)
}

/// The year gate: 4 digits in 1952-2099 (the first Official Journal
/// acts, the ECSC ones, date from 1952).
fn is_plausible_year(value: &str) -> bool {
    value.len() == 4
        && value.chars().all(|c| c.is_ascii_digit())
        && value
            .parse::<i64>()
            .map(|year| (1952..=2099).contains(&year))
            .unwrap_or(false)
}

/// Which of two printed numbers is the year, when the input has no
/// `No`: only one plausible year — that one; both — a first value from
/// 2015 is the year, otherwise a regulation prints `<number>/<year>`
/// and other acts `<year>/<number>`; neither — `None` (no year at all).
fn number_first_for(kind: EuKind, first: &str, second: &str) -> Option<bool> {
    let first_year = is_plausible_year(first);
    let second_year = is_plausible_year(second);
    if !first_year && !second_year {
        return None;
    }
    if first_year != second_year {
        return Some(!first_year);
    }
    if first.parse::<i64>().unwrap_or(0) >= EuIdentifier::FIXED_ORDER_YEAR {
        return Some(false);
    }
    Some(kind == Regulation)
}

/// Whether a plain print of `pair` in `number_first` order reads back
/// as this act (the renderer's reads_back?).
fn reads_back(kind: EuKind, pair: (&str, &str), number_first: bool) -> bool {
    number_first_for(kind, pair.0, pair.1) == Some(number_first)
}

/// The CELEX zero padding strip (`0679` → `679`; `0000` → `0`, which
/// the caller rejects).
fn strip_zeros(number: &str) -> String {
    let stripped = number.trim_start_matches('0');
    if stripped.is_empty() {
        "0".to_string()
    } else {
        stripped.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{EuIdentifier, EuKind};
    use serde_json::json;

    #[test]
    fn the_canonical_form_carries_the_act_and_the_year_is_identity() {
        let id = EuIdentifier::parse("EU Regulation 1227/2011").expect("parse");
        assert_eq!(id.kind, EuKind::Regulation);
        assert_eq!(id.number, "1227");
        assert_eq!(id.year.as_deref(), Some("2011"));
        assert_eq!(id.family, None);
        assert_eq!(id.number_first, None);
        assert_eq!(id.to_human(), "EU Regulation 1227/2011");
        assert_eq!(id.to_urn(), "urn:eu:reg:2011:1227");
        assert_eq!(id.to_celex(), "32011R1227");
        assert_eq!(id.to_eli(), "http://data.europa.eu/eli/reg/2011/1227/oj");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:eu:regulation",
                "number": "1227",
                "year": "2011",
            })
        );
    }

    #[test]
    fn the_long_form_carries_the_family_and_fixes_the_order_with_no() {
        let id = EuIdentifier::parse("Regulation (EU) No 1227/2011").expect("parse");
        assert_eq!(id.family.as_deref(), Some("EU"));
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:eu:regulation",
                "number": "1227",
                "year": "2011",
                "family": "EU",
            })
        );

        let decision = EuIdentifier::parse("Decision No 1082/2013/EU").expect("parse");
        assert_eq!(decision.kind, EuKind::Decision);
        assert_eq!(decision.family.as_deref(), Some("EU"));
        assert_eq!(decision.number_first, Some(true));
        assert_eq!(decision.to_human(), "EU Decision 1082/2013");
    }

    #[test]
    fn celex_and_eli_fix_the_order_and_drop_the_zero_padding() {
        let celex = EuIdentifier::parse("32016L0680").expect("parse");
        assert_eq!(celex.kind, EuKind::Directive);
        assert_eq!(celex.number, "680");
        assert_eq!(celex.year.as_deref(), Some("2016"));
        assert_eq!(celex.to_human(), "EU Directive 2016/680");

        let eli = EuIdentifier::parse("https://data.europa.eu/eli/dir/2016/680/oj").expect("parse");
        assert_eq!(eli.to_human(), "EU Directive 2016/680");

        let spaced = EuIdentifier::parse("CELEX: 32011R1227").expect("parse");
        assert_eq!(spaced.to_human(), "EU Regulation 1227/2011");
    }

    #[test]
    fn the_pair_rule_reads_one_plausible_year_as_the_year() {
        let directive = EuIdentifier::parse("EU Directive 2014/24").expect("parse");
        assert_eq!(directive.number, "24");
        assert_eq!(directive.year.as_deref(), Some("2014"));

        let decision = EuIdentifier::parse("EU Decision 2010/87").expect("parse");
        assert_eq!(decision.number, "87");
        assert_eq!(decision.year.as_deref(), Some("2010"));

        let decision = EuIdentifier::parse("EU Decision 1082/2013").expect("parse");
        assert_eq!(decision.number, "1082");
        assert_eq!(decision.year.as_deref(), Some("2013"));
        assert_eq!(decision.number_first, Some(true));
    }

    #[test]
    fn a_pre_2015_regulation_prints_number_first() {
        let id = EuIdentifier::parse("EU Regulation 2003/2003").expect("parse");
        assert_eq!(id.to_human(), "EU Regulation 2003/2003");
        assert_eq!(id.number_first, None);

        // The pair prints the year first (post-2015 rule) and stays
        // number-first only through the stored flag.
        let early = EuIdentifier::parse("EU Regulation 2010/1").expect("parse");
        assert_eq!(early.number, "1");
        assert_eq!(early.year.as_deref(), Some("2010"));
        assert_eq!(early.number_first, Some(false));
        assert_eq!(early.to_human(), "EU Regulation 2010/1");
    }

    #[test]
    fn two_acts_share_one_plain_print_so_the_loser_prints_no() {
        let later = EuIdentifier::parse("32015R2014").expect("parse");
        assert_eq!(later.to_human(), "EU Regulation 2015/2014");

        let earlier = EuIdentifier::parse("32014R2015").expect("parse");
        assert_eq!(earlier.number, "2015");
        assert_eq!(earlier.year.as_deref(), Some("2014"));
        assert_eq!(earlier.to_human(), "EU Regulation No 2015/2014");

        // Every print parses back to its own act.
        for print in [later.to_human(), earlier.to_human()] {
            let back = EuIdentifier::parse(&print).expect(print.as_str());
            assert_eq!(back.to_human(), print);
        }
    }

    #[test]
    fn the_no_form_fixes_the_number_first_order() {
        let id = EuIdentifier::parse("EU Regulation No 2015/2014").expect("parse");
        assert_eq!(id.number, "2015");
        assert_eq!(id.year.as_deref(), Some("2014"));
        // The input order matches the pre-2015 default, so the flag
        // does not serialize.
        assert_eq!(id.number_first, None);
        assert_eq!(id.to_human(), "EU Regulation No 2015/2014");

        let flagged = EuIdentifier::parse("EU Decision No 2012/2010").expect("parse");
        assert_eq!(flagged.number, "2012");
        assert_eq!(flagged.year.as_deref(), Some("2010"));
        assert_eq!(flagged.number_first, Some(true));
        assert_eq!(flagged.to_human(), "EU Decision No 2012/2010");
    }

    #[test]
    fn the_family_conflict_and_the_impossible_years_fail() {
        for input in [
            "Regulation (Euratom) No 1493/93",
            "EU Regulation 1227",
            "EU Regulation 12/34",
            "Directive 95/46/EC",
            "Regulation (EU) No 1227/2011/EC",
            "30000R0000",
            "31900R0001",
            "32011R0000",
            "http://data.europa.eu/eli/reg/0000/0",
        ] {
            assert!(
                EuIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn round_trips_through_the_hash() {
        for input in [
            "EU Regulation 1227/2011",
            "EU Decision 1082/2013",
            "Regulation (EC) No 2003/2003",
            "32016L0680",
            "32014R2015",
            "EU Regulation 2010/1",
            "Directive 2010/75/EU",
        ] {
            let id = EuIdentifier::parse(input).expect(input);
            let rebuilt = EuIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id, "{input}");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{input}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{input}");
        }
    }

    #[test]
    fn a_yearless_hash_prints_the_number_only() {
        let id = EuIdentifier::from_hash(&json!({
            "_type": "pubid:eu:regulation",
            "number": "1227",
        }))
        .expect("from_hash");
        assert_eq!(id.to_human(), "EU Regulation 1227");
    }
}
