//! Embedded artifacts: the byte-copied PARG envelopes this crate ships.
//!
//! Each artifact is vendored from a pinned pubid-grammar ref (see
//! `/pins.yaml` and the `VENDORED_FROM.md` note beside the file); CI
//! re-fetches the pinned ref and fails on drift. Loading verifies the
//! envelope's canonical sha256 checksum through parsanol — a corrupt or
//! tampered artifact never loads.

use std::path::Path;
use std::sync::OnceLock;

use parsanol::PargArtifact;

use crate::error::{Error, Result};

/// The vendored doi artifact (pubid-grammar `artifacts/doi.json`).
pub const DOI_ARTIFACT: &str = include_str!("../artifacts/doi.json");

/// Load and checksum-verify an artifact from a path.
pub fn load_artifact(path: impl AsRef<Path>) -> Result<PargArtifact> {
    PargArtifact::from_path(path).map_err(|err| Error::Artifact(err.to_string()))
}

/// The doi artifact, loaded once and checksum-verified on first use.
pub fn doi_artifact() -> Result<&'static PargArtifact> {
    static ONCE: OnceLock<std::result::Result<PargArtifact, String>> = OnceLock::new();
    ONCE.get_or_init(|| PargArtifact::from_json(DOI_ARTIFACT).map_err(|err| err.to_string()))
        .as_ref()
        .map_err(|detail| Error::Artifact(detail.clone()))
}
