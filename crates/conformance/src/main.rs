//! The pubid-testsuite conformance runner: load a flavor's corpus,
//! run it through its implementation, report the measured counts
//! honestly, and exit non-zero when a clean flavor fails.
//!
//! Usage:
//!   conformance --flavor doi                        Vendored slice.
//!   conformance --all                               Every ported flavor, vendored slices.
//!   conformance --testsuite <tests-dir> --flavor doi
//!       Run against a pubid-testsuite checkout (`tests/` directory).
//!   conformance --testsuite <tests-dir> --all
//!       Every ported flavor against the checkout.

use std::path::PathBuf;
use std::process::ExitCode;

use pubid_core::conformance::{flavor_impl, implemented_flavors, run_flavor, unimplemented_report};
use pubid_core::{Error, FlavorCorpus, Outcome, VENDORED_FLAVORS};

struct Options {
    testsuite: Option<PathBuf>,
    flavor: String,
    all: bool,
}

fn main() -> ExitCode {
    let mut opts = Options {
        testsuite: std::env::var_os("TESTSUITE_DIR").map(PathBuf::from),
        flavor: "doi".to_string(),
        all: false,
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
            "--all" => opts.all = true,
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => die(&format!("unknown argument {other:?}")),
        }
    }

    let flavors: Vec<String> = if opts.all {
        implemented_flavors()
            .into_iter()
            .map(String::from)
            .collect()
    } else {
        vec![opts.flavor.clone()]
    };

    let mut failing = Vec::new();
    for flavor in &flavors {
        let report = match run_one(&opts, flavor) {
            Ok(report) => report,
            Err(err) => {
                eprintln!("{flavor}: {err}");
                return ExitCode::from(1);
            }
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
        // A clean flavor's failures always gate. A ledgered flavor's
        // shape is recorded, not gated — unless it drifts past the
        // ledger's own documented mismatch count (worse than the
        // reference's documented state).
        let gates = match report.outcome {
            Outcome::Pass => false,
            Outcome::Fail => true,
            // A ledgered flavor's shape is recorded, not gated — unless it
            // drifts past the ledger's own documented mismatch count.
            Outcome::Ledger => (report.failures.len() as u64) > report.known_mismatches,
        };
        if gates {
            failing.push(flavor.clone());
        }
    }

    if failing.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("failing flavors: {}", failing.join(", "));
        ExitCode::from(1)
    }
}

/// Load the corpus (vendored slice or testsuite checkout) and run one
/// flavor through its implementation.
fn run_one(opts: &Options, flavor: &str) -> Result<pubid_core::FlavorReport, Error> {
    // A flavor without a testsuite corpus dir (e.g. ire) runs its
    // vendored slice only — the checkout does not carry every flavor.
    let corpus = match &opts.testsuite {
        Some(root) if root.join(flavor).exists() => FlavorCorpus::from_dir(root, flavor)?,
        _ => vendored_corpus_or_report(flavor)?,
    };
    match flavor_impl(flavor) {
        Some(impl_) => Ok(run_flavor(&corpus, impl_)),
        None => Ok(unimplemented_report(&corpus)),
    }
}

/// The vendored corpus for a flavor; an unknown flavor name is a usage
/// error listing what the crate carries.
fn vendored_corpus_or_report(flavor: &str) -> Result<FlavorCorpus, Error> {
    match pubid_core::vendored_corpus(flavor) {
        Some(corpus) => corpus,
        None => Err(Error::Corpus {
            path: flavor.to_string(),
            detail: format!(
                "no vendored slice; vendored flavors: {} (ported: {})",
                VENDORED_FLAVORS.join(", "),
                implemented_flavors()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }),
    }
}

fn die(message: &str) -> ! {
    eprintln!("{message}");
    print_help();
    std::process::exit(2);
}

fn print_help() {
    eprintln!(
        "usage: conformance [--flavor <name>] [--all] [--testsuite <tests-dir>]\
         \n  default: the vendored doi slice\
         \n  --all: every ported flavor (vendored slices, or the testsuite checkout)\
         \n  TESTSUITE_DIR or --testsuite: run against a pubid-testsuite checkout"
    );
}
