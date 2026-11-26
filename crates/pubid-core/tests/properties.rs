//! Property tests: `parse(render(x)) == x` over the attested corpus and
//! over randomized facet recombinations mined from it.
//!
//! Three layers, all seeded by [`pubid_core::generators::attested_universe`]
//! (the same rows the conformance gate checks):
//!
//! 1. The attested universe — sampled canonical rows must be fixed
//!    points: parse, render byte-identically, re-parse to the same
//!    canonical hash, and the deserialize closure holds.
//! 2. The randomized composition layer — facet templates mined from the
//!    corpus (digit slots harvested per attested skeleton, literals kept
//!    verbatim), recombined at random. A generated input may legitimately
//!    fail to parse (not every cross-product is a real identifier), but
//!    every input that DOES parse must land on a fixed point: rendering
//!    and re-parsing it never changes the canonical hash.
//! 3. The robustness layer — mutated corpus rows (insertions, deletions,
//!    substitutions, truncations, token splices, case flips) must never
//!    panic; when a mutant parses, the closure still holds.
//!
//! The case budget comes from `PUBID_PROPERTY_CASES` (default 64 per
//! property); CI runs the suite with a larger budget in its dedicated
//! property job. `PROPTEST_SEED`/`PROPTEST_CASES` stay available for
//! replaying a proptest failure.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use pubid_core::conformance::{flavor_impl, Identifier};
use pubid_core::generators::{attested_universe, mutate_row, AliasSpelling, FlavorUniverse};

/// A flavored input under test.
#[derive(Clone, Debug)]
struct Case {
    flavor: &'static str,
    input: String,
    /// Parse -> render chains before the fixed-point check (the alias
    /// rows' documented normalization depth; 1 everywhere else).
    rounds: u32,
    /// The canonical spelling an alias chain converges to.
    expect: Option<String>,
}

fn pick<T>(items: &[T], index: usize) -> Option<&T> {
    (!items.is_empty()).then(|| &items[index % items.len()])
}

fn flavor_universe(pick_index: usize) -> Option<&'static FlavorUniverse> {
    pick(&attested_universe().flavors, pick_index)
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// A sampled canonical row of any ported flavor.
fn row_case() -> BoxedStrategy<Case> {
    (any::<usize>(), any::<usize>())
        .prop_filter_map("a flavor with rows", |(flavor_pick, row_pick)| {
            let flavor = flavor_universe(flavor_pick)?;
            let row = pick(&flavor.rows, row_pick)?;
            Some(Case {
                flavor: flavor.name,
                input: row.clone(),
                rounds: 1,
                expect: None,
            })
        })
        .boxed()
}

/// A sampled alias spelling of any clean ported flavor. A ledgered
/// flavor's alias chain may sit on its documented known mismatches —
/// recorded debt, not a property the suite gates.
fn alias_case() -> BoxedStrategy<Case> {
    (any::<usize>(), any::<usize>())
        .prop_filter_map(
            "a clean flavor with aliases",
            |(flavor_pick, alias_pick)| {
                let flavor = flavor_universe(flavor_pick)?;
                if !flavor.clean {
                    return None;
                }
                let alias: &AliasSpelling = pick(&flavor.aliases, alias_pick)?;
                Some(Case {
                    flavor: flavor.name,
                    input: alias.spelling.clone(),
                    rounds: alias.rounds,
                    expect: Some(alias.canonical.clone()),
                })
            },
        )
        .boxed()
}

/// A randomized facet recombination: one mined template of any ported
/// flavor, each digit slot filled from its harvested values.
fn recombination_case() -> BoxedStrategy<Case> {
    (any::<usize>(), any::<usize>(), any::<Vec<usize>>())
        .prop_filter_map(
            "a flavor with templates",
            |(flavor_pick, template_pick, slot_picks)| {
                let flavor = flavor_universe(flavor_pick)?;
                let template = pick(&flavor.templates, template_pick)?;
                Some(Case {
                    flavor: flavor.name,
                    input: template.render(&slot_picks),
                    rounds: 1,
                    expect: None,
                })
            },
        )
        .boxed()
}

/// A mutated corpus row: the base row with one structural edit applied.
fn mutation_case() -> BoxedStrategy<Case> {
    (
        row_case(),
        any::<usize>(),
        any::<usize>(),
        any::<char>(),
        any::<usize>(),
    )
        .prop_filter_map(
            "a flavor with tokens",
            |(base, op, position, ch, token_pick)| {
                let flavor = attested_universe()
                    .flavors
                    .iter()
                    .find(|flavor| flavor.name == base.flavor)?;
                Some(Case {
                    flavor: base.flavor,
                    input: mutate_row(&base.input, op, position, ch, token_pick, &flavor.tokens),
                    rounds: 1,
                    expect: None,
                })
            },
        )
        .boxed()
}

