//! The arxiv flavor: arXiv e-print identifiers (pubid#753,
//! relaton/relaton#42).
//!
//! The artifact binds the raw faces (`archive`, `subject_class`,
//! `yymm`, `number`, `version`) over the two shapes — the new form
//! `YYMM.NNNN[vN]` (4-digit numbers, 0704-1412) and `YYMM.NNNNN[vN]`
//! (5-digit numbers, 1501 onward), and the old form
//! `archive[.SC]/YYMMNNN[vN]` (9108-0703) — with an optional
//! case-insensitive `arXiv:` / `arXiv ` prefix. The flavor layer is
//! the gem builder's date-range cast (the ranges the shape cannot
//! express):
//!
//! - the YYMM splits into year + month; the month must be 01-12, and
//!   an old id's 91-99 gains the 1900s (a new id is always in the
//!   2000s);
//! - an old-form id dates from 9108 (1991-08) to 0703 (2007-03); a
//!   4-digit new number from 0704 to 1412; a 5-digit new number from
//!   1501 on;
//! - the version is 1 or more with no leading zero (`v01` fails, so
//!   the printed `vN` is exact).
//!
//! The human prints the canonical colon form whatever prefix the
//! input used (`arXiv:<id>`); the year and month are the paper's YYMM
//! (identity, not an edition), and the URN is the lossless pubid-scheme
//! form — arXiv registers no official URN namespace.

use serde_json::Value;

use crate::artifact::arxiv_artifact;
use crate::bound::{bound_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// The arxiv identifier: an e-print, new form or old form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArxivIdentifier {
    /// The archive of an old-form id (`hep-th`, `math`); `None` for a
    /// new id.
    pub archive: Option<String>,
    /// The subject class of an old-form id (`GT` in `math.GT/0309136`).
    pub subject_class: Option<String>,
    /// The submission year (4 digits), read from the id's YYMM.
    pub year: i64,
    /// The submission month (1-12), read from the id's YYMM.
    pub month: i64,
    /// The e-print number, zero-padded as printed (`00001`).
    pub number: String,
    /// The e-print version (`v2` → 2); `None` when the id names no
    /// version.
    pub version: Option<i64>,
}

impl ArxivIdentifier {
    /// The polymorphic type tag carried in the canonical hash.
    pub const TYPE: &'static str = "pubid:arxiv:eprint";

    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// reference builder's date-range cast over the raw captures.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = arxiv_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;

        let archive = presence(optional_bound_str(&bound, "archive"));
        let subject_class = presence(optional_bound_str(&bound, "subject_class"));
        let number = bound_str(&bound, "number")?.to_string();

        let (year, month) = year_month(bound_str(&bound, "yymm")?, archive.is_some())?;
        validate_range(year, month, &number, archive.is_some())?;

        let version = match optional_bound_str(&bound, "version") {
            None | Some("") => None,
            // The printed `vN` must be exact: a leading zero is a
            // version the input never meant (`v01`).
            Some(text) if text.starts_with('0') => {
                return Err(Error::InvalidHash {
                    detail: format!("version v{text} is not 1 or more without a leading zero"),
                });
            }
            Some(text) => Some(text.parse::<i64>().map_err(|_| Error::InvalidHash {
                detail: format!("version v{text} is not a number"),
            })?),
        };

        Ok(Self {
            archive,
            subject_class,
            year,
            month,
            number,
            version,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        crate::bound::check_type(hash, Self::TYPE)?;
        let integer = |field: &'static str| -> Result<i64> {
            hash.get(field)
                .and_then(Value::as_i64)
                .ok_or(Error::MissingAttribute { capture: field })
        };
        Ok(Self {
            archive: optional_hash_str(hash, "archive").map(str::to_string),
            subject_class: optional_hash_str(hash, "subject_class").map(str::to_string),
            year: integer("year")?,
            month: integer("month")?,
            number: crate::bound::hash_str(hash, "number")?.to_string(),
            version: hash.get("version").and_then(Value::as_i64),
        })
    }

    /// The canonical hash: `_type`, the old-form faces, then the YYMM
    /// columns and the number; an absent version never serializes.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), Self::TYPE.into());
        if let Some(archive) = &self.archive {
            object.insert("archive".into(), archive.clone().into());
        }
        if let Some(subject_class) = &self.subject_class {
            object.insert("subject_class".into(), subject_class.clone().into());
        }
        object.insert("year".into(), self.year.into());
        object.insert("month".into(), self.month.into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(version) = self.version {
            object.insert("version".into(), version.into());
        }
        Value::Object(object)
    }

