//! The resolver: identifier → canonical publisher URL.
//!
//! A data-driven publisher-URL mapping (the versioned
//! `data/resolver.json` table, embedded and shipped with the crate)
//! plus the tiny render layer that turns a parsed identifier's
//! canonical hash into the attested landing-page URLs. Every entry in
//! the table cites its source and carries an example whose rendered
//! canonical URL the resolver tests re-assert (crates/pubid-core/
//! tests/resolver.rs); a URL pattern that cannot be attested from the
//! publisher's own site or a documented deep-link scheme stays out of
//! the table.
//!
//! Slot rendering: a template's `{slot}` names come from the
//! identifier's canonical hash (`_type` excluded), with a small set of
//! documented derived slots computed here —
//!
//! - `{arxiv_id}` / `{arxiv_doi_id}` — the arXiv abs id (`0704.0001`,
//!   `math.GT/0309136`) and its DOI-side form (the subject class is
//!   dropped from the DOI id: `math/0309136`, per arXiv's own DOI
//!   line on the abs page)
//! - `{eli_tag}` / `{celex}` — the EUR-Lex ELI act token (`reg`,
//!   `dir`, `dec`) and the CELEX number (`3{year}{R|L|D}{number}`
//!   zero-padded to four digits)
//! - `{iho_page}` — the IHO series designator (`s-411`, `b-12`)
//! - `{un_symbol}` — the full UN symbol (`A/RES/78/1`)
//! - `{w3c_dated}` — the dated TR companion path
//!   (`2009/CR-exi-20091208`); absent when the identifier carries no
//!   date or its kind prints no maturity token
//!
//! An alternate (and the DOI name) renders only when every slot it
//! names is available; the canonical URL must render completely or
//! the resolution fails.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::conformance::{flavor_impl, implemented_flavors, Identifier};
use crate::error::Error;

/// The embedded, versioned publisher-URL table.
const RESOLVER_JSON: &str = include_str!("../data/resolver.json");

// ---------------------------------------------------------------------------
// The table (the data file's shape, decoded once)
// ---------------------------------------------------------------------------

/// The versioned resolver table (the decoded `data/resolver.json`).
#[derive(Debug, Deserialize)]
pub struct ResolverTable {
    /// The table's own version (bumped with every mapping change).
    pub version: String,
    /// The table's editorial notes (the citation bar and the DOI
    /// coverage note).
    #[serde(default)]
    pub notes: Vec<String>,
    /// The mapping entries.
    pub entries: Vec<ResolverEntry>,
}

/// One publisher-URL mapping entry.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolverEntry {
    /// The flavor the entry maps.
    pub flavor: String,
    /// The exact `_type` the entry matches (absent when
    /// `type_prefix` is set).
    #[serde(default, rename = "type")]
    pub type_tag: Option<String>,
    /// The `_type` prefix the entry matches (e.g. the kind-tagged
    /// flavors `pubid:w3c:`, `pubid:eu:`, `pubid:iho:`).
    #[serde(default, rename = "type_prefix")]
    pub type_prefix: Option<String>,
    /// The canonical URL template.
    pub canonical: String,
    /// The alternate URL templates (rendered only when every slot is
    /// available).
    #[serde(default)]
    pub alternates: Vec<String>,
    /// The DOI name template (the DOI itself, not its URL); absent
    /// when the publisher's DOI pattern is not attested.
    #[serde(default)]
    pub doi: Option<String>,
    /// The attested example: parsed, rendered and asserted by the
    /// tests.
    pub example: ResolverExample,
    /// The entry's citation: the publisher source and the probe
    /// receipt.
    pub source: String,
}

/// The attested example row of one entry.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolverExample {
    /// The input spelling (parses under the entry's flavor).
    pub input: String,
    /// The canonical URL the example must render.
    pub url: String,
}

impl ResolverEntry {
    /// Whether the entry matches a `_type` tag.
    pub fn matches(&self, type_tag: &str) -> bool {
        match (&self.type_tag, &self.type_prefix) {
            (Some(exact), _) => exact == type_tag,
            (None, Some(prefix)) => type_tag.starts_with(prefix.as_str()),
            (None, None) => false,
        }
    }
}

