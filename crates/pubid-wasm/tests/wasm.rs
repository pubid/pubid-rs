//! The wasm-facing API suite (wasm-bindgen-test): the playground's
//! contract, exercised the way the browser exercises it — through the
//! exported functions, over the bundled checksum-verified artifacts.
//!
//! The native suite never runs this file (the js-sys imports have no
//! meaning off the wasm target); wasm-pack compiles it for
//! `wasm32-unknown-unknown` under `--node` or a browser.
//!
//! Run with:
//!   wasm-pack test --node crates/pubid-wasm

#![cfg(target_arch = "wasm32")]

use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

use pubid_wasm::{
    pubid_artifacts, pubid_detect, pubid_examples, pubid_flavors, pubid_parse, pubid_try_parse,
    pubid_version,
};

fn call<T>(value: Result<T, JsError>) -> T {
    match value {
        Ok(value) => value,
        Err(err) => panic!("exported call failed: {err:?}"),
    }
}

fn get(object: &JsValue, field: &str) -> JsValue {
    Reflect::get(object, &field.into()).unwrap_or_else(|_| panic!("field {field} missing"))
}

fn string_at(object: &JsValue, field: &str) -> String {
    get(object, field)
        .as_string()
        .unwrap_or_else(|| panic!("field {field} is not a string"))
}

fn js_string_array(value: &JsValue) -> Vec<String> {
    js_sys::Array::from(value)
        .to_vec()
        .iter()
        .map(|item| {
            item.as_string()
                .unwrap_or_else(|| panic!("array holds a non-string: {item:?}"))
        })
        .collect()
}

#[wasm_bindgen_test]
fn version_and_flavor_table_carry_the_ported_fleet() {
    assert!(!pubid_version().is_empty());
    let flavors = js_string_array(&call(pubid_flavors()));
    assert!(flavors.contains(&"doi".to_string()));
    assert!(flavors.contains(&"iho".to_string()));
    assert!(flavors.len() >= 27, "the ported fleet parses ({flavors:?})");
}

#[wasm_bindgen_test]
fn parse_returns_the_canonical_forms() {
    let report = call(pubid_parse("doi", "doi:10.1000/182"));
    assert_eq!(string_at(&report, "human"), "doi:10.1000/182");
    assert_eq!(
        string_at(&get(&report, "hash"), "_type"),
        "pubid:doi:resource"
    );
    assert_eq!(string_at(&report, "type"), "pubid:doi:resource");
    assert_eq!(string_at(&report, "urn"), "urn:doi");
    assert_eq!(string_at(&report, "urn_scheme"), "urn");
}

#[wasm_bindgen_test]
fn parse_reports_the_mrn_namespace_class() {
    let report = call(pubid_parse("iho", "IHO B-1 1.0.0"));
    let urn = string_at(&report, "urn");
    assert!(
        urn.starts_with("urn:mrn:iho:"),
        "the iho emission is an MRN ({urn})"
    );
    assert_eq!(string_at(&report, "urn_scheme"), "mrn");
}

#[wasm_bindgen_test]
fn try_parse_returns_the_verdict_classes() {
    let ok = call(pubid_try_parse("isbn", "ISBN 978-0-306-40615-7"));
    assert_eq!(get(&ok, "ok"), JsValue::TRUE);

    // The ISO 2108 check-digit failure: the `invalid` class.
    let rejected = call(pubid_try_parse("isbn", "ISBN 978-3-16-148410-1"));
    assert_eq!(get(&rejected, "ok"), JsValue::FALSE);
    assert_eq!(string_at(&rejected, "code"), "invalid");

    let unknown = call(pubid_try_parse("no-such-flavor", "x"));
    assert_eq!(get(&unknown, "ok"), JsValue::FALSE);
    assert_eq!(string_at(&unknown, "code"), "unknown_flavor");

    // The grammar rejection: the `parse` class.
    let unparseable = call(pubid_try_parse("doi", "doi:"));
    assert_eq!(get(&unparseable, "ok"), JsValue::FALSE);
    assert_eq!(string_at(&unparseable, "code"), "parse");
}

#[wasm_bindgen_test]
fn parse_throws_on_rejection() {
    match pubid_parse("doi", "not-a-doi") {
        Ok(_) => panic!("the rejection must throw"),
        Err(err) => assert!(format!("{err:?}").contains("parse failed")),
    }
}

#[wasm_bindgen_test]
fn detect_finds_the_flavor() {
    let hits = js_string_array(&call(pubid_detect("ISBN 978-0-306-40615-7")));
    assert!(
        hits.contains(&"isbn".to_string()),
        "the isbn flavor detects its own input ({hits:?})"
    );

    let none = js_string_array(&call(pubid_detect("this is not an identifier")));
    assert!(none.is_empty(), "unparseable input detects nothing");
}

#[wasm_bindgen_test]
fn artifact_receipts_carry_the_checksums() {
    let receipts = call(pubid_artifacts());
    let doi = get(&receipts, "doi");
    assert!(
        string_at(&doi, "checksum").len() >= 64,
        "the doi envelope carries its sha256"
    );
    assert!(
        !string_at(&doi, "semantics_hash").is_empty(),
        "the doi envelope carries its semantics hash"
    );
}

#[wasm_bindgen_test]
fn the_example_set_parses_under_its_flavors() {
    let set = call(pubid_examples());
    let flavors = js_sys::Object::keys(&js_sys::Object::from(set.clone()));
    assert!(
        flavors.length() >= 20,
        "the example set covers the fleet ({} flavors)",
        flavors.length()
    );
    for flavor_value in flavors.iter() {
        let flavor = flavor_value
            .as_string()
            .unwrap_or_else(|| panic!("flavor key is a string: {flavor_value:?}"));
        let array = js_sys::Array::from(&get(&set, &flavor));
        assert!(array.length() > 0, "flavor {flavor} carries rows");
        for row_value in array.iter() {
            let row = js_sys::Object::from(row_value);
            let input = string_at(&row, "input");
            let verdict = call(pubid_try_parse(&flavor, &input));
            assert_eq!(
                get(&verdict, "ok"),
                JsValue::TRUE,
                "the example row must parse: {flavor} {input}"
            );
        }
    }
}
