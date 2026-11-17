//! Embedded artifacts: the byte-copied PARG envelopes this crate ships.
//!
//! Each artifact is vendored from a pinned pubid-grammar ref (see
//! `/pins.yaml` and the `VENDORED_FROM.md` note beside the files); CI
//! re-fetches the pinned ref and fails on drift. Loading verifies the
//! envelope's canonical sha256 checksum through parsanol — a corrupt or
//! tampered artifact never loads.

use std::path::Path;
use std::sync::OnceLock;

use parsanol::PargArtifact;

use crate::error::{Error, Result};

/// The vendored artifact texts, their name table, and one checksum-verified
/// loader per flavor (a static `OnceLock` each, so the load-and-verify cost
/// is paid once per flavor for the process lifetime).
macro_rules! vendored_artifacts {
    ($( $flavor:literal => $const_name:ident, $loader:ident ),* $(,)?) => {
        $(
            /// The vendored artifact (pubid-grammar `artifacts/<flavor>.json`).
            pub const $const_name: &str = include_str!(concat!("../artifacts/", $flavor, ".json"));
        )*

        /// The vendored artifact texts, by flavor name.
        pub fn artifact_text(flavor: &str) -> Option<&'static str> {
            match flavor {
                $( $flavor => Some($const_name), )*
                _ => None,
            }
        }

        /// The flavor names with a vendored artifact.
        pub const VENDORED_FLAVORS: &[&str] = &[ $( $flavor, )* ];

        $(
            /// The flavor's artifact, loaded once and checksum-verified
            /// on first use.
            pub fn $loader() -> Result<&'static PargArtifact> {
                static ONCE: OnceLock<std::result::Result<PargArtifact, String>> = OnceLock::new();
                ONCE.get_or_init(|| {
                    PargArtifact::from_json($const_name).map_err(|err| err.to_string())
                })
                .as_ref()
                .map_err(|detail| Error::Artifact(detail.clone()))
            }
        )*
    };
}

vendored_artifacts!(
    "doi" => DOI_ARTIFACT, doi_artifact,
    "isbn" => ISBN_ARTIFACT, isbn_artifact,
    "iana" => IANA_ARTIFACT, iana_artifact,
    "oasis" => OASIS_ARTIFACT, oasis_artifact,
    "xsf" => XSF_ARTIFACT, xsf_artifact,
    "un" => UN_ARTIFACT, un_artifact,
    "ogc" => OGC_ARTIFACT, ogc_artifact,
    "ietf" => IETF_ARTIFACT, ietf_artifact,
    "w3c" => W3C_ARTIFACT, w3c_artifact,
    "api" => API_ARTIFACT, api_artifact,
    "amca" => AMCA_ARTIFACT, amca_artifact,
    "nesc" => NESC_ARTIFACT, nesc_artifact,
    "ire" => IRE_ARTIFACT, ire_artifact,
);

/// A flavor's artifact by name; `None` when the crate carries no
/// artifact for the flavor.
pub fn flavor_artifact(flavor: &str) -> Option<Result<&'static PargArtifact>> {
    match flavor {
        "doi" => Some(doi_artifact()),
        "isbn" => Some(isbn_artifact()),
        "iana" => Some(iana_artifact()),
        "oasis" => Some(oasis_artifact()),
        "xsf" => Some(xsf_artifact()),
        "un" => Some(un_artifact()),
        "ogc" => Some(ogc_artifact()),
        "ietf" => Some(ietf_artifact()),
        "w3c" => Some(w3c_artifact()),
        "api" => Some(api_artifact()),
        "amca" => Some(amca_artifact()),
        "nesc" => Some(nesc_artifact()),
        "ire" => Some(ire_artifact()),
        _ => None,
    }
}

/// Load and checksum-verify an artifact from a path.
pub fn load_artifact(path: impl AsRef<Path>) -> Result<PargArtifact> {
    PargArtifact::from_path(path).map_err(|err| Error::Artifact(err.to_string()))
}
