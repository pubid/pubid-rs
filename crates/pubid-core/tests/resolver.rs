//! The resolver tests: the citation bar and the render layer.
//!
//! The per-entry gate re-parses every entry's example under its own
//! flavor and asserts the rendered canonical URL against the attested
//! `example.url` — an entry whose pattern drifted from its citation
//! fails here rather than shipping. The rest pins the derived-slot
//! semantics (the arXiv DOI id's subject-class drop, the CELEX
//! zero-padding, the IHO series letters, the conditional W3C dated
//! path and draft revision path) and the error classes.

use pubid_core::conformance::flavor_impl;
use pubid_core::resolver::{resolve, resolver_table, resolver_version, ResolveError};

/// Every entry carries a source citation, matches on exactly one of
/// `type` / `type_prefix`, names a ported flavor, and its example
/// re-parses and re-renders to the attested URL.
#[test]
fn every_entry_is_attested_and_renders() {
    let table = resolver_table();
    assert!(!table.entries.is_empty(), "the mapping table is empty");
    for entry in &table.entries {
        assert!(
            !entry.source.trim().is_empty(),
            "entry {entry:?} carries no source citation"
        );
        assert!(
            entry.type_tag.is_some() ^ entry.type_prefix.is_some(),
            "entry for flavor {} must match on exactly one of type / type_prefix",
            entry.flavor
        );
        let Some(impl_) = flavor_impl(&entry.flavor) else {
            panic!("entry maps flavor {:?}, which is not ported", entry.flavor);
        };
        let identifier = impl_.parse(&entry.example.input).unwrap_or_else(|error| {
            panic!("example {:?} does not parse: {error}", entry.example.input)
        });
        let type_tag = identifier
            .to_hash()
            .get("_type")
            .and_then(serde_json::Value::as_str)
            .expect("parsed example carries a _type")
            .to_string();
        assert!(
            entry.matches(&type_tag),
            "entry for flavor {} does not match its own example's _type {type_tag}",
            entry.flavor
        );
        let resolved = resolve(&entry.example.input, Some(&entry.flavor)).unwrap_or_else(|error| {
            panic!(
                "example {:?} does not resolve: {error}",
                entry.example.input
            )
        });
        assert_eq!(resolved.url, entry.example.url, "canonical URL drifted");
        assert!(
            resolved.url.starts_with("https://"),
            "canonical URLs are https: {}",
            resolved.url
        );
    }
}

/// The table is versioned and carries its editorial notes.
#[test]
fn table_is_versioned() {
    assert_eq!(resolver_version(), "1.0.0");
    let table = resolver_table();
    assert!(!table.notes.is_empty(), "the table carries no notes");
}

/// The RFC entry renders the canonical RFC Editor URL, the info and
/// datatracker alternates, and the attested RFC DOI.
#[test]
fn rfc_resolves_with_doi_and_alternates() {
    let resolved = resolve("RFC 9110", Some("ietf")).expect("resolves");
    assert_eq!(resolved.url, "https://www.rfc-editor.org/rfc/rfc9110.html");
    assert_eq!(
        resolved.alternates,
        vec![
            "https://www.rfc-editor.org/info/rfc9110",
            "https://datatracker.ietf.org/doc/rfc9110/",
        ]
    );
    assert_eq!(resolved.doi.as_deref(), Some("10.17487/RFC9110"));
    assert_eq!(
        resolved.doi_url.as_deref(),
        Some("https://doi.org/10.17487/RFC9110")
    );
    assert_eq!(resolved.type_tag, "pubid:ietf:rfc");
}

/// The draft entry renders the revision-specific alternate only when
/// the input states a version.
#[test]
fn draft_revision_alternate_needs_a_version() {
    let with_version = resolve("draft-ietf-httpbis-messaging-19", Some("ietf")).expect("resolves");
    assert_eq!(
        with_version.url,
        "https://datatracker.ietf.org/doc/draft-ietf-httpbis-messaging/"
    );
    assert_eq!(
        with_version.alternates,
        vec!["https://datatracker.ietf.org/doc/draft-ietf-httpbis-messaging/19/"]
    );
    assert_eq!(with_version.doi, None);

    let without_version = resolve("draft-ietf-httpbis-messaging", Some("ietf")).expect("resolves");
    assert_eq!(
        without_version.url,
        "https://datatracker.ietf.org/doc/draft-ietf-httpbis-messaging/"
    );
    assert!(
        without_version.alternates.is_empty(),
        "no revision stated, no revision alternate"
    );
}

/// The arXiv id keeps the subject class; the DOI id drops it, per the
/// DOI line arXiv prints on the abs page.
#[test]
fn arxiv_id_and_doi_id() {
    let old = resolve("arXiv:math.GT/0309136", Some("arxiv")).expect("resolves");
    assert_eq!(old.url, "https://arxiv.org/abs/math.GT/0309136");
    assert_eq!(old.doi.as_deref(), Some("10.48550/arXiv.math/0309136"));

    let new = resolve("arXiv:0704.0001v1", Some("arxiv")).expect("resolves");
    assert_eq!(new.url, "https://arxiv.org/abs/0704.0001");
    assert_eq!(new.doi.as_deref(), Some("10.48550/arXiv.0704.0001"));
}

