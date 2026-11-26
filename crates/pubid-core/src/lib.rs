//! pubid-core: the Rust PubID models over parsanol PARG artifacts.
//!
//! The artifact is the contract. Grammars live in pubid-grammar and are
//! baked into checksummed PARG envelopes; this crate loads them through
//! the parsanol engine (parse → bound hash), layers the flavor
//! semantics on top (canonical hash, human, URN), and runs the
//! pubid-testsuite corpus through the same parse → render → compare
//! pipeline as the Ruby `Pubid::Conformance::Runner` and pubid-ts.
//!
//! - `artifact`: the embedded, checksum-verified envelopes (one per
//!   flavor, `flavor_artifact` is the lookup)
//! - the flavor modules: `doi` (the bootstrap smoke) plus the wave-2
//!   render-layer flavors `isbn`, `iana`, `oasis`, `xsf`, `un`, `ogc`,
//!   `ietf`, `w3c` — each parse → cast → render over its bound hash;
//!   the later rounds add the mid fleet, `iho` (the S-158
//!   `product` series model, pubid#774/#775, renamed from `checks_for` by gem #783) and `iala` (the
//!   S-2xx product specifications and the codeless slug publications,
//!   gem #772/#777) and `iec` (the consolidated-chain surface, the
//!   `+AMD+COR` composition, gem pubid#788 — the full flavor port
//!   follows)
//! - `bound`: the shared bound-hash / canonical-hash readers
//! - `conformance`: the corpus wire contract and the checker
//! - `Pubid` / `materialize_from_schema`: the schema-driven
//!   materialization boundary kept from the first bootstrap (RS1)

pub mod amca;
pub mod ansi;
pub mod api;
pub mod artifact;
pub mod arxiv;
pub mod bound;
pub mod ccsds;
pub mod cen_cenelec;
pub mod conformance;
pub mod doi;
pub mod ecma;
pub mod error;
pub mod etsi;
pub mod eu;
pub mod evs;
pub mod gb;
pub mod generators;
pub mod gost;
pub mod iala;
pub mod iana;
pub mod iec;
pub mod ieee_code;
pub mod ietf;
pub mod iho;
pub mod ire;
pub mod isbn;
pub mod jis;
pub mod nesc;
pub mod oasis;
pub mod ogc;
pub mod plateau;
pub mod resolver;
pub mod un;
pub mod w3c;
pub mod wmo;
pub mod xsf;

use serde_json::Value;

pub use amca::{AmcaIdentifier, AmcaKind};
pub use ansi::AnsiIdentifier;
pub use api::ApiIdentifier;
pub use artifact::{doi_artifact, flavor_artifact, load_artifact, VENDORED_FLAVORS};
pub use arxiv::ArxivIdentifier;
pub use ccsds::CcsdsIdentifier;
pub use cen_cenelec::CenIdentifier;
pub use conformance::{
    doi_vendored, flavor_impl, implemented_flavors, vendored_corpus, AmcaImpl, ApiImpl, DoiImpl,
    FlavorCorpus, FlavorImpl, FlavorReport, IanaImpl, IetfImpl, IreImpl, IsbnImpl, NescImpl,
    OasisImpl, OgcImpl, Outcome, Tally, UnImpl, W3cImpl, XsfImpl, VENDORED_CORPORA,
};
pub use doi::DoiIdentifier;
pub use ecma::{EcmaIdentifier, EcmaKind};
pub use error::{Error, Result};
pub use etsi::EtsiIdentifier;
pub use eu::{EuIdentifier, EuKind};
pub use evs::EvsIdentifier;
pub use gb::{GbIdentifier, GbKind};
pub use generators::{
    attested_universe, mutate_row, AliasSpelling, FacetTemplate, FlavorUniverse, Universe,
};
pub use gost::{GostIdentifier, GostKind};
pub use iala::{IalaIdentifier, IalaKind};
pub use iana::IanaIdentifier;
pub use iec::{IecDocument, IecIdentifier, IecKind, IecSupplement};
pub use ietf::{IetfIdentifier, IetfKind};
pub use iho::{IhoIdentifier, IhoKind};
pub use ire::IreIdentifier;
pub use isbn::IsbnIdentifier;
pub use jis::JisIdentifier;
pub use nesc::{NescIdentifier, NescKind};
pub use oasis::OasisIdentifier;
pub use ogc::OgcIdentifier;
pub use plateau::PlateauIdentifier;
pub use resolver::{
    accepting_flavors, detect_identifier, resolve, resolver_table, resolver_version, ResolveError,
    Resolved, ResolverEntry, ResolverExample, ResolverTable,
};
pub use un::UnIdentifier;
pub use w3c::{W3cIdentifier, W3cKind};
pub use wmo::{WmoIdentifier, WmoKind};
pub use xsf::XsfIdentifier;

