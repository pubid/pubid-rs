# Playground example fixture provenance

`examples.json` beside this note is the playground example set: the
canonical rows of every vendored testsuite slice, distilled by the
conformance binary's `--examples` emitter (five gated positive rows
per flavor, in corpus file order, skipping the quarantined and
render-unreparseable rows). It is a **projection of the vendored
slices** — it carries no row of its own, so its provenance is exactly
the slices' provenance (see
`crates/pubid-core/fixtures/testsuite/VENDORED_FROM.md` and the pins
in `/pins.yaml`).

Regenerate after a slice refresh round with:

```
cargo run -q -p conformance -- --examples > crates/pubid-wasm/fixtures/examples.json
```

The `pubid_examples` gate test re-parses every emitted row through
its flavor and re-checks the canonical human and the URN, so a
drifted or stale row fails the suite rather than shipping to the
playground.
