//! The `pubid` command line: parse, detect, resolve and render PubIDs
//! over the same checksummed PARG artifacts the library and the
//! conformance runner use.
//!
//! Commands:
//!
//! - `pubid parse <input> [--flavor F] [--json]` — the typed parse
//!   (the canonical hash, the human form, the URN)
//! - `pubid urn <input> [--flavor F] [--json]` — the URN / MRN form
//! - `pubid detect <input> [--json]` — the ported flavors that accept
//!   the input
//! - `pubid resolve <input> [--flavor F] [--json]` — the canonical
//!   publisher URL, the alternates and the attested DOI
//! - `pubid flavors` — the ported flavor list
//! - `pubid smoke` — the vendored artifacts + slices, green?
//!
//! Exit codes (mirroring the wasm verdict classes):
//!
//! - `0` — parse_ok
//! - `1` — invalid: the flavor's grammar or semantics rejected the
//!   input (or `urn` named a flavor that emits no URN)
//! - `2` — unknown_flavor: the named flavor is not ported, or the
//!   input is ambiguous across flavors (pass `--flavor`); clap's own
//!   usage errors exit `2` as well
//! - `3` — artifact: a PARG envelope failed to load or verify
//! - `4` — unmapped: `resolve` parsed the input but the resolver
//!   table carries no attested mapping for it
//!
//! The grammar artifacts are bundled (embedded, checksum-verified
//! envelopes — the same discipline as the wasm target); no external
//! artifact path is consulted at runtime.

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use pubid_core::conformance::{implemented_flavors, run_flavor, Identifier};
use pubid_core::{Error, Outcome, VENDORED_FLAVORS};
use serde::Serialize;
use serde_json::json;

// ---------------------------------------------------------------------------
// The command surface
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "pubid",
    version,
    about = "Parse, detect, resolve and render PubIDs",
    after_help = "Exit codes: 0 parse_ok, 1 invalid, 2 unknown_flavor, 3 artifact, 4 unmapped"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse one identifier and print its canonical forms. Without
    /// --flavor, detection picks the flavor (ambiguous inputs need
    /// --flavor).
    Parse {
        /// The identifier spelling.
        input: String,
        /// Parse through this flavor only.
        #[arg(long)]
        flavor: Option<String>,
        /// Emit the machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// Emit the URN / MRN form of one identifier. Exits 1 when the
    /// flavor emits no URN.
    Urn {
        /// The identifier spelling.
        input: String,
        /// Parse through this flavor only.
        #[arg(long)]
        flavor: Option<String>,
        /// Emit the machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// List the ported flavors that accept the input (empty: none).
    Detect {
        /// The identifier spelling.
        input: String,
        /// Emit a JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Resolve one identifier to its canonical publisher URL, the
    /// alternates and the attested DOI. Exits 4 when the table maps
    /// no attested URL for the flavor.
    Resolve {
        /// The identifier spelling.
        input: String,
        /// Parse through this flavor only (skips detection).
        #[arg(long)]
        flavor: Option<String>,
        /// Emit the machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
    /// List the ported flavors.
    Flavors,
    /// Run the vendored artifacts + slices smoke (exit 1 on red).
    Smoke,
}

/// The failure a command surfaces: the exit class plus the human
/// message (stderr in every mode).
struct Failure {
    code: Class,
    message: String,
}

/// The exit classes, mirroring the wasm verdict codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// The flavor's grammar or semantics rejected the input.
    Invalid,
    /// The named flavor is not ported, or the input is ambiguous.
    UnknownFlavor,
    /// An envelope failed to load or verify.
    Artifact,
    /// The resolver table carries no attested mapping.
    Unmapped,
}

impl From<Class> for ExitCode {
    fn from(class: Class) -> Self {
        match class {
            Class::Invalid => ExitCode::from(1),
            Class::UnknownFlavor => ExitCode::from(2),
            Class::Artifact => ExitCode::from(3),
            Class::Unmapped => ExitCode::from(4),
        }
    }
}

impl From<Error> for Class {
    fn from(err: Error) -> Self {
        match err {
            Error::Artifact(_) | Error::Io(_) => Class::Artifact,
            _ => Class::Invalid,
        }
    }
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Parse {
            input,
            flavor,
            json,
        } => parse(&input, flavor.as_deref(), json),
        Command::Urn {
            input,
            flavor,
            json,
        } => urn(&input, flavor.as_deref(), json),
        Command::Detect { input, json } => detect(&input, json),
        Command::Resolve {
            input,
            flavor,
            json,
        } => resolve(&input, flavor.as_deref(), json),
        Command::Flavors => {
            println!(
                "{}",
                implemented_flavors()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            ExitCode::SUCCESS
        }
        Command::Smoke => smoke(),
    }
}

// ---------------------------------------------------------------------------
// The shared parse path
// ---------------------------------------------------------------------------

/// One parsed identifier plus the flavor that produced it.
struct Parsed {
    flavor: String,
    identifier: Identifier,
}

/// Parse one input through `flavor`, or through detection when
/// `flavor` is `None`.
fn parse_input(input: &str, flavor: Option<&str>) -> Result<Parsed, Failure> {
    match flavor {
        Some(name) => {
            let Some(impl_) = pubid_core::flavor_impl(name) else {
                let flavors = implemented_flavors()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(Failure {
                    code: Class::UnknownFlavor,
                    message: format!("unimplemented flavor {name:?} (ported: {flavors})"),
                });
            };
            match impl_.parse(input) {
                Ok(identifier) => Ok(Parsed {
                    flavor: name.to_string(),
                    identifier,
                }),
                Err(err) => {
                    let message = format!("parse failed: {err}");
                    Err(Failure {
                        code: Class::from(err),
                        message,
                    })
                }
            }
        }
        None => match pubid_core::detect_identifier(input) {
            Ok((flavor, identifier)) => Ok(Parsed { flavor, identifier }),
            Err(pubid_core::ResolveError::Ambiguous { flavors }) => Err(Failure {
                code: Class::UnknownFlavor,
                message: format!("ambiguous input ({}); pass --flavor", flavors.join(", ")),
            }),
            Err(pubid_core::ResolveError::UnknownFlavor { name }) => {
                let flavors = implemented_flavors()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(Failure {
                    code: Class::UnknownFlavor,
                    message: format!("unimplemented flavor {name:?} (ported: {flavors})"),
                })
            }
            Err(err) => Err(Failure {
                code: Class::Invalid,
                message: err.to_string(),
            }),
        },
    }
}

/// The URN namespace class: `mrn` for the `urn:mrn:` members, `urn`
/// for the plain RFC 8141 forms.
fn urn_scheme(urn: &str) -> &'static str {
    if urn.starts_with("urn:mrn:") {
        "mrn"
    } else {
        "urn"
    }
}

/// The parse report (the wasm ParseReport shape plus the input).
#[derive(Serialize)]
struct ParseReport<'a> {
    input: &'a str,
    flavor: String,
    #[serde(rename = "type")]
    type_tag: String,
    hash: serde_json::Value,
    human: String,
    urn: Option<String>,
    urn_scheme: Option<&'static str>,
}

