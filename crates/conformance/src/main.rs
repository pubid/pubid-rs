//! The pubid-testsuite conformance runner: load a flavor's corpus,
//! run it through its implementation, report the measured counts
//! honestly, and exit non-zero when a clean flavor fails.
//!
//! Usage:
//!   conformance --flavor doi                        Vendored smoke slice.
//!   conformance --testsuite <tests-dir> --flavor doi
//!       Run against a pubid-testsuite checkout (`tests/` directory).

use std::path::PathBuf;
use std::process::ExitCode;

use pubid_core::conformance::{flavor_impl, run_flavor, unimplemented_report};
use pubid_core::{FlavorCorpus, Outcome};

struct Options {
    testsuite: Option<PathBuf>,
    flavor: String,
}

fn main() -> ExitCode {
    let mut opts = Options {
        testsuite: std::env::var_os("TESTSUITE_DIR").map(PathBuf::from),
        flavor: "doi".to_string(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--testsuite" => {
                opts.testsuite = Some(PathBuf::from(
                    args.next()
                        .unwrap_or_else(|| die("--testsuite needs a directory")),
                ))
            }
            "--flavor" => opts.flavor = args.next().unwrap_or_else(|| die("--flavor needs a name")),
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => die(&format!("unknown argument {other:?}")),
        }
    }

    let corpus = match &opts.testsuite {
        Some(root) => match FlavorCorpus::from_dir(root, &opts.flavor) {
            Ok(corpus) => corpus,
            Err(err) => {
                eprintln!("corpus load failed: {err}");
                return ExitCode::from(1);
            }
        },
        None => match FlavorCorpus::doi_vendored() {
            Ok(corpus) => corpus,
            Err(err) => {
                eprintln!("vendored corpus failed to load: {err}");
                return ExitCode::from(1);
            }
        },
    };

    let report = match flavor_impl(&opts.flavor) {
        Some(impl_) => run_flavor(&corpus, impl_),
        None => unimplemented_report(&corpus),
    };

    println!("flavor {} outcome {}", report.flavor, report.outcome);
    println!(
        "  rows: {} gated cases, {} expected-error, {} quarantined, {} review, {} reclassified",
        report.cases, report.errors, report.quarantined, report.review, report.reclassified
    );
    println!(
        "  measured: {} parsed, {} rejected, {} rendered, {} hashes, {} urns, {} deserializes",
        report.tally.parsed,
        report.tally.rejected,
        report.tally.rendered,
        report.tally.hash_compared,
        report.tally.urn_compared,
        report.tally.deserialize_checked
    );
    if !report.failures.is_empty() {
        println!("  failures ({}):", report.failures.len());
        for failure in &report.failures {
            println!("    {failure}");
        }
    }

    match report.outcome {
        Outcome::Pass => ExitCode::SUCCESS,
        Outcome::Fail | Outcome::Ledger => ExitCode::from(1),
    }
}

fn die(message: &str) -> ! {
    eprintln!("{message}");
    print_help();
    std::process::exit(2);
}

fn print_help() {
    eprintln!(
        "usage: conformance [--flavor <name>] [--testsuite <tests-dir>]\
         \n  default: the vendored doi smoke slice\
         \n  TESTSUITE_DIR or --testsuite: run against a pubid-testsuite checkout"
    );
}