/// The embedded table, decoded once.
pub fn resolver_table() -> &'static ResolverTable {
    static TABLE: OnceLock<ResolverTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(RESOLVER_JSON)
            .expect("the embedded resolver.json data file is valid")
    })
}

/// The embedded table's version.
pub fn resolver_version() -> &'static str {
    &resolver_table().version
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// A resolved identifier: the canonical URL, the rendered alternates
/// and the attested DOI mapping.
#[derive(Debug, Clone, Serialize)]
pub struct Resolved {
    /// The flavor that parsed the input.
    pub flavor: String,
    /// The parsed identifier's `_type` tag.
    pub type_tag: String,
    /// The canonical publisher URL.
    pub url: String,
    /// The rendered alternate URLs.
    pub alternates: Vec<String>,
    /// The attested DOI name (e.g. `10.17487/RFC9110`), when the
    /// publisher's pattern is attested.
    pub doi: Option<String>,
    /// The DOI's `https://doi.org/` URL, when a DOI name rendered.
    pub doi_url: Option<String>,
    /// The entry's citation.
    pub source: String,
}

/// A failed resolution: the input parsed under no flavor, or no
/// attested mapping covers it.
#[derive(Debug)]
pub enum ResolveError {
    /// No ported flavor accepts the input (or the `--flavor`-chosen
    /// flavor rejected it); carries the underlying error and, when
    /// detection picked a flavor, its name.
    Rejected {
        /// The flavor whose parse rejected the input, when known.
        flavor: Option<String>,
        /// The rejection.
        error: Error,
    },
    /// More than one ported flavor accepts the input; the caller must
    /// name the flavor.
    Ambiguous {
        /// The flavors that accept the input, in canonical order.
        flavors: Vec<String>,
    },
    /// The input parsed but the table has no attested mapping for its
    /// flavor/kind.
    Unmapped {
        /// The flavor that parsed the input.
        flavor: String,
        /// The parsed identifier's `_type` tag.
        type_tag: String,
    },
    /// A mapping matched but its canonical template did not render
    /// completely (a required slot was absent from the hash).
    Render {
        /// What failed to render.
        detail: String,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::Rejected { flavor: Some(flavor), error } => {
                write!(f, "flavor {flavor} rejected the input: {error}")
            }
            ResolveError::Rejected { flavor: None, error } => {
                write!(f, "no ported flavor parses the input ({error})")
            }
            ResolveError::Ambiguous { flavors } => {
                write!(f, "ambiguous input ({}); pass a flavor", flavors.join(", "))
            }
            ResolveError::Unmapped { flavor, type_tag } => write!(
                f,
                "no attested publisher URL for flavor {flavor} ({type_tag}); the table only maps attested patterns"
            ),
            ResolveError::Render { detail } => write!(f, "url render failed: {detail}"),
        }
    }
}

/// Resolve one input to its canonical publisher URL: parse (detection
/// picks the flavor when `flavor` is `None`), match the table, render.
pub fn resolve(input: &str, flavor: Option<&str>) -> Result<Resolved, ResolveError> {
    let (flavor_name, identifier) = match flavor {
        Some(name) => {
            let Some(impl_) = flavor_impl(name) else {
                return Err(ResolveError::Rejected {
                    flavor: Some(name.to_string()),
                    error: Error::InvalidHash {
                        detail: format!("unimplemented flavor {name:?}"),
                    },
                });
            };
            match impl_.parse(input) {
                Ok(identifier) => (name.to_string(), identifier),
                Err(error) => {
                    return Err(ResolveError::Rejected {
                        flavor: Some(name.to_string()),
                        error,
                    })
                }
            }
        }
        None => detect(input)?,
    };

    let type_tag = identifier
        .to_hash()
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(entry) = resolver_table()
        .entries
        .iter()
        .find(|entry| entry.flavor == flavor_name && entry.matches(&type_tag))
    else {
        return Err(ResolveError::Unmapped {
            flavor: flavor_name,
            type_tag,
        });
    };

    let slots = slot_map(&type_tag, &identifier.to_hash());
    let url = render(&entry.canonical, &slots).ok_or_else(|| ResolveError::Render {
        detail: format!(
            "canonical template {:?} has a slot the hash does not carry",
            entry.canonical
        ),
    })?;
    let alternates = entry
        .alternates
        .iter()
        .filter_map(|template| render(template, &slots))
        .collect();
    let doi = entry.doi.as_ref().and_then(|template| render(template, &slots));
    let doi_url = doi
        .as_ref()
        .map(|name| format!("https://doi.org/{name}"));

    Ok(Resolved {
        flavor: flavor_name,
        type_tag,
        url,
        alternates,
        doi,
        doi_url,
        source: entry.source.clone(),
    })
}

