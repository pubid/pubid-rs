//! The attested input generators: the seed universe mined from the
//! vendored fixture corpus, the facet templates, and the row mutator.
//!
//! Shared by the property suite (`tests/properties.rs`) and the
//! differential runner (`crates/differential`) so both layers compose
//! randomized inputs from the same attested patterns: literal text
//! stays verbatim, digit runs become slots harvesting every value the
//! skeleton's rows took, and mutations stay structurally near the rows.

use std::sync::OnceLock;

use crate::conformance::{implemented_flavors, vendored_corpus, CorpusCase};

/// Split a spelling into literal separators and digit-slot values:
/// each maximal digit run becomes one slot, the surrounding literals
/// become separators, and `seps.len() == slot_values.len() + 1` always
/// holds (empty strings at a leading/trailing boundary).
pub fn split_slots(spelling: &str) -> Option<(Vec<String>, Vec<String>)> {
    let mut seps: Vec<String> = Vec::new();
    let mut slot_values: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_slot = false;
    let mut has_slot = false;
    for ch in spelling.chars() {
        let digit = ch.is_ascii_digit();
        if digit == in_slot {
            current.push(ch);
        } else if in_slot {
            slot_values.push(std::mem::take(&mut current));
            current.push(ch);
            in_slot = false;
        } else {
            seps.push(std::mem::take(&mut current));
            current.push(ch);
            in_slot = true;
            has_slot = true;
        }
    }
    if in_slot {
        slot_values.push(std::mem::take(&mut current));
        seps.push(String::new());
    } else {
        seps.push(std::mem::take(&mut current));
    }
    has_slot.then_some((seps, slot_values))
}

/// Facet slots harvested from one attested skeleton: the literal text
/// stays verbatim; each maximal digit run becomes a slot holding every
/// value that run took across the skeleton's rows.
#[derive(Debug, Clone)]
pub struct FacetTemplate {
    /// Literal separators; `seps.len() == slots.len() + 1`.
    pub seps: Vec<String>,
    /// Per-slot harvested values (non-empty each).
    pub slots: Vec<Vec<String>>,
}

impl FacetTemplate {
    /// The skeleton key: separators joined with a slot marker. Two
    /// spellings share a skeleton when their literal separators are
    /// identical and their digit runs sit at the same positions.
    fn key(seps: &[String]) -> String {
        seps.join("\u{1}")
    }

    /// Mine the templates of one spelling pool. Every spelling with at
    /// least one digit run lands in exactly one skeleton; its slots
    /// harvest that skeleton's member values.
    pub fn mine(pool: &[String]) -> Vec<Self> {
        // Skeleton key -> (separators, per-slot value sets). The first
        // member fixes the skeleton's literal separators.
        let mut mined: Vec<(String, Vec<String>, Vec<Vec<String>>)> = Vec::new();
        for spelling in pool {
            let Some((seps, slot_values)) = split_slots(spelling) else {
                continue;
            };
            let key = Self::key(&seps);
            let entry = mined.iter_mut().find(|(key_, _, _)| *key_ == key);
            match entry {
                Some((_, _, slots)) => {
                    for (slot, value) in slots.iter_mut().zip(slot_values) {
                        if !slot.contains(&value) {
                            slot.push(value);
                        }
                    }
                }
                None => mined.push((
                    key,
                    seps,
                    slot_values.into_iter().map(|v| vec![v]).collect(),
                )),
            }
        }
        mined
            .into_iter()
            .map(|(_, seps, slots)| FacetTemplate { seps, slots })
            .collect()
    }

    /// A splice of the template: slot `i` draws `picks[i] % len` of its
    /// harvested values. Out-of-range or short `picks` fall back to the
    /// first value, so callers need no modulo discipline.
    pub fn render(&self, picks: &[usize]) -> String {
        let mut input = String::new();
        input.push_str(&self.seps[0]);
        for (i, values) in self.slots.iter().enumerate() {
            let pick = picks.get(i).copied().unwrap_or(0);
            input.push_str(&values[pick % values.len()]);
            input.push_str(&self.seps[i + 1]);
        }
        input
    }
}

/// One alias spelling: the input, its documented normalization depth,
/// and the canonical spelling the chain converges to.
#[derive(Debug, Clone)]
pub struct AliasSpelling {
    pub spelling: String,
    pub rounds: u32,
    pub canonical: String,
}

/// One flavor's attested seed material.
#[derive(Debug, Clone)]
pub struct FlavorUniverse {
    pub name: &'static str,
    /// Whether the flavor's ledger is clean. A ledgered flavor's alias
    /// chains may sit on its documented known mismatches, so they are
    /// sampled only for clean flavors.
    pub clean: bool,
    /// Canonical humans of the gated positive rows.
    pub rows: Vec<String>,
    /// Alias spellings with their documented normalization rounds.
    pub aliases: Vec<AliasSpelling>,
    /// Templates mined from `rows`.
    pub templates: Vec<FacetTemplate>,
    /// Every literal token of `rows`, for mutation splices.
    pub tokens: Vec<String>,
}

/// The whole seed universe: one entry per ported flavor with a vendored
/// slice, mined once per process.
#[derive(Debug)]
pub struct Universe {
    pub flavors: Vec<FlavorUniverse>,
}

