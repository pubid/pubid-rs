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
    "ansi" => ANSI_ARTIFACT, ansi_artifact,
    "ecma" => ECMA_ARTIFACT, ecma_artifact,
    "gb" => GB_ARTIFACT, gb_artifact,
    "gost" => GOST_ARTIFACT, gost_artifact,
    "adobe" => ADOBE_ARTIFACT, adobe_artifact,
    "plateau" => PLATEAU_ARTIFACT, plateau_artifact,
    "evs" => EVS_ARTIFACT, evs_artifact,
    "ccsds" => CCSDS_ARTIFACT, ccsds_artifact,
    "cen_cenelec" => CEN_CENELEC_ARTIFACT, cen_cenelec_artifact,
    "jis" => JIS_ARTIFACT, jis_artifact,
    "etsi" => ETSI_ARTIFACT, etsi_artifact,
    "doi_pfs" => DOI_PFS_ARTIFACT, doi_pfs_artifact,
    "calconnect_pfs" => CALCONNECT_PFS_ARTIFACT, calconnect_pfs_artifact,
    "cen_pfs" => CEN_PFS_ARTIFACT, cen_pfs_artifact,
    "etsi_pfs" => ETSI_PFS_ARTIFACT, etsi_pfs_artifact,
    "bsi_pfs" => BSI_PFS_ARTIFACT, bsi_pfs_artifact,
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
        "ansi" => Some(ansi_artifact()),
        "ecma" => Some(ecma_artifact()),
        "gb" => Some(gb_artifact()),
        "gost" => Some(gost_artifact()),
        "adobe" => Some(adobe_artifact()),
        "plateau" => Some(plateau_artifact()),
        "evs" => Some(evs_artifact()),
        "ccsds" => Some(ccsds_artifact()),
        "cen_cenelec" => Some(cen_cenelec_artifact()),
        "jis" => Some(jis_artifact()),
        "etsi" => Some(etsi_artifact()),
        "doi_pfs" => Some(doi_pfs_artifact()),
        "calconnect_pfs" => Some(calconnect_pfs_artifact()),
        "cen_pfs" => Some(cen_pfs_artifact()),
        "etsi_pfs" => Some(etsi_pfs_artifact()),
        "bsi_pfs" => Some(bsi_pfs_artifact()),
        _ => None,
    }
}

/// Load and checksum-verify an artifact from a path.
pub fn load_artifact(path: impl AsRef<Path>) -> Result<PargArtifact> {
    PargArtifact::from_path(path).map_err(|err| Error::Artifact(err.to_string()))
}
