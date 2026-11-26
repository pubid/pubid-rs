//! The CLI tests: every command through the built `pubid` binary,
//! covering the output shapes and the exit-code contract (0 parse_ok,
//! 1 invalid, 2 unknown_flavor, 3 artifact, 4 unmapped).

use std::process::Command;

fn pubid(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_pubid"))
        .args(args)
        .output()
        .expect("the pubid binary runs");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().expect("the binary exits by code"),
    )
}

fn json(stdout: &str) -> serde_json::Value {
    serde_json::from_str(stdout).expect("the stdout is one JSON document")
}

/// `pubid parse` prints the typed report (hash, human, urn) in text
/// mode and the ParseReport JSON in --json mode.
#[test]
fn parse_prints_the_typed_report() {
    let (stdout, stderr, code) = pubid(&["parse", "doi:10.1000/182"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("flavor: doi"), "stdout: {stdout}");
    assert!(
        stdout.contains("human:  doi:10.1000/182"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("\"prefix\":\"10.1000\""),
        "stdout: {stdout}"
    );

    let (stdout, stderr, code) = pubid(&["parse", "doi:10.1000/182", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let report = json(&stdout);
    assert_eq!(report["flavor"], "doi");
    assert_eq!(report["type"], "pubid:doi:resource");
    assert_eq!(report["hash"]["prefix"], "10.1000");
    assert_eq!(report["human"], "doi:10.1000/182");
    assert_eq!(report["input"], "doi:10.1000/182");
}

/// `--flavor` names the flavor and skips detection; an unimplemented
/// flavor name is exit 2.
#[test]
fn parse_with_flavor_and_unknown_flavor() {
    let (_, stderr, code) = pubid(&["parse", "10.1000/182", "--flavor", "doi"]);
    assert_eq!(code, 0, "stderr: {stderr}");

    let (stdout, stderr, code) = pubid(&["parse", "X", "--flavor", "nosuchflavor"]);
    assert_eq!(code, 2, "unknown flavor is exit 2");
    assert!(stderr.contains("unimplemented flavor"), "stderr: {stderr}");
    assert!(stdout.is_empty(), "stdout: {stdout}");
}

/// A rejected input is exit 1 (invalid), whatever the mode.
#[test]
fn parse_rejection_is_exit_1() {
    let (_, stderr, code) = pubid(&["parse", "this parses under no ported flavor ??"]);
    assert_eq!(code, 1, "stderr: {stderr}");
    assert!(stderr.contains("no ported flavor"), "stderr: {stderr}");

    let (_, _, code) = pubid(&["parse", "this parses under no ported flavor ??", "--json"]);
    assert_eq!(code, 1);
}

/// `pubid urn` prints the URN (and the MRN class); a flavor that
/// emits none is exit 1.
#[test]
fn urn_prints_the_urn_form() {
    let (stdout, stderr, code) = pubid(&["urn", "XEP 0004"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "urn:xsf:xep:0004");

    let (stdout, _, code) = pubid(&["urn", "XEP 0004", "--json"]);
    assert_eq!(code, 0);
    let report = json(&stdout);
    assert_eq!(report["urn"], "urn:xsf:xep:0004");
    assert_eq!(report["urn_scheme"], "urn");
    assert_eq!(report["flavor"], "xsf");

    // The IHO emission is an MRN member identifier.
    let (stdout, stderr, code) = pubid(&["urn", "IHO B-12 2.0.3", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let report = json(&stdout);
    assert_eq!(report["urn_scheme"], "mrn");
    let urn = report["urn"].as_str().expect("urn present");
    assert!(urn.starts_with("urn:mrn:iho:"), "urn: {urn}");
}

/// `pubid detect` lists the accepting flavors (one per line, or a
/// JSON array); an input nothing accepts still exits 0 with an empty
/// list.
#[test]
fn detect_lists_the_accepting_flavors() {
    let (stdout, stderr, code) = pubid(&["detect", "RFC 9110"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let flavors: Vec<&str> = stdout.lines().map(str::trim).collect();
    assert!(flavors.contains(&"ietf"), "stdout: {stdout}");
    // The permissive gb parse is listed honestly too.
    assert!(flavors.contains(&"gb"), "stdout: {stdout}");

    let (stdout, stderr, code) = pubid(&["detect", "RFC 9110", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let hits = json(&stdout);
    assert!(hits.is_array(), "stdout: {stdout}");

    let (stdout, _, code) = pubid(&["detect", "nothing accepts this ??"]);
    assert_eq!(code, 0, "empty detection is not a failure");
    assert_eq!(stdout.trim(), "(none)");
}

/// `pubid resolve` prints the canonical URL, the alternates, the DOI
/// and the citation in text mode; the JSON shape carries the same
/// fields.
#[test]
fn resolve_prints_the_canonical_url_and_the_doi() {
    let (stdout, stderr, code) = pubid(&["resolve", "RFC 9110"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.starts_with("https://www.rfc-editor.org/rfc/rfc9110.html"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("doi:       10.17487/RFC9110"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("doi_url:   https://doi.org/10.17487/RFC9110"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("source:    RFC Editor"), "stdout: {stdout}");

    let (stdout, _, code) = pubid(&["resolve", "RFC 9110", "--json"]);
    assert_eq!(code, 0);
    let resolved = json(&stdout);
    assert_eq!(
        resolved["url"],
        "https://www.rfc-editor.org/rfc/rfc9110.html"
    );
    assert_eq!(resolved["doi"], "10.17487/RFC9110");
    assert_eq!(resolved["alternates"].as_array().map(Vec::len), Some(2));
    assert_eq!(resolved["flavor"], "ietf");
}

/// `pubid resolve` on an attested mapping without a DOI carries none.
#[test]
fn resolve_without_a_doi_mapping_omits_it() {
    let (stdout, stderr, code) = pubid(&["resolve", "IHO S-411 2.0.0", "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let resolved = json(&stdout);
    assert_eq!(resolved["url"], "https://iho.int/en/s-411");
    assert!(resolved["doi"].is_null(), "doi: {resolved}");
}

/// A parsed identifier with no attested mapping is exit 4 (unmapped).
#[test]
fn resolve_unmapped_is_exit_4() {
    let (_, stderr, code) = pubid(&["resolve", "ISBN 978-0-306-40615-7"]);
    assert_eq!(code, 4, "stderr: {stderr}");
    assert!(
        stderr.contains("no attested publisher URL"),
        "stderr: {stderr}"
    );
}

/// A rejected input is exit 1 even for resolve.
#[test]
fn resolve_rejection_is_exit_1() {
    let (_, _, code) = pubid(&["resolve", "nothing accepts this ??"]);
    assert_eq!(code, 1);
}

/// The genuinely ambiguous input is exit 2; naming the flavor
/// resolves it.
#[test]
fn resolve_ambiguity_is_exit_2_and_flavor_resolves_it() {
    let (_, stderr, code) = pubid(&["resolve", "S-411 1.2.1"]);
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(stderr.contains("ambiguous input"), "stderr: {stderr}");

    let (stdout, _, code) = pubid(&["resolve", "S-411 1.2.1", "--flavor", "iho"]);
    assert_eq!(code, 0, "stdout: {stdout}");
    assert_eq!(stdout.lines().next(), Some("https://iho.int/en/s-411"));
}

/// `pubid flavors` lists the ported flavors; `pubid smoke` runs the
/// vendored set green.
#[test]
fn flavors_and_smoke_run() {
    let (stdout, _, code) = pubid(&["flavors"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("doi"), "stdout: {stdout}");
    assert!(stdout.contains("iho"), "stdout: {stdout}");

    let (stdout, stderr, code) = pubid(&["smoke"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("smoke: flavor doi"), "stdout: {stdout}");
}

/// The usage surface: no arguments is a usage error (clap exits 2)
/// and --help works.
#[test]
fn usage_and_help() {
    let (_, _, code) = pubid(&[]);
    assert_eq!(code, 2, "no arguments is a usage error");

    let (stdout, _, code) = pubid(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("parse"), "stdout: {stdout}");
    assert!(stdout.contains("resolve"), "stdout: {stdout}");

    let (stdout, _, code) = pubid(&["parse", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--flavor"), "stdout: {stdout}");
}
