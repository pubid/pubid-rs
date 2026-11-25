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

use pubid_core::conformance::{
    flavor_impl, implemented_flavors, run_flavor, unimplemented_report, PendingRegistry,
};
use pubid_core::{Error, FlavorCorpus, Outcome, VENDORED_FLAVORS};

/// The implementation-side pending markers (the gem's
/// `conformance/pending.yaml`): corpus cases this implementation
/// explicitly cannot handle yet, with a reason and the ref the unmark
/// waits on. They apply to the testsuite-checkout runs — the corpora
/// that may still record expectations the implementation has outgrown.
const PENDING_YAML: &str = include_str!("../pending.yaml");

struct Options {
    testsuite: Option<PathBuf>,
    flavor: String,
    all: bool,
    /// Flavors to skip on an `--all` run: the ones gated at their own
    /// flavor-only testsuite ref (see `/pins.yaml`,
    /// `testsuite_ref_*`), whose `tests/<flavor>/` at the generic ref
    /// records the pre-round rows. A skipped flavor prints its line —
    /// visible debt placement, never a silent pass.
    skip: Vec<String>,
}

fn main() -> ExitCode {
    let mut opts = Options {
        testsuite: std::env::var_os("TESTSUITE_DIR").map(PathBuf::from),
        flavor: "doi".to_string(),
        all: false,
        skip: Vec::new(),
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
            "--skip-flavor" => {
                opts.skip.push(
                    args.next()
                        .unwrap_or_else(|| die("--skip-flavor needs a name")),
                );
            }
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
    // Pending markers bind to the testsuite checkout: the vendored
    // slices are byte-current with the implementation, so a vendored
    // run never needs them (and a pending row that passes there would
    // fire vacuous pending-satisfied alarms).
    let pending = match &opts.testsuite {
        Some(_) => match PendingRegistry::from_yaml(PENDING_YAML) {
            Ok(registry) => Some(registry),
            Err(err) => {
                eprintln!("pending.yaml: {err}");
                return ExitCode::from(1);
            }
        },
        None => None,
    };
    for flavor in &flavors {
        // A flavor gated at its own flavor-only testsuite ref skips
        // the generic checkout run: its `tests/<flavor>/` at this ref
        // records the pre-round rows, and the dedicated
        // `--testsuite <flavor-ref>/tests --flavor <name>` step gates
        // it at the pinned corpus (the wmo shape, where the fallback
        // is dir absence; iho's dir exists with the pre-re-key rows).
        if opts.all && opts.skip.iter().any(|skipped| skipped == flavor) {
            println!("flavor {flavor} skipped (gated at its flavor-only testsuite ref)");
            continue;
        }
        // A ported flavor whose corpus slice has not landed yet is
        // reported and skipped when no corpus can be loaded at all —
        // visible debt, never a silent pass and never a fabricated
        // gate. When a checkout DOES carry the flavor's rows, the run
        // proceeds and the pending registry records the pre-round debt.
        let checkout_has_corpus = opts
            .testsuite
            .as_ref()
            .map(|root| root.join(flavor).exists())
            .unwrap_or(false);
        if pubid_core::flavor_impl(flavor).is_some()
            && pubid_core::vendored_corpus(flavor).is_none()
            && !checkout_has_corpus
        {
            println!("flavor {flavor} outcome pass (ported; no corpus slice yet — skipped)");
            continue;
        }
        let report = match run_one(&opts, flavor, pending.as_ref()) {
            Ok(report) => report,
            Err(err) => {
                eprintln!("{flavor}: {err}");
                return ExitCode::from(1);
            }
        };

        println!("flavor {} outcome {}", report.flavor, report.outcome);
        println!(
            "  rows: {} gated cases, {} expected-error, {} quarantined, {} review, {} reclassified, {} pending, {} pending-satisfied",
            report.cases, report.errors, report.quarantined, report.review,
            report.reclassified, report.pending, report.pending_satisfied
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
        for id in &report.pending_satisfied_ids {
            println!("  PENDING-SATISFIED {id} - remove the marker");
        }
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
fn run_one(
    opts: &Options,
    flavor: &str,
    pending: Option<&PendingRegistry>,
) -> Result<pubid_core::FlavorReport, Error> {
    // A flavor without a testsuite corpus dir (e.g. ire) runs its
    // vendored slice only — the checkout does not carry every flavor.
    let corpus = match &opts.testsuite {
        Some(root) if root.join(flavor).exists() => FlavorCorpus::from_dir(root, flavor)?,
        _ => vendored_corpus_or_report(flavor)?,
    };
    match flavor_impl(flavor) {
        Some(impl_) => Ok(run_flavor(&corpus, impl_, pending)),
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
        "usage: conformance [--flavor <name>] [--all] [--skip-flavor <name>] [--testsuite <tests-dir>]\
         \n  default: the vendored doi slice\
         \n  --all: every ported flavor (vendored slices, or the testsuite checkout)\
         \n  --skip-flavor: exclude a flavor from --all (gated at its flavor-only testsuite ref)\
         \n  TESTSUITE_DIR or --testsuite: run against a pubid-testsuite checkout"
    );
}
