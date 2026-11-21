//! The pubid command line: parse an identifier and print its canonical
//! forms, or run the self-contained smoke over the ported flavors.
//!
//! Usage:
//!   pubid-cli parse <flavor> <input>   Parse one identifier, print the forms.
//!   pubid-cli smoke                    Vendored artifacts + slices, green?
//!   pubid-cli flavors                  List the ported flavors.

use std::process::ExitCode;

use pubid_core::conformance::{implemented_flavors, run_flavor};
use pubid_core::{Outcome, VENDORED_FLAVORS};
use serde_json::json;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parse") => parse(&args[1..]),
        Some("smoke") => smoke(),
        Some("flavors") => {
            println!(
                "{}",
                implemented_flavors()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!(
                "usage: pubid-cli parse <flavor> <input> | pubid-cli smoke | pubid-cli flavors"
            );
            ExitCode::from(2)
        }
    }
}

fn parse(args: &[String]) -> ExitCode {
    let [flavor, input] = args else {
        eprintln!("usage: pubid-cli parse <flavor> <input>");
        return ExitCode::from(2);
    };
    let Some(impl_) = pubid_core::flavor_impl(flavor) else {
        eprintln!(
            "unimplemented flavor {flavor:?} (ported: {})",
            implemented_flavors()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        );
        return ExitCode::from(2);
    };
    let identifier = match impl_.parse(input) {
        Ok(identifier) => identifier,
        Err(err) => {
            eprintln!("parse failed: {err}");
            return ExitCode::from(1);
        }
    };
    let report = json!({
        "input": input,
        "hash": identifier.to_hash(),
        "human": identifier.to_human(),
        "urn": identifier.to_urn(),
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
    ExitCode::SUCCESS
}

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
                return ExitCode::from(1);
            }
            None => continue,
        };
        let report = run_flavor(&corpus, impl_, None);
        let embedded = match pubid_core::flavor_artifact(flavor) {
            Some(Ok(artifact)) => artifact.run_tests(),
            Some(Err(err)) => {
                eprintln!("vendored artifact failed to load: {err}");
                return ExitCode::from(1);
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