/// The gated positive cases (the rows conformance checks), per the
/// checker's own skip rules.
fn gated_cases(corpus: &[CorpusCase]) -> Vec<&CorpusCase> {
    corpus
        .iter()
        .filter(|case| {
            case.review.is_none()
                && !case.expect_error
                && !case.quarantined
                && !case.render_unreparseable
                && case.human.is_some()
        })
        .collect()
}

fn build_universe() -> Universe {
    let mut flavors = Vec::new();
    for name in implemented_flavors() {
        let Some(Ok(corpus)) = vendored_corpus(name) else {
            continue;
        };
        let mut rows = Vec::new();
        let mut aliases = Vec::new();
        for case in gated_cases(&corpus.positives) {
            let Some(human) = &case.human else {
                continue;
            };
            rows.push(human.clone());
            for alias in &case.aliases {
                aliases.push(AliasSpelling {
                    spelling: alias.spelling.clone(),
                    rounds: alias.rounds,
                    canonical: human.clone(),
                });
            }
        }
        let clean = corpus.status.map(|status| status.clean).unwrap_or(true);
        let templates = FacetTemplate::mine(&rows);
        let mut tokens: Vec<String> = Vec::new();
        for row in &rows {
            for part in row.split(|ch: char| ch.is_ascii_digit() || ch.is_whitespace()) {
                if !part.is_empty() && !tokens.iter().any(|token| token == part) {
                    tokens.push(part.to_string());
                }
            }
        }
        flavors.push(FlavorUniverse {
            name,
            clean,
            rows,
            aliases,
            templates,
            tokens,
        });
    }
    Universe { flavors }
}

/// The seed universe, mined once per process from the vendored slices.
pub fn attested_universe() -> &'static Universe {
    static UNIVERSE: OnceLock<Universe> = OnceLock::new();
    UNIVERSE.get_or_init(build_universe)
}

/// The row mutator: one structural edit at a byte-ish position (clamped
/// to char boundaries by construction, since the edit composes from
/// `chars`). `op % 8` picks the edit: insert, delete, substitute,
/// truncate, duplicate, token-splice, upper-case run, junk separator.
pub fn mutate_row(
    input: &str,
    op: usize,
    position: usize,
    ch: char,
    token_pick: usize,
    tokens: &[String],
) -> String {
    let chars: Vec<char> = input.chars().collect();
    let at = position % (chars.len() + 1);
    let mut edited: String = chars[..at].iter().collect();
    match op % 8 {
        0 => {
            edited.push(ch);
            edited.extend(&chars[at..]);
        }
        1 => edited.extend(&chars[(at + 1).min(chars.len())..]),
        2 => {
            edited.push(ch);
            edited.extend(&chars[(at + 1).min(chars.len())..]);
        }
        3 => edited.clear(),
        4 => {
            let end = (at + (position % 4) + 1).min(chars.len());
            edited.extend(&chars[at..end]);
            edited.extend(&chars[at..]);
        }
        5 => {
            if !tokens.is_empty() {
                edited.push_str(&tokens[token_pick % tokens.len()]);
            }
            edited.extend(&chars[at..]);
        }
        6 => {
            edited.extend(chars[at..].iter().map(|c| {
                if c.is_ascii_alphabetic() {
                    c.to_ascii_uppercase()
                } else {
                    *c
                }
            }));
        }
        _ => {
            edited.push_str(":::-");
            edited.extend(&chars[at..]);
        }
    }
    edited
}

#[cfg(test)]
mod tests {
    use super::{attested_universe, mutate_row, split_slots, FacetTemplate};

    #[test]
    fn split_keeps_the_seps_slots_invariant() {
        let (seps, slots) = split_slots("ETSI EN 300 010-1 V1.1.1 (1997-04)").expect("slots");
        assert_eq!(seps.len(), slots.len() + 1);
        let template = FacetTemplate {
            slots: slots.into_iter().map(|value| vec![value]).collect(),
            seps,
        };
        // Slot picks all zero render the template's first attested member.
        let rendered = template.render(&vec![0; template.slots.len()]);
        assert!(rendered.contains("ETSI EN"), "{rendered}");
        assert!(rendered.contains("V1.1.1"), "{rendered}");
        // A mined template reproduces its source spelling.
        assert_eq!(
            FacetTemplate::mine(std::slice::from_ref(&rendered))[0].render(&[0; 8]),
            rendered
        );
    }

    #[test]
    fn a_digitless_spelling_has_no_slots() {
        assert!(split_slots("ETSI guide").is_none());
    }

    #[test]
    fn mutation_ops_all_terminate() {
        let tokens = vec!["EN".to_string()];
        for op in 0..16 {
            let edited = mutate_row("ETSI EN 300 010", op, 3, 'x', 0, &tokens);
            assert!(edited.len() <= "ETSI EN 300 010".len() * 3 + 8);
        }
    }

    #[test]
    fn the_universe_carries_every_vendored_flavor() {
        let universe = attested_universe();
        assert!(universe.flavors.iter().any(|flavor| flavor.name == "etsi"));
        let etsi = universe
            .flavors
            .iter()
            .find(|flavor| flavor.name == "etsi")
            .expect("etsi");
        assert!(etsi.rows.len() > 1_000);
        assert!(!etsi.templates.is_empty());
    }
}
