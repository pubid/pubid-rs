//! The pubid command line: parse an identifier and print its canonical
//! forms, or run the self-contained smoke.
//!
//! Usage:
//!   pubid-cli parse doi <input>   Parse one DOI and print the forms.
//!   pubid-cli smoke               Vendored doi artifact + corpus, green?

use std::process::ExitCode;

use pubid_core::{conformance, DoiImpl, FlavorCorpus, FlavorImpl, Outcome};
use serde_json::json;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parse") => parse(&args[1..]),
        Some("smoke") => smoke(),
        _ => {
            eprintln!("usage: pubid-cli parse doi <input> | pubid-cli smoke");
            ExitCode::from(2)
        }
    }
}

fn parse(args: &[String]) -> ExitCode {
    let [flavor, input] = args else {
        eprintln!("usage: pubid-cli parse doi <input>");
        return ExitCode::from(2);
    };
    if flavor != "doi" {
        eprintln!("unimplemented flavor {flavor:?} (ported: doi)");
        return ExitCode::from(2);
    }
    let identifier = match <DoiImpl as FlavorImpl>::parse(&DoiImpl, input) {
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
    let corpus = match FlavorCorpus::doi_vendored() {
        Ok(corpus) => corpus,
        Err(err) => {
            eprintln!("vendored corpus failed to load: {err}");
            return ExitCode::from(1);
        }
    };
    let report = conformance::run_flavor(&corpus, &DoiImpl);
    let pubid = match pubid_core::Pubid::embedded_doi() {
        Ok(pubid) => pubid,
        Err(err) => {
            eprintln!("vendored artifact failed to load: {err}");
            return ExitCode::from(1);
        }
    };
    let embedded = pubid.run_tests();
    let green = report.outcome == Outcome::Pass && embedded.is_empty();
    println!(
        "smoke: flavor {} outcome {} ({} cases, {} rejected negatives, {} embedded tests failing)",
        report.flavor,
        report.outcome,
        report.cases,
        report.tally.rejected,
        embedded.len()
    );
    if green {
        ExitCode::SUCCESS
    } else {
        for failure in &report.failures {
            println!("  {failure}");
        }
        ExitCode::from(1)
    }
}
