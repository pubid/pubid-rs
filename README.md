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
bootstrap wave (this one) stands up the pipeline end to end with the
smallest flavor; later waves port flavors onto the established pattern
until the corpus runs fleet-wide.

- `pubid-core` — artifact loading (embedded, checksum-verified),
  flavor semantics (canonical hash / human / URN), the conformance
  checker, and the schema-driven materialization boundary
  (`materialize_from_schema`).
- `pubid-cli` — `pubid-cli parse doi <input>` prints the canonical
  forms; `pubid-cli smoke` runs the self-contained smoke.
- `conformance` — the pubid-testsuite runner: load a flavor's corpus,
  parse → render → compare, report the measured counts, gate on clean
  flavors only.

## Layout

```
crates/pubid-core/
  src/           lib, doi flavor, conformance checker, errors
  artifacts/     vendored PARG artifact(s)   (+ VENDORED_FROM.md)
  fixtures/      vendored testsuite slice(s) (+ VENDORED_FROM.md)
crates/pubid-cli/    the `pubid-cli` binary
crates/conformance/  the `conformance` runner binary
pins.yaml            pinned testsuite + grammar refs (single source)
.github/workflows/   CI
```

## Conformance status (bootstrap wave, measured)

Flavor `doi` is ported end to end. Measured on the vendored smoke
slice (byte-copied from the pinned testsuite ref):

```
flavor doi outcome pass
  rows: 3 gated cases, 0 expected-error, 0 quarantined, 0 review, 0 reclassified
  measured: 6 parsed, 4 rejected, 6 rendered, 3 hashes, 3 urns, 3 deserializes
```

Every gated doi corpus case passes: canonical hash, human, URN, all
non-normalized alias spellings, and the deserialize round-trip. The
embedded artifact suite (4 accept/reject/example tests baked into the
envelope) is green. doi is 3 of the testsuite's 99,793 cases — parity
at fleet scale is the later waves' work, and unimplemented flavors are
reported honestly, never claimed.

CI additionally runs the conformance binary against a full clone of
pubid-testsuite at the pinned ref, not just the vendored slice.

## Pins

`pins.yaml` is the single source (same pattern as pubid-ts):

- `testsuite_ref` — pubid-testsuite commit the vendored fixture slice
  was byte-copied from (`crates/pubid-core/fixtures/testsuite/doi/`,
  provenance in its `VENDORED_FROM.md`).
- `grammar_ref` — pubid-grammar commit the vendored artifact was
  byte-copied from (`crates/pubid-core/artifacts/doi.json`, provenance
  in its `VENDORED_FROM.md`).

CI checks out both refs and byte-compares (`cmp`) every vendored copy;
any drift fails the build. Refreshing means re-copying the files from
the new ref and updating `pins.yaml` + the `VENDORED_FROM.md` notes
together.

## Engine

- `parsanol = "0.20"` (the artifact runtime: `PargArtifact::from_json`
  verifies the envelope checksum, `parse_and_bind` produces the bound
  hash, `run_tests` replays the embedded suite). The exact resolution
  is locked in `Cargo.lock`.

## Track plan

1. **Done — bootstrap**: workspace, vendored-artifact loading, doi
   end to end (parse → hash → human → URN → aliases → deserialize),
   conformance runner, CI with drift gates.
2. **Wave 2 — small render-layer flavors**: isbn, iana, oasis, xsf,
   un, ogc, ietf, w3c (1–3 bindings each) to harden the flavor
   pattern: per-flavor render functions, builder casts like doi's
   `10.` prefix, URN templates.
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
cargo run -p conformance -- --flavor doi
cargo run -p pubid-cli -- parse doi "10.1000/182"
```

`cargo test` needs no network: the artifact and corpus slice are
vendored. To run a flavor against a full pubid-testsuite checkout:

```
cargo run -p conformance -- --testsuite <checkout>/tests --flavor doi
```

## License

MIT — see [LICENSE](LICENSE).