// ---------------------------------------------------------------------------
// The properties
// ---------------------------------------------------------------------------

fn cases() -> u32 {
    std::env::var("PUBID_PROPERTY_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(64)
}

/// Parse a case, mapping every failure to a shrinking test failure.
fn parse_case(case: &Case) -> Result<Identifier, TestCaseError> {
    let impl_ = flavor_impl(case.flavor).ok_or_else(|| {
        TestCaseError::Fail(format!("flavor {} is not ported", case.flavor).into())
    })?;
    impl_.parse(&case.input).map_err(|err| {
        TestCaseError::Fail(format!("{} {:?}: {err}", case.flavor, case.input).into())
    })
}

/// Parse an input and assert the endpoint is a fixed point: rendering
/// it re-parses to the same canonical hash, and the hash survives the
/// deserialize closure.
fn fixed_point_at(flavor: &'static str, input: &str) -> Result<Identifier, TestCaseError> {
    let impl_ = flavor_impl(flavor)
        .ok_or_else(|| TestCaseError::Fail(format!("flavor {flavor} is not ported").into()))?;
    let identifier = impl_
        .parse(input)
        .map_err(|err| TestCaseError::Fail(format!("{flavor} {input:?}: {err}").into()))?;
    let rendered = identifier.to_human();
    let reparsed = impl_
        .parse(&rendered)
        .map_err(|err| TestCaseError::Fail(format!("{flavor} {rendered:?}: {err}").into()))?;
    prop_assert_eq!(
        &reparsed.to_hash(),
        &identifier.to_hash(),
        "{}: closure at {:?}",
        flavor,
        rendered
    );
    prop_assert_eq!(
        &rendered,
        input,
        "{}: fixed point at {:?}",
        flavor,
        rendered
    );
    let hash = identifier.to_hash();
    let rebuilt = Identifier::from_hash(&hash)
        .map_err(|err| TestCaseError::Fail(format!("{flavor}: from_hash: {err}").into()))?;
    prop_assert_eq!(&rebuilt.to_hash(), &hash, "{}: deserialize closure", flavor);
    Ok(identifier)
}

proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: cases(),
        max_global_rejects: 4096,
        ..proptest::test_runner::Config::default()
    })]

    /// A sampled canonical row is a fixed point: it parses, renders back
    /// byte-identically, re-parses to the same canonical hash, and the
    /// hash survives the deserialize closure.
    #[test]
    fn canonical_row_is_a_fixed_point(case in row_case()) {
        let identifier = parse_case(&case)?;
        prop_assert_eq!(&identifier.to_human(), &case.input, "canonical render");
        fixed_point_at(case.flavor, &identifier.to_human())?;
    }

    /// A facet recombination that parses lands on a fixed point:
    /// rendering and re-parsing it never changes the canonical hash.
    #[test]
    fn recombination_closes(case in recombination_case()) {
        let impl_ = flavor_impl(case.flavor).expect("ported");
        if let Ok(identifier) = impl_.parse(&case.input) {
            let rendered = identifier.to_human();
            let reparsed = impl_
                .parse(&rendered)
                .map_err(|err| TestCaseError::Fail(format!("{} {rendered:?}: {err}", case.flavor).into()))?;
            prop_assert_eq!(&reparsed.to_hash(), &identifier.to_hash(), "recombination closure");
        }
    }

    /// A sampled alias spelling normalizes, through its documented
    /// rounds, onto the canonical fixed point.
    #[test]
    fn alias_normalizes(case in alias_case()) {
        let impl_ = flavor_impl(case.flavor)
            .ok_or_else(|| TestCaseError::Fail(format!("flavor {} is not ported", case.flavor).into()))?;
        let mut input = case.input.clone();
        for round in 0..case.rounds.max(1) {
            let identifier = impl_.parse(&input).map_err(|err| {
                TestCaseError::Fail(
                    format!("{} {input:?} (round {round}): {err}", case.flavor).into(),
                )
            })?;
            input = identifier.to_human();
        }
        if let Some(canonical) = &case.expect {
            prop_assert_eq!(&input, canonical, "alias converges to the canonical spelling");
        }
        fixed_point_at(case.flavor, &input)?;
    }

    /// A mutated row never panics; when the mutant parses, the closure
    /// still holds (parse -> render -> parse keeps one hash).
    #[test]
    fn mutation_never_panics(case in mutation_case()) {
        let impl_ = flavor_impl(case.flavor).expect("ported");
        if let Ok(identifier) = impl_.parse(&case.input) {
            let rendered = identifier.to_human();
            let reparsed = impl_
                .parse(&rendered)
                .map_err(|err| TestCaseError::Fail(format!("{} {rendered:?}: {err}", case.flavor).into()))?;
            prop_assert_eq!(&reparsed.to_hash(), &identifier.to_hash(), "mutant closure");
        }
    }
}
