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
  flavors only. `--all` runs every ported flavor. `--examples` emits
  the corpus-fed example set (the JSON the wasm bundle embeds).
- `pubid-wasm` — the WebAssembly target: the pubid-core flavor layer
  compiled to wasm for the pubid.github.io playground.

## The wasm target (crates/pubid-wasm)

`pubid-wasm` compiles pubid-core + the grammar-artifact runtime to
wasm-bindgen/wasm-pack for the site playground — a thin JS-facing shim
over the same checksum-verified artifacts, no grammar logic of its
own. The JS surface mirrors the Rust API:

| export | the Rust mirror |
| ------ | --------------- |
| `pubid_version()` | the crate version |
| `pubid_flavors()` | `implemented_flavors()` |
| `pubid_parse(flavor, input)` | `FlavorImpl::parse` + the canonical forms (throws on rejection) |
| `pubid_try_parse(flavor, input)` | the same parse as a `{ ok, result?, error?, code? }` verdict (never throws) |
| `pubid_detect(input)` | every ported flavor that accepts the input |
| `pubid_artifacts()` | the vendored-artifact receipts (checksum / semantics hash, by flavor) |
| `pubid_examples()` | the corpus-fed example rows (vendored slices, distilled) |

The parse report carries the typed tree (the canonical `_type`-tagged
attribute hash), the canonical human, and the URN with its namespace
class (`urn` for the RFC 8141 forms, `mrn` for the
`urn:mrn:` guidance forms — IHO/IALA/WMO). The verdict `code` classes
are `unknown_flavor`, `parse`, `invalid`, `artifact`.

The grammar artifacts are **bundled**, not fetched: the ported
flavors' envelopes total a few hundred kilobytes of JSON, so the
playground stays a fully client-side, zero-round-trip parse; the
envelopes are the vendored checksummed ones from `pubid-core`
(`/pins.yaml`), and the 11 MB corpus fixtures stay out of the bundle
(the linker GCs the unreferenced slice data; only the distilled
`examples.json` rides along). Release bundle at `wasm-opt -O3`: ~2.6 MB
raw, ~850 KB gzipped.

Build and test:

```
wasm-pack build crates/pubid-wasm --target bundler --release --out-dir pkg
wasm-pack test --node crates/pubid-wasm
```

The `pubid_examples` gate test re-parses every example row through its
flavor, so a slice refresh that moves a row fails the suite.

## Layout

```
crates/pubid-core/
  src/           lib, one module per flavor, conformance checker, errors
  artifacts/     vendored PARG artifact(s)   (+ VENDORED_FROM.md)
  fixtures/      vendored testsuite slice(s) (+ VENDORED_FROM.md)
crates/pubid-cli/    the `pubid-cli` binary
crates/conformance/  the `conformance` runner binary
crates/pubid-wasm/   the wasm target (built pkg/ is gitignored)
pins.yaml            pinned testsuite + grammar refs (single source)
.github/workflows/   CI
```

## Conformance status (waves 1–3, measured)

Twenty-six flavors are ported: `doi` (bootstrap), the
wave-2 render-layer flavors `isbn`, `iana`, `oasis`, `xsf`, `un`,
`ogc`, `ietf`, `w3c`, the wave-3 mid-fleet flavors `api`, `amca`,
`nesc`, `ire`, `ansi`, `ecma`, `gb`, `gost`, the wave-3 tier
`plateau`, `evs`, `ccsds`, `cen_cenelec`, `jis`, `etsi` (the
big-corpus flavors — `etsi` alone is 24,718 rows), and the new-flavor
round `eu`, `arxiv` (pubid#752/#753, the EU legal acts and the arXiv
e-prints), and the S-158 round `iho` (pubid#774/#775 — the flavor is
ported with the `checks_for` series model and the migrated
`urn:mrn:iho:pub:spec:` emission; its corpus slice waits on the
testsuite re-key round, so the vendored gate reports it as
ported-no-slice). Measured against the full pubid-testsuite corpora at
the pinned ref — every gated case passes except the one api ledger row
(below), nothing hidden. The `eu`/`arxiv` rows measure their
gem-fixture-derived vendored slices, the only corpora the two flavors
have until the testsuite exports them (see the fixtures provenance
note):

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
| plateau| 118         | 119    | 0                    | 119      | 118    | 118  | 118          |
| evs    | 35          | 36     | 0                    | 36       | 35     | 35   | 35           |
| ccsds  | 490         | 490    | 0                    | 490      | 490    | 487  | 490          |
| cen_cenelec | 189    | 199    | 14                   | 199      | 189    | 189  | 189          |
| jis    | 10,575      | 10,576 | 0                    | 10,576   | 10,575 | 10,575 | 10,575     |
| etsi   | 24,718      | 24,718 | 0                    | 24,718   | 24,718 | 21,432 | 24,718     |
| eu     | 11          | 27     | 6                    | 27       | 11     | 11   | 11           |
| arxiv  | 14          | 21     | 15                   | 21       | 14     | 14   | 14           |
| **all**| **40,390**  | 40,659 | 68                   | 40,659   | 40,390 | 37,101 | 40,390   |

Counts: `parsed` covers the canonical humans plus every non-normalized
alias spelling; `rejected` covers the `_negative.yaml` rows (plus the
doi/isbn builder-layer rejections the flavors enforce); `rendered`
covers every human compared byte-for-byte; `hashes`/`urns` the deep
comparisons against the corpus (a flavor emits no URN where the
reference emits none — the ccsds language rows the reference's own
`to_urn` raises on, and the etsi rows whose exporter omitted the
representation); `deserializes` the `from_hash(to_hash())`
round-trips. All ledgers except `api` are `clean: true`, so every
flavor gates; the api ledger documents one known alias mismatch (the
slash-combined `API COS 1-07/RP 75` chain), under its ceiling, so the
run exits green. The embedded artifact suites (the envelopes' own
accept/reject tests) are green for every vendored envelope.

That is 40,365 of the testsuite's cases — the six flavors this tier
adds carry 36,125 of them. The remaining mid fleet is reported
honestly, never claimed. `adobe` and the five `_pfs` envelopes
(`doi_pfs`, `calconnect_pfs`, `cen_pfs`, `etsi_pfs`, `bsi_pfs`) carry
no testsuite corpus at the pinned ref, so their artifacts are vendored
without ported flavors (the runner reports them as vendored-not-ported
rather than gating on invented ground truth).

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