/// The EU ELI form per act kind, and the CELEX alternate with the
/// four-digit zero-padded number.
#[test]
fn eu_eli_and_celex_per_kind() {
    let reg = resolve("EU Regulation 1227/2011", Some("eu")).expect("resolves");
    assert_eq!(reg.url, "https://eur-lex.europa.eu/eli/reg/2011/1227/oj");
    assert_eq!(reg.doi, None);
    assert_eq!(
        reg.alternates,
        vec!["https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32011R1227"]
    );

    // Regulation 2016/679 exercises the padding: CELEX 32016R0679.
    let short = resolve("EU Regulation 2016/679", Some("eu")).expect("resolves");
    assert_eq!(short.url, "https://eur-lex.europa.eu/eli/reg/2016/679/oj");
    assert_eq!(
        short.alternates,
        vec!["https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32016R0679"]
    );

    let dir = resolve("EU Directive 2004/38", Some("eu")).expect("resolves");
    assert_eq!(dir.url, "https://eur-lex.europa.eu/eli/dir/2004/38/oj");
    assert_eq!(
        dir.alternates,
        vec!["https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32004L0038"]
    );
}

/// The IHO page slot follows the series letter per kind.
#[test]
fn iho_page_follows_the_series_letter() {
    let s = resolve("IHO S-411 2.0.0", Some("iho")).expect("resolves");
    assert_eq!(s.url, "https://iho.int/en/s-411");
    let b = resolve("IHO B-12 2.0.3", Some("iho")).expect("resolves");
    assert_eq!(b.url, "https://iho.int/en/b-12");
}

/// The W3C dated alternate renders only when the identifier carries a
/// date and its kind prints a maturity token.
#[test]
fn w3c_dated_alternate_is_conditional() {
    let dated = resolve("W3C CR-exi-20091208", Some("w3c")).expect("resolves");
    assert_eq!(dated.url, "https://www.w3.org/TR/exi/");
    assert_eq!(
        dated.alternates,
        vec!["https://www.w3.org/TR/2009/CR-exi-20091208/"]
    );
}

/// The UN symbol slot joins the path tokens with the trailing number.
#[test]
fn un_symbol_joins_the_path() {
    let resolved = resolve("A/RES/78/1", Some("un")).expect("resolves");
    assert_eq!(resolved.url, "https://docs.un.org/en/A/RES/78/1");
}

/// Detection resolves an unambiguous input without naming a flavor.
#[test]
fn detection_resolves_the_unambiguous_input() {
    let resolved = resolve("doi:10.1000/182", None).expect("resolves");
    assert_eq!(resolved.flavor, "doi");
    assert_eq!(resolved.url, "https://doi.org/10.1000/182");
    assert_eq!(resolved.alternates, vec!["https://dx.doi.org/10.1000/182"]);
}

/// A parsed identifier with no attested mapping is `Unmapped`, not a
/// parse failure.
#[test]
fn unmapped_flavor_is_unmapped() {
    let error = resolve("ISBN 978-0-306-40615-7", Some("isbn"))
        .expect_err("isbn carries no attested landing page");
    match error {
        ResolveError::Unmapped { flavor, type_tag } => {
            assert_eq!(flavor, "isbn");
            assert_eq!(type_tag, "pubid:isbn:book");
        }
        other => panic!("expected Unmapped, got {other:?}"),
    }
}

/// An input no flavor accepts is a rejection.
#[test]
fn rejected_input_is_rejected() {
    let error = resolve("this is not an identifier of any ported flavor", None)
        .expect_err("rejection expected");
    assert!(
        matches!(error, ResolveError::Rejected { .. }),
        "got {error}"
    );
}

/// An ambiguous input names the flavors and requires `--flavor`.
#[test]
fn ambiguous_input_names_the_flavors() {
    // `S-411 1.2.1` parses under both the wmo (S-100 spec) and the
    // iho (S-series) flavors — the canonical ambiguity case.
    let error = resolve("S-411 1.2.1", None).expect_err("ambiguity expected");
    match error {
        ResolveError::Ambiguous { flavors } => {
            assert!(flavors.contains(&"wmo".to_string()), "got {flavors:?}");
            assert!(flavors.contains(&"iho".to_string()), "got {flavors:?}");
            // Naming the flavor resolves the ambiguity: the wmo pick
            // parses (then unmapped — wmo carries no attested page),
            // the iho pick renders its series landing page.
            match resolve("S-411 1.2.1", Some("wmo")) {
                Err(ResolveError::Unmapped { flavor, .. }) => assert_eq!(flavor, "wmo"),
                other => panic!("expected Unmapped for the wmo pick, got {other:?}"),
            }
            let resolved = resolve("S-411 1.2.1", Some("iho")).expect("resolves");
            assert_eq!(resolved.url, "https://iho.int/en/s-411");
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

/// An unknown flavor name is its own error class.
#[test]
fn unknown_flavor_name_is_its_own_error() {
    let error = resolve("RFC 9110", Some("nosuchflavor")).expect_err("rejection expected");
    match error {
        ResolveError::UnknownFlavor { name } => {
            assert_eq!(name, "nosuchflavor");
        }
        other => panic!("expected UnknownFlavor, got {other:?}"),
    }
}
