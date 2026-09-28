//! RS2: corpus/suite conformance over the native engine, plus RS1
//! materialization checks. Runs against the sibling pubid-grammar
//! artifacts (PG_ARTIFACT_DIR overrides).

use pubid_rs::Pubid;

fn artifacts_dir() -> String {
    std::env::var("PG_ARTIFACT_DIR").unwrap_or_else(|_| {
        for ancestor in std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .skip(1)
        {
            let candidate = ancestor
                .join("pubid")
                .join("pubid-grammar")
                .join("artifacts");
            if candidate.is_dir() {
                return candidate.to_string_lossy().to_string();
            }
        }
        panic!("artifacts dir not found (set PG_ARTIFACT_DIR)")
    })
}

#[test]
fn iso_materializes_schema_typed_attributes() {
    let pubid = Pubid::load(format!("{}/iso.json", artifacts_dir()), Some("identifier")).unwrap();
    let (entry, attributes) = pubid.materialize("ISO 5537:2025").expect("must parse");
    assert_eq!(entry, "identifier");
    assert_eq!(attributes["publisher"], "ISO");
    assert_eq!(
        attributes["publisher_name"],
        "International Organization for Standardization"
    );
}

#[test]
fn iso_embedded_and_external_suites_are_green() {
    let dir = artifacts_dir();
    let pubid = Pubid::load(format!("{dir}/iso.json"), Some("identifier")).unwrap();
    assert!(pubid.run_tests().is_empty());
    let suites = format!("{}/../suites", dir);
    let text = std::fs::read_to_string(format!("{suites}/iso.pgtest")).unwrap();
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("accept \"") {
            let input = rest.trim_end_matches('"');
            assert!(
                pubid.parse_and_bind(input).is_some(),
                "suite accept failed: {input}"
            );
        }
    }
}

#[test]
fn reject_input_is_rejected() {
    let pubid = Pubid::load(format!("{}/iso.json", artifacts_dir()), Some("identifier")).unwrap();
    assert!(pubid.parse_and_bind("nonsense").is_none());
}
