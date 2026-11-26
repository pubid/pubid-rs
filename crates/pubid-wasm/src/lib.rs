//! pubid-wasm: the WebAssembly bindings over pubid-core.
//!
//! A thin JS-facing shim — every parse runs through the same
//! checksum-verified PARG artifacts and the same flavor layer the CLI
//! and the conformance runner use. No grammar logic lives here: the
//! artifact is the contract (the grammar-artifact discipline holds in
//! the wasm target exactly as in the native one), the vendored
//! envelopes are compiled into the bundle, and each load re-verifies
//! the envelope's canonical sha256 through the parsanol engine.
//!
//! The JS surface mirrors the Rust API:
//!
//! - [`pubid_flavors`] / [`pubid_version`] — the flavor table (the
//!   `implemented_flavors()` mirror)
//! - [`pubid_parse`] — one flavor's parse; throws on rejection (the
//!   `Result` mirror)
//! - [`pubid_try_parse`] — the same parse as a verdict object that
//!   never throws (the `parse_ok`-class shape for UI work)
//! - [`pubid_detect`] — the flavor detection: every ported flavor that
//!   accepts the input, in the canonical flavor order
//! - [`pubid_artifacts`] — the vendored-artifact receipts (checksums
//!   and semantics hashes, by flavor)
//! - [`pubid_examples`] — the corpus-fed example set (the vendored
//!   testsuite slices, distilled)
//!
//! The payloads cross the boundary through `serde_wasm_bindgen`, so
//! the JSON shapes are the canonical-hash / corpus shapes verbatim.
//!
//! # The grammar-artifact loading decision (documented per the
//! playground contract, pubid-rs#22)
//!
//! The artifacts are **bundled**, not fetched: the envelopes for the
//! ported flavors total a few hundred kilobytes of JSON text — smaller
//! than any realistic engine build — and bundling keeps the playground
//! a fully client-side, zero-round-trip parse. The envelopes are the
//! *vendored, checksummed* ones from `pubid-core` (see `/pins.yaml`);
//! no artifact is authored in this crate, and a corrupted envelope
//! fails its checksum at first use rather than parsing through it.

use pubid_core::conformance::{flavor_impl, implemented_flavors, Identifier};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

/// The corpus-fed example rows, distilled once from the vendored
/// testsuite slices (see `fixtures/VENDORED_FROM.md` for the pins and
/// the distillation rule; the `pubid_examples` gate test re-parses
/// every row, so a drifted row fails the suite rather than shipping).
const EXAMPLES_JSON: &str = include_str!("../fixtures/examples.json");

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

/// The parse report: the canonical forms of one parsed identifier.
#[derive(Debug, Serialize)]
pub struct ParseReport {
    /// The flavor that parsed the input.
    pub flavor: String,
    /// The polymorphic `_type` wire tag (the typed tree's root).
    #[serde(rename = "type")]
    pub type_tag: String,
    /// The typed tree: the canonical `_type`-tagged attribute record
    /// (the `Identifier::to_hash` mirror).
    pub hash: Value,
    /// The canonical human representation (`Identifier::to_human`).
    pub human: String,
    /// The emitted URN / MRN (`Identifier::to_urn`); absent when the
    /// flavor emits none.
    pub urn: Option<String>,
    /// The URN namespace class: `mrn` when the emission is an
    /// `urn:mrn:` member identifier (the IHO/IALA/WMO guidance form),
    /// `urn` for the plain RFC 8141 forms; absent with the URN.
    pub urn_scheme: Option<String>,
}

/// The non-throwing verdict: `ok` carries the report, the error arms
/// carry a stable `code` plus the human message.
#[derive(Debug, Serialize)]
pub struct ParseVerdict {
    /// Whether the input parsed.
    pub ok: bool,
    /// The report, present only when `ok`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ParseReport>,
    /// The human-readable failure message, present only when `!ok`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The failure class: `unknown_flavor` (no such ported flavor),
    /// `parse` (the grammar rejected the input), `invalid` (the bound
    /// capture or canonical hash failed the flavor semantics),
    /// `artifact` (an envelope failed to load or verify).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ParseVerdict {
    fn ok(report: ParseReport) -> Self {
        Self {
            ok: true,
            result: Some(report),
            error: None,
            code: None,
        }
    }

    fn reject(code: &str, message: String) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(message),
            code: Some(code.to_string()),
        }
    }
}

/// One corpus-fed example row.
#[derive(Debug, Serialize)]
pub struct ExampleRow {
    /// The testsuite row id, e.g. `doi.resource.0001`.
    pub id: String,
    /// The canonical spelling (the parse input).
    pub input: String,
    /// The emitted URN, when the corpus records one.
    pub urn: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The stable failure class for a pubid-core error.
fn error_code(err: &pubid_core::Error) -> &'static str {
    use pubid_core::Error as E;
    match err {
        E::Artifact(_) | E::Io(_) => "artifact",
        E::Parse(_) => "parse",
        E::MissingAttribute { .. } | E::InvalidHash { .. } => "invalid",
        E::Corpus { .. } => "corpus",
        _ => "internal",
    }
}