    /// The canonical human: the `arXiv:` colon form whatever prefix
    /// the input used — `<YYMM>.<number>` for a new id,
    /// `<archive>[.<subject class>]/<YYMM><number>` for an old one,
    /// plus `v<version>` when stated.
    pub fn to_human(&self) -> String {
        format!("arXiv:{}", self.bare_body())
    }

    /// The bare id without the `arXiv:` prefix (the form the arXiv
    /// API takes in `id_list`).
    pub fn to_bare(&self) -> String {
        self.bare_body()
    }

    /// The body face shared by both print forms.
    fn bare_body(&self) -> String {
        let yymm = self.yymm();
        let mut body = match &self.archive {
            None => format!("{yymm}.{}", self.number),
            Some(archive) => {
                let head = match &self.subject_class {
                    Some(subject_class) => format!("{archive}.{subject_class}"),
                    None => archive.clone(),
                };
                format!("{head}/{yymm}{}", self.number)
            }
        };
        if let Some(version) = self.version {
            body.push('v');
            body.push_str(&version.to_string());
        }
        body
    }

    /// The four-digit YYMM the id prints.
    fn yymm(&self) -> String {
        format!("{:02}{:02}", self.year % 100, self.month)
    }

    /// The lossless pubid-scheme URN:
    /// `urn:pubid:arxiv:eprint:<year>:<month>:<number>
    /// [:archive-<archive>][:sc-<subject class>][:v-<version>]` — the
    /// base fallback maps different papers to one URN, so every face
    /// rides along.
    pub fn to_urn(&self) -> String {
        let mut out = format!(
            "urn:pubid:arxiv:eprint:{}:{}:{}",
            self.year, self.month, self.number
        );
        if let Some(archive) = &self.archive {
            out.push_str(&format!(":archive-{archive}"));
        }
        if let Some(subject_class) = &self.subject_class {
            out.push_str(&format!(":sc-{subject_class}"));
        }
        if let Some(version) = self.version {
            out.push_str(&format!(":v-{version}"));
        }
        out
    }
}

/// The reference's `presence`: an empty capture is an absent one.
fn presence(value: Option<&str>) -> Option<String> {
    value.filter(|text| !text.is_empty()).map(str::to_string)
}

/// The YYMM split: the month must be 01-12, and an old id's 91-99
/// gains the 1900s (a new id is always in the 2000s).
fn year_month(yymm: &str, old: bool) -> Result<(i64, i64)> {
    let digits = || -> Option<(i64, i64)> {
        if yymm.len() != 4 || !yymm.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some((yymm[..2].parse().ok()?, yymm[2..].parse().ok()?))
    };
    let (two_digit, month) = digits().ok_or_else(|| Error::InvalidHash {
        detail: format!("YYMM {yymm:?} is not four digits"),
    })?;
    if !(1..=12).contains(&month) {
        return Err(Error::InvalidHash {
            detail: format!("month out of range in YYMM {yymm}"),
        });
    }
    let century = if old && two_digit >= 91 { 1900 } else { 2000 };
    Ok((century + two_digit, month))
}

