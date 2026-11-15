//! The conformance checker: parse → render → compare over the
//! pubid-testsuite corpus.
//!
//! Mirrors `Pubid::Conformance::Runner` (Ruby) and pubid-ts
//! `src/conformance/`: one corpus case yields a mismatch list (empty
//! means passing — control flow never depends on exception mechanics),
//! review/quarantined rows are counted and skipped, error rows must be
//! rejected, and only a clean flavor's failures gate the verdict.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use serde_json::Value;
use yaml_rust2::{Yaml, YamlLoader};

use crate::doi::DoiIdentifier;
use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Identifiers and flavors
// ---------------------------------------------------------------------------

/// A parsed identifier of any ported flavor.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Identifier {
    /// A DOI identifier.
    Doi(DoiIdentifier),
}

impl Identifier {
    /// The canonical hash (`_type`-tagged attribute record).
    pub fn to_hash(&self) -> Value {
        match self {
            Identifier::Doi(id) => id.to_hash(),
        }
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        match self {
            Identifier::Doi(id) => id.to_human(),
        }
    }

    /// The emitted URN, when the flavor emits one.
    pub fn to_urn(&self) -> Option<String> {
        match self {
            Identifier::Doi(id) => Some(id.to_urn()),
        }
    }

    /// Build from a canonical hash (the polymorphic `_type` dispatch).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::InvalidHash {
                detail: "missing _type".to_string(),
            })?;
        match kind {
            DoiIdentifier::TYPE => Ok(Identifier::Doi(DoiIdentifier::from_hash(hash)?)),
            other => Err(Error::InvalidHash {
                detail: format!("unknown _type {other:?}"),
            }),
        }
    }
}

/// A flavor implementation: parse an input into an [`Identifier`].
pub trait FlavorImpl: Sync {
    /// The flavor name (the testsuite `tests/<name>/` directory).
    fn name(&self) -> &'static str;

    /// Parse an input spelling; `Err` when the input must be rejected.
    fn parse(&self, input: &str) -> Result<Identifier>;
}

/// The doi flavor implementation.
pub struct DoiImpl;

impl FlavorImpl for DoiImpl {
    fn name(&self) -> &'static str {
        "doi"
    }

    fn parse(&self, input: &str) -> Result<Identifier> {
        Ok(Identifier::Doi(DoiIdentifier::parse(input)?))
    }
}

/// The implemented flavors, by testsuite name.
pub fn implemented_flavors() -> BTreeSet<&'static str> {
    BTreeSet::from(["doi"])
}