/// Build the report for one parsed identifier.
fn report(flavor: &str, identifier: Identifier) -> ParseReport {
    let urn = identifier.to_urn();
    let urn_scheme = urn.as_deref().map(|urn: &str| {
        if urn.starts_with("urn:mrn:") {
            "mrn"
        } else {
            "urn"
        }
    });
    ParseReport {
        flavor: flavor.to_string(),
        type_tag: identifier
            .to_hash()
            .get("_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        hash: identifier.to_hash(),
        human: identifier.to_human(),
        urn,
        urn_scheme: urn_scheme.map(str::to_string),
    }
}

/// Parse one input through one ported flavor, as a verdict (the
/// failure arms carry the stable `code` classes).
fn parse_flavor(flavor: &str, input: &str) -> ParseVerdict {
    let Some(impl_) = flavor_impl(flavor) else {
        let flavors = implemented_flavors()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        return ParseVerdict::reject(
            "unknown_flavor",
            format!("unimplemented flavor {flavor:?} (ported: {flavors})"),
        );
    };
    match impl_.parse(input) {
        Ok(identifier) => ParseVerdict::ok(report(flavor, identifier)),
        Err(err) => ParseVerdict::reject(error_code(&err), err.to_string()),
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    // JSON-compatible shapes: the maps cross as plain objects (not
    // `Map` instances), so the payloads round-trip through
    // `JSON.stringify`/structured-clone unchanged and the playground
    // reads them with plain property access.
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|err| JsError::new(&err.to_string()))
}

// ---------------------------------------------------------------------------
// The JS-facing surface
// ---------------------------------------------------------------------------

/// The bundle's start hook: route Rust panics through
/// `console.error` so a playground panic is visible in the browser
/// devtools instead of silently trapping the instance.
#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

/// The pubid-core version this bundle was built from.
#[wasm_bindgen]
pub fn pubid_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The ported flavors, in the canonical (sorted) order — the
/// `implemented_flavors()` mirror.
#[wasm_bindgen]
pub fn pubid_flavors() -> Result<JsValue, JsError> {
    let flavors: Vec<String> = implemented_flavors()
        .into_iter()
        .map(str::to_string)
        .collect();
    to_js(&flavors)
}

/// The vendored-artifact receipts, by flavor: the checksum and
/// semantics hash each envelope was loaded and verified under.
#[wasm_bindgen]
pub fn pubid_artifacts() -> Result<JsValue, JsError> {
    let mut receipts = serde_json::Map::new();
    for flavor in implemented_flavors() {
        let Some(Ok(artifact)) = pubid_core::artifact::flavor_artifact(flavor) else {
            continue;
        };
        receipts.insert(
            flavor.to_string(),
            serde_json::json!({
                "checksum": artifact.checksum(),
                "semantics_hash": artifact.semantics_hash(),
                "grammar_name": artifact.grammar_name(),
                "version": artifact.version(),
            }),
        );
    }
    to_js(&Value::Object(receipts))
}

/// Parse one input through one flavor; throws on rejection (the
/// `Result` mirror). See [`pubid_try_parse`] for the non-throwing
/// verdict form.
///
/// # Errors
/// A JS `Error` carrying the human message; the stable failure
/// classes ride [`pubid_try_parse`].
#[wasm_bindgen]
pub fn pubid_parse(flavor: &str, input: &str) -> Result<JsValue, JsError> {
    match parse_flavor(flavor, input) {
        ParseVerdict {
            ok: true,
            result: Some(parsed),
            ..
        } => to_js(&parsed),
        verdict => Err(JsError::new(
            verdict
                .error
                .as_deref()
                .unwrap_or("parse failed (no message)"),
        )),
    }
}

/// Parse one input through one flavor as a verdict object that never
/// throws: `{ ok, result?, error?, code? }`.
#[wasm_bindgen]
pub fn pubid_try_parse(flavor: &str, input: &str) -> Result<JsValue, JsError> {
    to_js(&parse_flavor(flavor, input))
}

/// The flavor detection: every ported flavor that accepts the input,
/// in the canonical flavor order. Empty means no ported flavor parses
/// it.
#[wasm_bindgen]
pub fn pubid_detect(input: &str) -> Result<JsValue, JsError> {
    let hits: Vec<String> = implemented_flavors()
        .into_iter()
        .filter(|flavor| {
            flavor_impl(flavor)
                .map(|impl_| impl_.parse(input).is_ok())
                .unwrap_or(false)
        })
        .map(str::to_string)
        .collect();
    to_js(&hits)
}

/// The corpus-fed example set: rows distilled from the vendored
/// testsuite slices, keyed by flavor. Every row parses under its
/// flavor (the gate test re-parses the whole set).
#[wasm_bindgen]
pub fn pubid_examples() -> Result<JsValue, JsError> {
    let parsed: Value = serde_json::from_str(EXAMPLES_JSON)
        .map_err(|err| JsError::new(&format!("examples fixture invalid: {err}")))?;
    to_js(&parsed)
}
