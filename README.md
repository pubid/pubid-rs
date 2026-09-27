# pubid-rs

Rust PubID models over PG artifacts (TODO 12 / RS1–RS2). **Local
scaffold — pushing to github.com/pubid/pubid-rs and publishing are
owner decisions, deliberately not taken.**

- RS1: schema-driven materialization (`pubid_rs::materialize_from_schema`,
  `Pubid::materialize`) over `parsanol::pg` — the artifact is the
  contract; no grammar code lives here.
- RS2: corpus/suite runner (`Pubid::run_tests`, tests/conformance.rs)
  over the native engine.

Tests need the sibling pubid-grammar checkout (or `PG_ARTIFACT_DIR`).