fn report<'a>(input: &'a str, parsed: Parsed) -> ParseReport<'a> {
    let urn = parsed.identifier.to_urn();
    let urn_scheme = urn.as_deref().map(urn_scheme);
    ParseReport {
        input,
        type_tag: parsed
            .identifier
            .to_hash()
            .get("_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        flavor: parsed.flavor,
        hash: parsed.identifier.to_hash(),
        human: parsed.identifier.to_human(),
        urn,
        urn_scheme,
    }
}

fn emit_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("the report serializes")
    );
}

// ---------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------

fn parse(input: &str, flavor: Option<&str>, json: bool) -> ExitCode {
    let parsed = match parse_input(input, flavor) {
        Ok(parsed) => parsed,
        Err(failure) => return fail(&failure),
    };
    if json {
        emit_json(&report(input, parsed));
    } else {
        let report = report(input, parsed);
        println!("flavor: {}", report.flavor);
        println!("human:  {}", report.human);
        match &report.urn {
            Some(urn) => println!("urn:    {urn}"),
            None => println!("urn:    (none)"),
        }
        println!(
            "hash:   {}",
            serde_json::to_string(&report.hash).expect("json")
        );
    }
    ExitCode::SUCCESS
}

fn urn(input: &str, flavor: Option<&str>, json: bool) -> ExitCode {
    let parsed = match parse_input(input, flavor) {
        Ok(parsed) => parsed,
        Err(failure) => return fail(&failure),
    };
    let flavor_name = parsed.flavor.clone();
    let urn = parsed.identifier.to_urn();
    let type_tag = parsed
        .identifier
        .to_hash()
        .get("_type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let urn_scheme = urn.as_deref().map(urn_scheme);
    if json {
        emit_json(&json!({
            "input": input,
            "flavor": flavor_name,
            "type": type_tag,
            "urn": urn,
            "urn_scheme": urn_scheme,
        }));
    }
    match urn {
        Some(urn) => {
            if !json {
                println!("{urn}");
            }
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("flavor {flavor_name} emits no URN for this identifier ({type_tag})");
            Class::Invalid.into()
        }
    }
}

fn detect(input: &str, json: bool) -> ExitCode {
    let hits: Vec<String> = pubid_core::accepting_flavors(input)
        .into_iter()
        .map(str::to_string)
        .collect();
    if json {
        emit_json(&hits);
    } else if hits.is_empty() {
        println!("(none)");
    } else {
        for flavor in &hits {
            println!("{flavor}");
        }
    }
    ExitCode::SUCCESS
}

fn resolve(input: &str, flavor: Option<&str>, json: bool) -> ExitCode {
    match pubid_core::resolve(input, flavor) {
        Ok(resolved) => {
            if json {
                emit_json(&json!({
                    "input": input,
                    "flavor": resolved.flavor,
                    "type": resolved.type_tag,
                    "url": resolved.url,
                    "alternates": resolved.alternates,
                    "doi": resolved.doi,
                    "doi_url": resolved.doi_url,
                    "source": resolved.source,
                }));
            } else {
                println!("{}", resolved.url);
                for alternate in &resolved.alternates {
                    println!("alternate: {alternate}");
                }
                if let Some(doi) = &resolved.doi {
                    println!("doi:       {doi}");
                }
                if let Some(doi_url) = &resolved.doi_url {
                    println!("doi_url:   {doi_url}");
                }
                println!("source:    {}", resolved.source);
            }
            ExitCode::SUCCESS
        }
        Err(pubid_core::ResolveError::UnknownFlavor { name }) => {
            let flavors = implemented_flavors()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ");
            fail(&Failure {
                code: Class::UnknownFlavor,
                message: format!("unimplemented flavor {name:?} (ported: {flavors})"),
            })
        }
        Err(pubid_core::ResolveError::Rejected { error, .. }) => {
            let message = error.to_string();
            fail(&Failure {
                code: Class::from(error),
                message,
            })
        }
        Err(err) => {
            let code = match &err {
                pubid_core::ResolveError::Unmapped { .. } => Class::Unmapped,
                pubid_core::ResolveError::Ambiguous { .. } => Class::UnknownFlavor,
                _ => Class::Invalid,
            };
            fail(&Failure {
                code,
                message: err.to_string(),
            })
        }
    }
}

fn fail(failure: &Failure) -> ExitCode {
    eprintln!("{}", failure.message);
    failure.code.into()
}

// ---------------------------------------------------------------------------
// The smoke (vendored artifacts + slices, green?)
// ---------------------------------------------------------------------------

fn smoke() -> ExitCode {
    let mut green = true;
    for flavor in VENDORED_FLAVORS {
        let Some(impl_) = pubid_core::flavor_impl(flavor) else {
            // Vendored slice without a ported flavor: report, never gate.
            println!("smoke: flavor {flavor} not ported (slice vendored)");
            continue;
        };
        let corpus = match pubid_core::vendored_corpus(flavor) {
            Some(Ok(corpus)) => corpus,
            Some(Err(err)) => {
                eprintln!("vendored corpus failed to load: {err}");
                return Class::Artifact.into();
            }
            None => continue,
        };
        let report = run_flavor(&corpus, impl_, None);
        let embedded = match pubid_core::flavor_artifact(flavor) {
            Some(Ok(artifact)) => artifact.run_tests(),
            Some(Err(err)) => {
                eprintln!("vendored artifact failed to load: {err}");
                return Class::Artifact.into();
            }
            None => Vec::new(),
        };
        // A ledgered flavor's shape is recorded, not gated — the same
        // semantics as the conformance runner's exit gate (api's corpus
        // carries a known_mismatches ceiling).
        let at_or_under_ledger = report.outcome == Outcome::Ledger
            && (report.failures.len() as u64) <= report.known_mismatches;
        let flavor_green =
            (report.outcome == Outcome::Pass || at_or_under_ledger) && embedded.is_empty();
        green &= flavor_green;
        println!(
            "smoke: flavor {} outcome {} ({} cases, {} rejected negatives, {} embedded tests failing)",
            report.flavor,
            report.outcome,
            report.cases,
            report.tally.rejected,
            embedded.len()
        );
        for failure in &report.failures {
            println!("  {failure}");
        }
    }
    if green {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