/// The flavor implementation for a testsuite name.
pub fn flavor_impl(name: &str) -> Option<&'static dyn FlavorImpl> {
    match name {
        "doi" => Some(&DoiImpl),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Corpus wire contract
// ---------------------------------------------------------------------------

/// A non-normalized alias spelling from a corpus row.
#[derive(Debug, Clone)]
pub struct Alias {
    /// The alias spelling to parse.
    pub spelling: String,
    /// Render/parse rounds needed to reach the canonical human (above 1
    /// only, a documented multi-round normalization chain).
    pub rounds: u32,
}

/// One corpus case, per the testsuite wire contract (schema/
/// test.schema.yaml in pubid-testsuite is the authority).
#[derive(Debug, Clone)]
pub struct CorpusCase {
    /// The row id, e.g. `doi.resource.0001`.
    pub id: String,
    /// The reference implementation's canonical hash, carried as data.
    pub identifier: Option<Value>,
    /// The canonical spelling AND canonical parse input.
    pub human: Option<String>,
    /// The emitted URN, absent when the reference emits none.
    pub urn: Option<String>,
    /// The non-normalized alias spellings.
    pub aliases: Vec<Alias>,
    /// The raw input on debt/negative rows.
    pub input: Option<String>,
    /// The row expects an error (`expect.error`).
    pub expect_error: bool,
    /// Reference-bug debt: nothing to gate on.
    pub quarantined: bool,
    /// The rendered human does not re-parse in the reference itself.
    pub render_unreparseable: bool,
    /// Contract-linked expectation flag; never gated.
    pub review: Option<String>,
}

/// The `_status.yaml` gate.
#[derive(Debug, Clone, Copy)]
pub struct Status {
    /// Whether the flavor's ledger is clean.
    pub clean: bool,
    /// The ledger's known mismatch count.
    pub known_mismatches: u64,
}

/// A flavor's corpus: positive rows, negative rows, and the status gate.
#[derive(Debug, Clone)]
pub struct FlavorCorpus {
    /// The flavor name.
    pub flavor: String,
    /// Rows from the positive corpus files.
    pub positives: Vec<CorpusCase>,
    /// Rows from `_negative.yaml`.
    pub negatives: Vec<CorpusCase>,
    /// The `_status.yaml` gate, when present.
    pub status: Option<Status>,
}

impl FlavorCorpus {
    /// Load a flavor's corpus from a testsuite checkout
    /// (`tests_root` is the checkout's `tests/` directory).
    pub fn from_dir(tests_root: &Path, flavor: &str) -> Result<Self> {
        let dir = tests_root.join(flavor);
        let mut entries: Vec<(String, String)> = Vec::new();
        let list = std::fs::read_dir(&dir).map_err(Error::Io)?;
        for entry in list {
            let entry = entry.map_err(Error::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".yaml") {
                let text = std::fs::read_to_string(entry.path()).map_err(Error::Io)?;
                entries.push((name, text));
            }
        }
        entries.sort();
        Self::from_entries(flavor, &entries)
    }

    /// Build a corpus from named YAML documents (embedded fixture use).
    pub fn from_entries(flavor: &str, entries: &[(String, String)]) -> Result<Self> {
        let mut positives = Vec::new();
        let mut negatives = Vec::new();
        let mut status = None;
        for (name, text) in entries {
            let path = format!("{flavor}/{name}");
            if name == "_status.yaml" {
                status = Some(status_from_yaml(text, &path)?);
            } else if name == "_negative.yaml" {
                negatives = cases_from_yaml(text, &path)?;
            } else {
                positives.extend(cases_from_yaml(text, &path)?);
            }
        }
        Ok(Self {
            flavor: flavor.to_string(),
            positives,
            negatives,
            status,
        })
    }

    /// The vendored doi fixture slice (self-contained smoke corpus).
    pub fn doi_vendored() -> Result<Self> {
        const RESOURCE: &str = include_str!("../fixtures/testsuite/doi/resource.yaml");
        const NEGATIVE: &str = include_str!("../fixtures/testsuite/doi/_negative.yaml");
        const STATUS: &str = include_str!("../fixtures/testsuite/doi/_status.yaml");
        Self::from_entries(
            "doi",
            &[
                ("resource.yaml".to_string(), RESOURCE.to_string()),
                ("_negative.yaml".to_string(), NEGATIVE.to_string()),
                ("_status.yaml".to_string(), STATUS.to_string()),
            ],
        )
    }
}

/// Read the `_status.yaml` gate.
fn status_from_yaml(text: &str, path: &str) -> Result<Status> {
    let doc = YamlLoader::load_from_str(text)
        .map_err(|err| corpus_error(path, format!("invalid YAML: {err}")))?
        .into_iter()
        .find(|doc| !matches!(doc, Yaml::Null | Yaml::BadValue))
        .ok_or_else(|| corpus_error(path, "no status document"))?;
    let clean = doc["clean"]
        .as_bool()
        .ok_or_else(|| corpus_error(path, "status has no clean flag"))?;
    let known_mismatches = doc["known_mismatches"].as_i64().unwrap_or(0).max(0) as u64;
    Ok(Status {
        clean,
        known_mismatches,
    })
}

// ---------------------------------------------------------------------------
// YAML -> corpus rows
// ---------------------------------------------------------------------------

fn cases_from_yaml(text: &str, path: &str) -> Result<Vec<CorpusCase>> {
    let docs = YamlLoader::load_from_str(text)
        .map_err(|err| corpus_error(path, format!("invalid YAML: {err}")))?;
    let mut cases = Vec::new();
    for doc in &docs {
        let rows = match doc {
            Yaml::Array(rows) => rows.as_slice(),
            Yaml::Null | Yaml::BadValue => continue,
            Yaml::Hash(_) => std::slice::from_ref(doc),
            other => {
                return Err(corpus_error(
                    path,
                    format!("unexpected document shape {other:?}"),
                ))
            }
        };
        for row in rows {
            cases.push(row_to_case(row, path)?);
        }
    }
    Ok(cases)
}

/// Map one wire row to a [`CorpusCase`].
fn row_to_case(row: &Yaml, path: &str) -> Result<CorpusCase> {
    let hash = row
        .as_hash()
        .ok_or_else(|| corpus_error(path, "row is not a mapping"))?;
    let field = |key: &str| hash.get(&Yaml::String(key.to_string()));

    let id = field("id")
        .and_then(Yaml::as_str)
        .ok_or_else(|| corpus_error(path, "row has no string id"))?
        .to_string();

    let identifier = field("identifier").map(yaml_to_json);
    let representations = field("representations");
    let human = representations
        .and_then(|r| r["human"].as_str())
        .map(str::to_string);
    let urn = representations
        .and_then(|r| r["urn"].as_str())
        .map(str::to_string);

    let aliases = match field("non_normalized_aliases") {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => Vec::new(),
        Some(Yaml::Array(items)) => items
            .iter()
            .map(|alias| {
                let spelling = alias["spelling"]
                    .as_str()
                    .ok_or_else(|| corpus_error(path, format!("{id}: alias has no spelling")))?
                    .to_string();
                let rounds = alias["rounds"].as_i64().unwrap_or(1).max(1) as u32;
                Ok(Alias { spelling, rounds })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(other) => {
            return Err(corpus_error(
                path,
                format!("{id}: non_normalized_aliases is {other:?}"),
            ))
        }
    };

    let expect_error = !matches!(
        field("expect").map(|expect| &expect["error"]),
        None | Some(Yaml::Null) | Some(Yaml::BadValue)
    );

    let quarantined = identifier.is_none() && !expect_error;
    let review = field("review").and_then(Yaml::as_str).map(str::to_string);

    Ok(CorpusCase {
        id,
        identifier,
        human,
        urn,
        aliases,
        input: field("input").and_then(Yaml::as_str).map(str::to_string),
        expect_error,
        quarantined,
        render_unreparseable: matches!(field("render_unreparseable"), Some(Yaml::Boolean(true))),
        review,
    })
}

/// Convert a YAML node to JSON for the hash comparison. YAML reals go
/// through f64; the corpus holds strings and integers in practice.
fn yaml_to_json(value: &Yaml) -> Value {
    match value {
        Yaml::Null | Yaml::BadValue | Yaml::Alias(_) => Value::Null,
        Yaml::Boolean(b) => Value::Bool(*b),
        Yaml::Integer(i) => Value::Number((*i).into()),
        Yaml::Real(text) => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Yaml::String(s) => Value::String(s.clone()),
        Yaml::Array(items) => Value::Array(items.iter().map(yaml_to_json).collect()),
        Yaml::Hash(map) => {
            let mut object = serde_json::Map::new();
            for (key, member) in map {
                let key = match key {
                    Yaml::String(s) => s.clone(),
                    Yaml::Integer(i) => i.to_string(),
                    other => format!("{other:?}"),
                };
                object.insert(key, yaml_to_json(member));
            }
            Value::Object(object)
        }
    }
}

fn corpus_error(path: &str, detail: impl Into<String>) -> Error {
    Error::Corpus {
        path: path.to_string(),
        detail: detail.into(),
    }
}

// ---------------------------------------------------------------------------
// The checker
// ---------------------------------------------------------------------------

/// Per-category measurement counters (the honest counts a run reports).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Successful parses of positive inputs and alias spellings.
    pub parsed: u64,
    /// Rejected negative inputs (expected-error rows that failed to
    /// parse, as required).
    pub rejected: u64,
    /// Human renders compared against the corpus.
    pub rendered: u64,
    /// Canonical hashes deep-compared against the corpus.
    pub hash_compared: u64,
    /// URNs compared against the corpus.
    pub urn_compared: u64,
    /// Deserialize round-trips checked (`from_hash(to_hash())`).
    pub deserialize_checked: u64,
}

/// Check one corpus case: parse → render → compare. Returns the
/// mismatch list (empty means passing).
///
/// Mirrors pubid-ts `checkCase` field for field: canonical hash, human,
/// urn, every non-normalized alias (multi-round chains re-walk the
/// render/parse round per `rounds`), then the deserialize round-trip.
fn check_case(case: &CorpusCase, flavor: &dyn FlavorImpl, tally: &mut Tally) -> Vec<String> {
    let mut mismatches = Vec::new();
    let label = &case.id;

    let Some(human) = case.human.as_deref() else {
        mismatches.push(format!("{} has no representations.human", case.id));
        return mismatches;
    };

    let identifier = match flavor.parse(human) {
        Ok(identifier) => {
            tally.parsed += 1;
            identifier
        }
        Err(err) => {
            mismatches.push(format!("{label} raised {err}"));
            return mismatches;
        }
    };

    let hash = identifier.to_hash();
    if let Some(expected) = &case.identifier {
        tally.hash_compared += 1;
        if hash != *expected {
            mismatches.push(format!("{label} canonical hash"));
        }
    }

    tally.rendered += 1;
    if identifier.to_human() != human {
        mismatches.push(format!("{label} human"));
    }

    if let Some(expected_urn) = &case.urn {
        match identifier.to_urn() {
            Some(urn) => {
                tally.urn_compared += 1;
                if &urn != expected_urn {
                    mismatches.push(format!("{label} urn"));
                }
            }
            None => mismatches.push(format!("{label} urn absent")),
        }
    }

    for alias in &case.aliases {
        let chain = (0..alias.rounds).try_fold(alias.spelling.clone(), |input, _| {
            flavor.parse(&input).map(|parsed| {
                tally.parsed += 1;
                parsed.to_human()
            })
        });
        match chain {
            Ok(rendered) => {
                tally.rendered += 1;
                if rendered != human {
                    mismatches.push(format!("{label} alias {}", alias.spelling));
                }
            }
            Err(err) => {
                mismatches.push(format!("{label} alias {} raised {err}", alias.spelling));
            }
        }
    }

    match Identifier::from_hash(&hash) {
        Ok(rebuilt) => {
            tally.deserialize_checked += 1;
            if rebuilt.to_hash() != hash {
                mismatches.push(format!("{label} deserialize"));
            }
        }
        Err(err) => mismatches.push(format!("{label} deserialize raised {err}")),
    }

    mismatches
}

/// The conformance verdict for one flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every gated case passed.
    Pass,
    /// A clean flavor has failures.
    Fail,
    /// The flavor's ledger is dirty; its shape is recorded, not gated.
    Ledger,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Pass => write!(f, "pass"),
            Outcome::Fail => write!(f, "fail"),
            Outcome::Ledger => write!(f, "ledger"),
        }
    }
}

