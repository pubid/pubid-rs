//! The pubid_examples gate: every corpus-fed example row must parse
//! under its flavor and re-emit the recorded canonical human and URN.
//!
//! This is the drift gate for the `fixtures/examples.json` projection
//! (see `fixtures/VENDORED_FROM.md`): when a slice refresh round moves
//! a row, the stale projection fails here instead of shipping to the
//! playground.

use serde_json::Value;

const EXAMPLES_JSON: &str = include_str!("../fixtures/examples.json");

#[test]
fn examples_parse_and_reemit_their_recorded_forms() {
    let set: Value = serde_json::from_str(EXAMPLES_JSON).expect("examples fixture is valid JSON");
    let flavors = set
        .as_object()
        .expect("examples fixture is a flavor-keyed object");
    assert!(!flavors.is_empty(), "the example set is empty");

    let mut total = 0;
    for (flavor, rows) in flavors {
        let impl_ = pubid_core::conformance::flavor_impl(flavor)
            .unwrap_or_else(|| panic!("example flavor {flavor} is not ported"));
        let rows = rows
            .as_array()
            .unwrap_or_else(|| panic!("{flavor} rows are an array"));
        assert!(!rows.is_empty(), "flavor {flavor} carries no rows");
        for row in rows {
            total += 1;
            let id = row["id"].as_str().expect("row id");
            let input = row["input"].as_str().expect("row input");
            let identifier = impl_.parse(input).unwrap_or_else(|err| {
                panic!("{flavor} row {id} ({input:?}) failed to parse: {err}")
            });
            assert_eq!(
                identifier.to_human(),
                input,
                "{flavor} row {id}: canonical human drifted"
            );
            match (identifier.to_urn(), row["urn"].as_str()) {
                // The corpus gates only the rows it records a URN
                // expectation for; an emission where the row records
                // none is the reference's choice, not projection
                // drift.
                (Some(urn), Some(expected)) => {
                    assert_eq!(urn, expected, "{flavor} row {id}: URN drifted");
                }
                (Some(_), None) => {}
                (None, Some(expected)) => {
                    panic!("{flavor} row {id}: records {expected} but emits none")
                }
                (None, None) => {}
            }
        }
    }
    // The distillation rule emits five rows per ported flavor with a
    // vendored slice; the corpus currently covers every ported flavor,
    // so the set stays at or above one row per flavor and the exact
    // total is asserted loosely (the slice rounds own the counts).
    assert!(
        total >= flavors.len(),
        "every flavor key carries at least one row ({total} rows, {len} flavors)",
        len = flavors.len()
    );
}

#[test]
fn every_ported_flavor_with_a_slice_is_represented() {
    let set: Value = serde_json::from_str(EXAMPLES_JSON).expect("examples fixture is valid JSON");
    let flavors = set.as_object().expect("flavor-keyed object");
    for name in pubid_core::conformance::implemented_flavors() {
        if pubid_core::vendored_corpus(name).is_some() {
            assert!(
                flavors.contains_key(name),
                "ported flavor {name} has a vendored slice but no example rows"
            );
        }
    }
}
