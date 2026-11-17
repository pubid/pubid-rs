//! End-to-end smoke tests: the vendored artifact loads, doi parses,
//! renders and round-trips, the embedded suite is green, and the
//! vendored corpus slice passes conformance.

use pubid_core::conformance::{run_flavor, DoiImpl, Identifier, Outcome};
use pubid_core::{doi_vendored, DoiIdentifier, Pubid};
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
    let corpus = doi_vendored().expect("vendored slice");
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

/// One canonical spelling per wave-2 flavor: parse → hash → human →
/// URN → deserialize, through the polymorphic enum (the same wire the
/// corpus checker drives).
#[test]
fn wave2_flavors_parse_render_and_round_trip() {
    let cases: &[(&str, &str, serde_json::Value, &str, &str)] = &[
        (
            "isbn",
            "ISBN 978-3-16-148410-0",
            json!({"_type": "pubid:isbn:book", "raw": "9783161484100", "hyphenated": "978-3-16-148410-0"}),
            "ISBN 978-3-16-148410-0",
            "urn:isbn",
        ),
        (
            "iana",
            "IANA _6lowpan-parameters/critical-6lowpan-routing-header-type",
            json!({"_type": "pubid:iana:registry", "number": "_6lowpan-parameters", "sub_registry": "critical-6lowpan-routing-header-type"}),
            "IANA _6lowpan-parameters/critical-6lowpan-routing-header-type",
            "urn:iana:_6lowpan-parameters:critical-6lowpan-routing-header-type",
        ),
        (
            "oasis",
            "OASIS AkomaNtosoCore-v1.0-Pt2-Specifications",
            json!({"_type": "pubid:oasis:standard", "original": "AkomaNtosoCore-v1.0-Pt2-Specifications", "number": "AkomaNtosoCore", "version": "v1.0", "part": "Pt2", "label": "Specifications"}),
            "OASIS AkomaNtosoCore-v1.0-Pt2-Specifications",
            "urn:oasis:AkomaNtosoCore-v1.0-Pt2-Specifications",
        ),
        (
            "xsf",
            "XEP 0001",
            json!({"_type": "pubid:xsf:xep", "number": "0001"}),
            "XEP 0001",
            "urn:xsf:xep:0001",
        ),
        (
            "un",
            "TRADE/CEFACT/2004/32",
            json!({"_type": "pubid:un:document", "path": ["TRADE", "CEFACT", "2004"], "number": "32"}),
            "TRADE/CEFACT/2004/32",
            "urn:un:32:2004",
        ),
        (
            "ogc",
            "OGC 06-121r9",
            json!({"_type": "pubid:ogc:document", "year": "06", "number": "121", "revision": "r9"}),
            "06-121r9",
            "urn:ogc:06:121:r9",
        ),
        (
            "ietf",
            "draft-ietf-httpbis-messaging-19",
            json!({"_type": "pubid:ietf:internet-draft", "number": "draft-ietf-httpbis-messaging", "version": "19"}),
            "draft-ietf-httpbis-messaging-19",
            "urn:ietf:id:draft-ietf-httpbis-messaging:19",
        ),
        (
            "w3c",
            "W3C NOTE-TPRC-970930",
            json!({"_type": "pubid:w3c:note", "number": "TPRC", "date": "970930"}),
            "W3C NOTE-TPRC-970930",
            "urn:w3c:note:TPRC:970930",
        ),
    ];
    for (flavor, input, hash, human, urn) in cases {
        let impl_ = pubid_core::flavor_impl(flavor).expect("ported flavor");
        let identifier = impl_
            .parse(input)
            .unwrap_or_else(|err| panic!("{flavor}: {err}"));
        assert_eq!(identifier.to_hash(), *hash, "{flavor} hash");
        assert_eq!(identifier.to_human(), *human, "{flavor} human");
        assert_eq!(identifier.to_urn().as_deref(), Some(*urn), "{flavor} urn");

        let rebuilt = Identifier::from_hash(&identifier.to_hash())
            .unwrap_or_else(|err| panic!("{flavor} from_hash: {err}"));
        assert_eq!(
            rebuilt.to_hash(),
            identifier.to_hash(),
            "{flavor} round-trip"
        );
    }
}

/// Every flavor with negative rows rejects them all (the vendored
/// slices are the corpora, so this runs offline).
#[test]
fn wave2_negative_rows_are_rejected() {
    for flavor in ["isbn", "iana", "un", "ogc"] {
        let corpus = pubid_core::vendored_corpus(flavor)
            .expect("vendored slice")
            .expect("corpus loads");
        let impl_ = pubid_core::flavor_impl(flavor).expect("ported flavor");
        let report = run_flavor(&corpus, impl_);
        assert_eq!(
            report.outcome,
            Outcome::Pass,
            "{flavor}: {:?}",
            report.failures
        );
        assert_eq!(report.failures, Vec::<String>::new(), "{flavor}");
        assert!(report.tally.rejected > 0, "{flavor} rejected nothing");
    }
}
