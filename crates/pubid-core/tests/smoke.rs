//! End-to-end smoke tests: the vendored artifact loads, doi parses,
//! renders and round-trips, the embedded suite is green, and the
//! vendored corpus slice passes conformance.

use pubid_core::conformance::{run_flavor, DoiImpl, Outcome};
use pubid_core::{DoiIdentifier, FlavorCorpus, Pubid};
use serde_json::json;

#[test]
fn parses_the_canonical_and_alias_spellings() {
    let canonical = DoiIdentifier::parse("doi:10.1000/182").expect("parse");
    assert_eq!(canonical.prefix, "10.1000");
    assert_eq!(canonical.suffix, "182");

    // Every non-normalized alias class parses to the same identifier.
    for alias in [
        "https://doi.org/10.1000/182",
        "http://doi.org/10.1000/182",
        "doi.org/10.1000/182",
        "DOI:10.1000/182",
        "Doi:10.1000/182",
        "10.1000/182",
    ] {
        assert_eq!(
            DoiIdentifier::parse(alias).expect("parse"),
            canonical,
            "alias {alias}"
        );
    }
}

#[test]
fn renders_and_round_trips() {
    let id = DoiIdentifier::parse("DOI:10.6028/NIST.2022-04-15.001").expect("parse");
    assert_eq!(id.to_human(), "doi:10.6028/NIST.2022-04-15.001");
    assert_eq!(id.to_urn(), "urn:doi");

    let hash = id.to_hash();
    assert_eq!(
        hash,
        json!({
            "_type": "pubid:doi:resource",
            "prefix": "10.6028",
            "suffix": "NIST.2022-04-15.001",
        })
    );

    let rebuilt = DoiIdentifier::from_hash(&hash).expect("from_hash");
    assert_eq!(rebuilt.to_hash(), hash);
}

#[test]
fn rejects_malformed_inputs() {
    for input in ["doi:", "doi:10", "doi:10.X/abc", "doi:10.1000/", "nonsense"] {
        assert!(
            DoiIdentifier::parse(input).is_err(),
            "{input} must be rejected"
        );
    }
}

#[test]
fn embedded_artifact_suite_is_green() {
    let pubid = Pubid::embedded_doi().expect("embedded artifact");
    assert_eq!(pubid.entry(), "identifier");
    let failures = pubid.run_tests();
    assert!(failures.is_empty(), "embedded suite: {failures:?}");
}

#[test]
fn schema_materialization_carries_the_bound_attributes() {
    let pubid = Pubid::embedded_doi().expect("embedded artifact");
    let (entry, attributes) = pubid.materialize("doi:10.1000/182").expect("materialize");
    assert_eq!(entry, "identifier");
    // The raw bound capture: the artifact's `prefix` holds the digits
    // after the grammar's literal `10.` (the flavor layer casts it to
    // the canonical `10.1000`).
    assert_eq!(attributes["prefix"], "1000");
    assert_eq!(attributes["suffix"], "182");
}

#[test]
fn vendored_corpus_slice_passes_conformance() {
    let corpus = FlavorCorpus::doi_vendored().expect("vendored slice");
    let report = run_flavor(&corpus, &DoiImpl);
    assert_eq!(
        report.outcome,
        Outcome::Pass,
        "failures: {:?}",
        report.failures
    );
    assert_eq!(report.cases, 3);
    assert_eq!(report.errors, 0); // error rows live in _negative.yaml
    assert_eq!(report.failures, Vec::<String>::new());

    // The measured counts: every case is parsed, hashed, rendered and
    // round-tripped; every negative is rejected.
    assert_eq!(report.tally.hash_compared, 3);
    assert_eq!(report.tally.rendered, 6); // 3 humans + 3 alias renders
    assert_eq!(report.tally.urn_compared, 3);
    assert_eq!(report.tally.deserialize_checked, 3);
    assert_eq!(report.tally.rejected, 4);
}