/// The date-range gate: an old-form id runs 9108-0703; a 4-digit new
/// number 0704-1412; a 5-digit new number 1501 onward.
fn validate_range(year: i64, month: i64, number: &str, old: bool) -> Result<()> {
    let ym = year * 100 + month;
    let valid = if old {
        (199108..=200703).contains(&ym)
    } else if number.len() == 4 {
        (200704..=201412).contains(&ym)
    } else {
        ym >= 201501
    };
    if valid {
        Ok(())
    } else if old {
        Err(Error::InvalidHash {
            detail: "an old-form id dates from 9108 to 0703".to_string(),
        })
    } else {
        Err(Error::InvalidHash {
            detail: format!(
                "a {}-digit number does not fit YYMM {:02}{:02}",
                number.len(),
                year % 100,
                month
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ArxivIdentifier;
    use serde_json::json;

    #[test]
    fn the_new_form_splits_the_yymm_and_keeps_the_zero_pad() {
        let id = ArxivIdentifier::parse("arXiv:2301.00001v1").expect("parse");
        assert_eq!(id.archive, None);
        assert_eq!(id.subject_class, None);
        assert_eq!(id.year, 2023);
        assert_eq!(id.month, 1);
        assert_eq!(id.number, "00001");
        assert_eq!(id.version, Some(1));
        assert_eq!(id.to_human(), "arXiv:2301.00001v1");
        assert_eq!(id.to_bare(), "2301.00001v1");
        assert_eq!(id.to_urn(), "urn:pubid:arxiv:eprint:2023:1:00001:v-1");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:arxiv:eprint",
                "year": 2023,
                "month": 1,
                "number": "00001",
                "version": 1,
            })
        );
    }

    #[test]
    fn the_four_digit_form_omits_the_version_when_absent() {
        let id = ArxivIdentifier::parse("arXiv:0704.0001").expect("parse");
        assert_eq!(id.year, 2007);
        assert_eq!(id.month, 4);
        assert_eq!(id.version, None);
        assert_eq!(id.to_human(), "arXiv:0704.0001");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:arxiv:eprint",
                "year": 2007,
                "month": 4,
                "number": "0001",
            })
        );
    }

    #[test]
    fn the_old_form_gains_the_nineteen_century_and_carries_the_archive() {
        let id = ArxivIdentifier::parse("arXiv:hep-th/9901001v2").expect("parse");
        assert_eq!(id.archive.as_deref(), Some("hep-th"));
        assert_eq!(id.subject_class, None);
        assert_eq!(id.year, 1999);
        assert_eq!(id.month, 1);
        assert_eq!(id.number, "001");
        assert_eq!(id.version, Some(2));
        assert_eq!(id.to_human(), "arXiv:hep-th/9901001v2");
        assert_eq!(
            id.to_urn(),
            "urn:pubid:arxiv:eprint:1999:1:001:archive-hep-th:v-2"
        );

        let scoped = ArxivIdentifier::parse("arXiv:math.GT/0309136").expect("parse");
        assert_eq!(scoped.archive.as_deref(), Some("math"));
        assert_eq!(scoped.subject_class.as_deref(), Some("GT"));
        assert_eq!(scoped.year, 2003);
        assert_eq!(scoped.month, 9);
        assert_eq!(scoped.to_human(), "arXiv:math.GT/0309136");
        assert_eq!(
            scoped.to_urn(),
            "urn:pubid:arxiv:eprint:2003:9:136:archive-math:sc-GT"
        );
    }

    #[test]
    fn the_prefix_spellings_all_render_the_canonical_colon_form() {
        for (input, rendered) in [
            ("arXiv 2301.00001v1", "arXiv:2301.00001v1"),
            ("arxiv:2301.00001v1", "arXiv:2301.00001v1"),
            ("ARXIV:hep-th/9901001", "arXiv:hep-th/9901001"),
            ("2301.00001v1", "arXiv:2301.00001v1"),
            ("0704.0001", "arXiv:0704.0001"),
            ("math.GT/0309136", "arXiv:math.GT/0309136"),
        ] {
            let id = ArxivIdentifier::parse(input).expect(input);
            assert_eq!(id.to_human(), rendered, "{input}");
        }
    }

    #[test]
    fn the_date_ranges_and_the_version_gate_reject() {
        for input in [
            "1412.00001",     // a 5-digit number before 1501
            "1501.0001",      // a 4-digit number from 1501
            "0703.0001",      // the new form before 0704
            "2313.00001",     // month 13
            "2300.00001",     // month 00
            "2301.00001v0",   // version 0
            "2301.00001v01",  // leading zero
            "hep-th/0801001", // the old form after 0703
            "hep-th/9013001", // month 13
            "hep-th/9107001", // the old form before 9108
            "arXiv:",
            "2301.001",
            "arXiv:2301.00001.v1",
            "arXiv:Hep-th/9901001",
        ] {
            assert!(
                ArxivIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn round_trips_through_the_hash() {
        for input in [
            "arXiv:0704.0001",
            "arXiv:2301.00001v1",
            "arXiv:hep-th/9901001v2",
            "arXiv:math.GT/0309136",
            "arXiv:cs.AI/0101001v1",
            "arXiv:astro-ph/9108001",
        ] {
            let id = ArxivIdentifier::parse(input).expect(input);
            let rebuilt = ArxivIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt, id, "{input}");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{input}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{input}");
            assert_eq!(rebuilt.to_urn(), id.to_urn(), "{input}");
        }
    }
}