/// The run report for one flavor.
#[derive(Debug, Clone)]
pub struct FlavorReport {
    /// The flavor name.
    pub flavor: String,
    /// The verdict.
    pub outcome: Outcome,
    /// Gated positive cases that were checked.
    pub cases: u64,
    /// Expected-error rows checked.
    pub errors: u64,
    /// Quarantined (reference-debt) rows skipped.
    pub quarantined: u64,
    /// Review-flagged rows skipped (never gated).
    pub review: u64,
    /// Negative rows that carry no error expectation (the exporter's
    /// reclassify alarms; counted, not gated).
    pub reclassified: u64,
    /// Positive rows that must parse but were skipped as unimplemented.
    pub unimplemented: u64,
    /// The measurement counters.
    pub tally: Tally,
    /// The mismatch list.
    pub failures: Vec<String>,
}

/// Run one flavor's corpus through its implementation.
pub fn run_flavor(corpus: &FlavorCorpus, flavor: &dyn FlavorImpl) -> FlavorReport {
    let mut tally = Tally::default();
    let mut failures = Vec::new();
    let mut cases = 0u64;
    let mut errors = 0u64;
    let mut quarantined = 0u64;
    let mut review = 0u64;
    let mut reclassified = 0u64;

    for case in &corpus.positives {
        if case.review.is_some() {
            review += 1;
            continue;
        }
        if case.expect_error {
            errors += 1;
            if let Some(input) = &case.input {
                match flavor.parse(input) {
                    Ok(_) => failures.push(format!("{} unexpectedly parsed", case.id)),
                    Err(_) => tally.rejected += 1,
                }
            }
            continue;
        }
        if case.quarantined || case.render_unreparseable {
            quarantined += 1;
            continue;
        }
        cases += 1;
        failures.extend(check_case(case, flavor, &mut tally));
    }

    for row in &corpus.negatives {
        if !row.expect_error {
            reclassified += 1;
            continue;
        }
        let Some(input) = &row.input else {
            continue;
        };
        match flavor.parse(input) {
            Ok(_) => failures.push(format!("{} unexpectedly parsed", row.id)),
            Err(_) => tally.rejected += 1,
        }
    }

    let clean = corpus.status.map(|status| status.clean).unwrap_or(true);
    let outcome = if clean {
        if failures.is_empty() {
            Outcome::Pass
        } else {
            Outcome::Fail
        }
    } else {
        Outcome::Ledger
    };

    FlavorReport {
        flavor: corpus.flavor.clone(),
        outcome,
        cases,
        errors,
        quarantined,
        review,
        reclassified,
        unimplemented: 0,
        tally,
        failures,
    }
}

/// The report for a flavor with no implementation yet: full corpus
/// footprint, never gates.
pub fn unimplemented_report(corpus: &FlavorCorpus) -> FlavorReport {
    FlavorReport {
        flavor: corpus.flavor.clone(),
        outcome: Outcome::Pass,
        cases: corpus
            .positives
            .iter()
            .filter(|case| case.human.is_some())
            .count() as u64,
        errors: corpus
            .negatives
            .iter()
            .filter(|row| row.expect_error)
            .count() as u64,
        quarantined: 0,
        review: 0,
        reclassified: corpus
            .negatives
            .iter()
            .filter(|row| !row.expect_error)
            .count() as u64,
        unimplemented: 1,
        tally: Tally::default(),
        failures: Vec::new(),
    }
}