/// Schema-driven materialization (mirrors pubid-ts
/// `materializeFromSchema`): the schema's fields define the attribute
/// set; `*`/`1*` cards become arrays, `0..1` becomes nullable.
pub fn materialize_from_schema(schema: &Value, bound: &Value) -> Result<(String, Value)> {
    let entry = schema["root"]
        .as_str()
        .ok_or_else(|| Error::InvalidHash {
            detail: "schema entry has no root".to_string(),
        })?
        .to_string();
    let mut attributes = serde_json::Map::new();
    for (path, requirements) in schema["fields"]
        .as_object()
        .ok_or_else(|| Error::InvalidHash {
            detail: "schema entry has no fields".to_string(),
        })?
    {
        let leaf = path.rsplit("].").next().unwrap_or(path);
        let card = requirements
            .get("card")
            .and_then(Value::as_str)
            .unwrap_or("1");
        let value = bound.get(leaf);
        let materialized = if card.ends_with('*') {
            Value::Array(match value {
                Some(Value::Array(items)) => items.clone(),
                Some(v) => vec![v.clone()],
                None => Vec::new(),
            })
        } else if card == "0..1" {
            value.cloned().unwrap_or(Value::Null)
        } else if let Some(v) = value {
            v.clone()
        } else {
            continue; // absent scalar: not part of the attribute set
        };
        attributes.insert(leaf.to_string(), materialized);
    }
    Ok((entry, Value::Object(attributes)))
}

/// A loaded artifact plus its schema, ready to parse and materialize.
pub struct Pubid {
    artifact: parsanol::PargArtifact,
    schema: Option<Value>,
    entry: String,
}

impl Pubid {
    /// Load an artifact from disk; the entry defaults to the
    /// compiler-baked `default_entry`.
    pub fn load(path: impl AsRef<std::path::Path>, entry: Option<&str>) -> Result<Self> {
        let artifact = load_artifact(path)?;
        let schema = parsanol::parg::schema::from_artifact(&artifact)
            .map_err(|err| Error::Artifact(err.to_string()))?;
        let entry = entry
            .map(str::to_string)
            .or_else(|| artifact.default_entry().map(str::to_string))
            .ok_or_else(|| {
                Error::Artifact("artifact has no default_entry; pass an entry".to_string())
            })?;
        Ok(Self {
            artifact,
            schema: Some(schema),
            entry,
        })
    }

    /// Load an embedded, checksum-verified artifact.
    pub fn embedded_doi() -> Result<Self> {
        let artifact = doi_artifact()?.clone();
        let schema = parsanol::parg::schema::from_artifact(&artifact)
            .map_err(|err| Error::Artifact(err.to_string()))?;
        let entry = artifact
            .default_entry()
            .unwrap_or(DoiIdentifier::ENTRY)
            .to_string();
        Ok(Self {
            artifact,
            schema: Some(schema),
            entry,
        })
    }

    /// The entry this handle parses through.
    pub fn entry(&self) -> &str {
        &self.entry
    }

    /// Parse and bind; `None` when the input does not parse.
    pub fn parse_and_bind(&self, input: &str) -> Option<Value> {
        self.artifact.parse_and_bind(&self.entry, input).ok()
    }

    /// Parse, bind, and materialize through the schema.
    pub fn materialize(&self, input: &str) -> Option<(String, Value)> {
        let bound = self.parse_and_bind(input)?;
        let schema = self.schema.as_ref()?.get(&self.entry)?;
        materialize_from_schema(schema, &bound).ok()
    }

    /// Embedded-suite runner (RS2); empty means green.
    pub fn run_tests(&self) -> Vec<String> {
        self.artifact.run_tests()
    }
}
