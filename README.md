# pubid-rs

Rust implementation of PubID: resolution, normalization and canonical
hashes for standards identifiers, running on the same engine as the
rest of the fleet. Parsers are **baked PARG artifacts** (checksummed
JSON envelopes from [pubid-grammar]) consumed through
[parsanol-rs](https://crates.io/crates/parsanol) — no grammar code
lives here, and a corrupt or tampered artifact refuses to load.

[pubid-grammar]: https://github.com/pubid/pubid-grammar

## Scope

This repository is the **pubid-rs track**: engine-backed flavors
growing toward fleet parity with the Ruby pubid gem and pubid-ts. The
bootstrap wave stood up the pipeline end to end with the smallest
flavor; wave 2 ported the small render-layer flavors onto that
pattern; later waves port the mid fleet and the hard core until the
corpus runs fleet-wide.

- `pubid-core` — artifact loading (embedded, checksum-verified),
  flavor semantics (canonical hash / human / URN), the conformance
  checker, and the schema-driven materialization boundary
  (`materialize_from_schema`).
- `pubid-cli` — `pubid-cli parse <flavor> <input>` prints the canonical
  forms; `pubid-cli smoke` runs every vendored slice.
- `conformance` — the pubid-testsuite runner: load a flavor's corpus,
  parse → render → compare, report the measured counts, gate on clean
  flavors only. `--all` runs every ported flavor.

## Layout

```
crates/pubid-core/
  src/           lib, one module per flavor, conformance checker, errors
  artifacts/     vendored PARG artifact(s)   (+ VENDORED_FROM.md)
  fixtures/      vendored testsuite slice(s) (+ VENDORED_FROM.md)
crates/pubid-cli/    the `pubid-cli` binary
crates/conformance/  the `conformance` runner binary
pins.yaml            pinned testsuite + grammar refs (single source)
.github/workflows/   CI
```

## Conformance status (waves 1–3, measured)

Seventeen flavors are ported end to end: `doi` (bootstrap), the
wave-2 render-layer flavors `isbn`, `iana`, `oasis`, `xsf`, `un`,
`ogc`, `ietf`, `w3c`, and the wave-3 mid-fleet flavors `api`, `amca`,
`nesc`, `ire`, `ansi`, `ecma`, `gb`, `gost`. Measured against the full
pubid-testsuite corpora at the pinned ref — every gated case passes
except the one api ledger row (below), nothing hidden:

| flavor | gated cases | parsed | rejected (negatives) | rendered | hashes | urns | deserializes |
| ------ | ----------: | -----: | -------------------: | -------: | -----: | ---: | -----------: |
| doi    | 3           | 6      | 4                    | 6        | 3      | 3    | 3            |
| isbn   | 3           | 5      | 4                    | 5        | 3      | 3    | 3            |
| iana   | 30          | 30     | 5                    | 30       | 30     | 30   | 30           |
| oasis  | 45          | 45     | 0                    | 45       | 45     | 45   | 45           |
| xsf    | 51          | 51     | 0                    | 51       | 51     | 51   | 51           |
| un     | 4           | 5      | 5                    | 5        | 4      | 4    | 4            |
| ogc    | 88          | 88     | 6                    | 88       | 88     | 88   | 88           |
| ietf   | 2,526       | 2,526  | 0                    | 2,526    | 2,526  | 2,526| 2,526        |
| w3c    | 29          | 29     | 0                    | 29       | 29     | 29   | 29           |
| api    | 193         | 196    | 0                    | 196      | 193    | 193  | 193          |
| amca   | 39          | 49     | 4                    | 49       | 39     | 39   | 39           |
| nesc   | 11          | 19     | 0                    | 19       | 11     | 11   | 11           |
| ire    | 1           | 1      | 0                    | 1        | 1      | 1    | 1            |
| ansi   | 175         | 175    | 0                    | 175      | 175    | 175  | 175          |
| ecma   | 833         | 839    | 0                    | 839      | 833    | 833  | 833          |
| gb     | 10          | 10     | 5                    | 10       | 10     | 10   | 10           |
| gost   | 199         | 399    | 0                    | 399      | 199    | 199  | 199          |
| **all**| **4,240**   | 4,473  | 33                   | 4,473    | 4,240  | 4,240| 4,240        |

Counts: `parsed` covers the canonical humans plus every non-normalized
alias spelling; `rejected` covers the `_negative.yaml` rows (plus the
doi/isbn builder-layer rejections the flavors enforce); `rendered`
covers every human compared byte-for-byte; `hashes`/`urns` the deep
comparisons against the corpus; `deserializes` the
`from_hash(to_hash())` round-trips. All ledgers except `api` are
`clean: true`, so every flavor gates; the api ledger documents one
known alias mismatch (the slash-combined `API COS 1-07/RP 75` chain),
under its ceiling, so the run exits green. The embedded artifact
suites (the envelopes' own accept/reject tests) are green for every
vendored envelope.

That is 4,240 of the testsuite's 99,793 cases. The remaining mid
fleet is reported honestly, never claimed. `adobe` carries no
testsuite corpus, so its vendored artifact is carried without a ported
flavor (the runner reports it as vendored-not-ported rather than
gating on invented ground truth).

CI additionally runs the conformance binary against a full clone of
pubid-testsuite at the pinned ref, not just the vendored slices.

## Pins

`pins.yaml` is the single source (same pattern as pubid-ts):

- `testsuite_ref` — pubid-testsuite commit the vendored fixture slices
  were byte-copied from (`crates/pubid-core/fixtures/testsuite/`,
  provenance + per-file sha256s in `VENDORED_FROM.md`).
- `grammar_ref` — pubid-grammar commit the vendored artifacts were
  byte-copied from (`crates/pubid-core/artifacts/`, provenance +
  per-file sha256s in its `VENDORED_FROM.md`).

CI checks out both refs and byte-compares (`cmp`) every vendored copy;
any drift fails the build. Refreshing means re-copying the files from
the new ref and updating `pins.yaml` + the `VENDORED_FROM.md` notes
together.

The pinned grammar ref is the parsanol 1.5.1.0 re-seal round (#129
stamp normalization): the envelopes are byte-identical grammars
modulo the producer stamp (`doi` carries the same `semantics_hash` as
the bootstrap artifact), and all nine load and self-test green under
the locked `parsanol = "0.20"` runtime.

## Track plan

1. **Done — bootstrap**: workspace, vendored-artifact loading, doi
   end to end (parse → hash → human → URN → aliases → deserialize),
   conformance runner, CI with drift gates.
2. **Done — wave 2, small render-layer flavors**: isbn, iana, oasis,
   xsf, un, ogc, ietf, w3c — per-flavor render functions over the
   bound hash, builder casts (isbn check digit, iana `registry` →
   `number`, oasis slug decomposition, ogc revision case, ietf pad
   strip + draft version split, w3c date split), URN templates,
   multi-type dispatch (ietf 5, w3c 11). 100% on every corpus.
3. **Wave 3 — the mid fleet**: etsi, gb, gost, ccsds, plateau, ecma,
   ansi, adobe, iala, ire, sae, api, amca, nesc, jcgm, idf,
   cen_cenelec, evs, jis (5–50 bindings), plus doi_pfs/calconnect_pfs.
4. **Wave 4 — the hard core** (largest artifacts, staged by binding
   count): astm, asme, bipm, oiml, csa, iho, itu, cie, iec, ashrae,
   bsi, nist, iso (the only artifact-level render spec in the fleet),
   ieee (122 bindings).

Flavor order follows the fleet's port-hard-first rule where practical;
small flavors ride the established pattern. A wave lands only when its
flavors pass their corpus conformance cleanly.

## Building

```
cargo build --all-targets
cargo test
cargo run -p conformance -- --all
cargo run -p conformance -- --flavor ietf --testsuite <checkout>/tests
cargo run -p pubid-cli -- parse w3c "W3C REC-html40-19980424"
cargo run -p pubid-cli -- smoke
```

`cargo test` needs no network: the artifacts and corpus slices are
vendored. To run a flavor against a full pubid-testsuite checkout:

```
cargo run -p conformance -- --testsuite <checkout>/tests --all
```

## License

MIT — see [LICENSE](LICENSE).