/// The flavors that accept the input, then the parse. One hit is the
/// answer; zero hits is a rejection; several hits are ambiguous.
fn detect(input: &str) -> Result<(String, Identifier), ResolveError> {
    let mut hits = Vec::new();
    let mut last_error = None;
    for name in implemented_flavors() {
        let Some(impl_) = flavor_impl(name) else {
            continue;
        };
        match impl_.parse(input) {
            Ok(identifier) => hits.push((name, identifier)),
            Err(error) => last_error = Some(error),
        }
    }
    match hits.len() {
        1 => {
            let (name, identifier) = hits.pop().expect("exactly one hit");
            Ok((name.to_string(), identifier))
        }
        0 => Err(ResolveError::Rejected {
            flavor: None,
            error: last_error.unwrap_or_else(|| Error::InvalidHash {
                detail: "no ported flavor parses the input".to_string(),
            }),
        }),
        _ => Err(ResolveError::Ambiguous {
            flavors: hits.into_iter().map(|(name, _)| name.to_string()).collect(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Slot derivation
// ---------------------------------------------------------------------------

/// The render slots: the hash's scalar attributes plus the derived
/// slots. `BTreeMap` keeps the derived-slot computation deterministic
/// and the test receipts stable.
fn slot_map(type_tag: &str, hash: &Value) -> BTreeMap<String, String> {
    let mut slots = BTreeMap::new();
    if let Some(object) = hash.as_object() {
        for (name, value) in object {
            if name == "_type" {
                continue;
            }
            match value {
                Value::String(text) => {
                    slots.insert(name.clone(), text.clone());
                }
                Value::Number(number) => {
                    slots.insert(name.clone(), number.to_string());
                }
                Value::Bool(flag) => {
                    slots.insert(name.clone(), flag.to_string());
                }
                // Arrays and objects never slot directly: the derived
                // slots below render them (the UN path array, for
                // instance).
                _ => {}
            }
        }
    }

    let suffix = type_tag.rsplit(':').next().unwrap_or_default();
    match hash.get("_type").and_then(Value::as_str) {
        Some("pubid:arxiv:eprint") => {
            derive_arxiv(&mut slots, hash);
        }
        Some(t) if t.starts_with("pubid:eu:") => {
            derive_eu(&mut slots, suffix, hash);
        }
        Some("pubid:un:document") => {
            derive_un(&mut slots, hash);
        }
        Some(t) if t.starts_with("pubid:w3c:") => {
            derive_w3c_dated(&mut slots, suffix, hash);
        }
        Some(t) if t.starts_with("pubid:iho:") => {
            derive_iho(&mut slots, suffix, hash);
        }
        _ => {}
    }
    slots
}

/// `{arxiv_id}` and `{arxiv_doi_id}`: the abs id keeps the subject
/// class (`math.GT/0309136`); the DOI id drops it (`math/0309136`),
/// per the DOI line arXiv prints on the abs page.
fn derive_arxiv(slots: &mut BTreeMap<String, String>, hash: &Value) {
    let Some(year) = hash.get("year").and_then(Value::as_u64) else {
        return;
    };
    let Some(month) = hash.get("month").and_then(Value::as_u64) else {
        return;
    };
    let Some(number) = hash.get("number").and_then(Value::as_str) else {
        return;
    };
    let stamp = format!("{:02}{:02}", year % 100, month);
    let abs_id = match (hash.get("archive"), hash.get("subject_class")) {
        (Some(Value::String(archive)), Some(Value::String(class))) => {
            format!("{archive}.{class}/{stamp}{number}")
        }
        (Some(Value::String(archive)), _) => format!("{archive}/{stamp}{number}"),
        _ => format!("{stamp}.{number}"),
    };
    let doi_id = match hash.get("archive") {
        Some(Value::String(archive)) => format!("{archive}/{stamp}{number}"),
        _ => format!("{stamp}.{number}"),
    };
    slots.insert("arxiv_id".to_string(), abs_id);
    slots.insert("arxiv_doi_id".to_string(), doi_id);
}

/// `{eli_tag}` and `{celex}`: the ELI act token (`reg`/`dir`/`dec`)
/// and the CELEX number `3{year}{R|L|D}{number}`, the number
/// zero-padded to four digits (Reg 2016/679 → `32016R0679`).
fn derive_eu(slots: &mut BTreeMap<String, String>, suffix: &str, hash: &Value) {
    let (eli, letter) = match suffix {
        "regulation" => ("reg", 'R'),
        "directive" => ("dir", 'L'),
        "decision" => ("dec", 'D'),
        _ => return,
    };
    slots.insert("eli_tag".to_string(), eli.to_string());
    let (Some(year), Some(number)) = (
        hash.get("year").and_then(Value::as_str),
        hash.get("number").and_then(Value::as_str),
    ) else {
        return;
    };
    slots.insert(
        "celex".to_string(),
        format!("3{year}{letter}{number:0>4}"),
    );
}

/// `{un_symbol}`: the full symbol, the path tokens joined with the
/// trailing number (`A/RES/78/1`).
fn derive_un(slots: &mut BTreeMap<String, String>, hash: &Value) {
    let Some(number) = hash.get("number").and_then(Value::as_str) else {
        return;
    };
    let mut parts: Vec<&str> = hash
        .get("path")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    parts.push(number);
    slots.insert("un_symbol".to_string(), parts.join("/"));
}

/// `{w3c_dated}`: the dated TR companion path
/// (`{yyyy}/{TOKEN}-{slug}-{date}`), present only when the
/// identifier carries a date and its kind prints a maturity token.
fn derive_w3c_dated(slots: &mut BTreeMap<String, String>, suffix: &str, hash: &Value) {
    let Some(token) = w3c_token(suffix) else {
        return;
    };
    let (Some(date), Some(number)) = (
        hash.get("date").and_then(Value::as_str),
        hash.get("number").and_then(Value::as_str),
    ) else {
        return;
    };
    if date.len() < 4 {
        return;
    }
    slots.insert(
        "w3c_dated".to_string(),
        format!("{}/{token}-{number}-{date}", &date[..4]),
    );
}

/// The printed maturity token per `_type` suffix (the mirror of the
/// w3c flavor's kind tokens; the data-file templates reference the
/// derived slot, not the token).
fn w3c_token(suffix: &str) -> Option<&'static str> {
    match suffix {
        "note" => Some("NOTE"),
        "draft-note" => Some("DNOTE"),
        "working-draft" => Some("WD"),
        "candidate-recommendation" => Some("CR"),
        "candidate-recommendation-draft" => Some("CRD"),
        "recommendation" => Some("REC"),
        "proposed-recommendation" => Some("PR"),
        "proposed-edited-recommendation" => Some("PER"),
        "superseded-recommendation" => Some("SPSD"),
        "obsolete-recommendation" => Some("OBSL"),
        _ => None,
    }
}

/// `{iho_page}`: the series designator, lowercase (`s-411`, `b-12`),
/// as the IHO publication pages key on iho.int.
fn derive_iho(slots: &mut BTreeMap<String, String>, suffix: &str, hash: &Value) {
    let letter = match suffix {
        "standard" => "s",
        "publication" => "p",
        "miscellaneous" => "m",
        "bathymetric" => "b",
        "capacity_building" => "c",
        _ => return,
    };
    let Some(number) = hash.get("number").and_then(Value::as_str) else {
        return;
    };
    slots.insert("iho_page".to_string(), format!("{letter}-{number}"));
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

/// Render a template: every `{slot}` is looked up in the slot map; a
/// missing slot leaves the template unrendered (`None` — alternates
/// skip, the canonical surfaces a [`ResolveError::Render`]).
fn render(template: &str, slots: &BTreeMap<String, String>) -> Option<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let end = rest[start..].find('}')? + start;
        out.push_str(&rest[..start]);
        let slot = &rest[start + 1..end];
        out.push_str(slots.get(slot)?);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}
