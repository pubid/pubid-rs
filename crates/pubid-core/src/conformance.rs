//! The conformance checker: parse → render → compare over the
//! pubid-testsuite corpus.
//!
//! Mirrors `Pubid::Conformance::Runner` (Ruby) and pubid-ts
//! `src/conformance/`: one corpus case yields a mismatch list (empty
//! means passing — control flow never depends on exception mechanics),
//! review/quarantined rows are counted and skipped, error rows must be
//! rejected, and only a clean flavor's failures gate the verdict.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use serde_json::Value;
use yaml_rust2::{Yaml, YamlLoader};

use crate::amca::AmcaKind;
use crate::api::ApiKind;
use crate::ccsds::CcsdsKind;
use crate::cen_cenelec::CenKind;
use crate::doi::DoiIdentifier;
use crate::ecma::EcmaKind;
use crate::error::{Error, Result};
use crate::etsi::EtsiKind;
use crate::eu::EuKind;
use crate::evs::EvsKind;
use crate::gb::GbKind;
use crate::gost::GostKind;
use crate::jis::JisKind;
use crate::nesc::NescKind;
use crate::plateau::PlateauKind;
pub use crate::wmo::WmoKind;
pub use crate::{
    AmcaIdentifier, AnsiIdentifier, ApiIdentifier, ArxivIdentifier, CcsdsIdentifier, CenIdentifier,
    EcmaIdentifier, EtsiIdentifier, EuIdentifier, EvsIdentifier, GbIdentifier, GostIdentifier,
    IanaIdentifier, IetfIdentifier, IreIdentifier, IsbnIdentifier, JisIdentifier, NescIdentifier,
    OasisIdentifier, OgcIdentifier, PlateauIdentifier, UnIdentifier, W3cIdentifier, WmoIdentifier,
    XsfIdentifier,
};

// ---------------------------------------------------------------------------
// Identifiers and flavors
// ---------------------------------------------------------------------------

/// A parsed identifier of any ported flavor.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
// The cen_cenelec struct is an order of magnitude larger than the
// small render-layer identifiers; it stays inline because the uniform
// flavor parse wraps every variant the same way.
#[allow(clippy::large_enum_variant)]
pub enum Identifier {
    /// A DOI identifier.
    Doi(DoiIdentifier),
    /// An ISBN (book) identifier.
    Isbn(IsbnIdentifier),
    /// An IANA registry identifier.
    Iana(IanaIdentifier),
    /// An OASIS standard identifier.
    Oasis(OasisIdentifier),
    /// An XSF XEP identifier.
    Xsf(XsfIdentifier),
    /// A UN document identifier.
    Un(UnIdentifier),
    /// An OGC document identifier.
    Ogc(OgcIdentifier),
    /// An IETF identifier (RFC / BCP / STD / FYI / Internet-Draft).
    Ietf(IetfIdentifier),
    /// A W3C identifier (one kind per maturity level).
    W3c(W3cIdentifier),
    /// An API identifier (MPMS / typed / typeless / combined).
    Api(ApiIdentifier),
    /// An AMCA identifier (standard / publication / interpretation).
    Amca(AmcaIdentifier),
    /// A NESC identifier (the IEEE C2 family).
    Nesc(NescIdentifier),
    /// An IRE identifier (the pre-IEEE radio-engineers series).
    Ire(IreIdentifier),
    /// An ANSI identifier (standards and copublished editions).
    Ansi(AnsiIdentifier),
    /// An ECMA identifier (standard / technical report / memento).
    Ecma(EcmaIdentifier),
    /// A GB identifier (Chinese national standards, all-parts).
    Gb(GbIdentifier),
    /// A GOST identifier (interstate/national/harmonized/identical).
    Gost(GostIdentifier),
    /// A CEN/CENELEC identifier (documents, supplements, consolidated,
    /// adopted). Kept inline — the uniform flavor parse wraps it.
    #[allow(clippy::large_enum_variant)]
    CenCenelec(CenIdentifier),
    /// An EVS identifier (plain Estonian standards and adoptions).
    Evs(EvsIdentifier),
    /// A CCSDS identifier (base documents and corrigenda).
    Ccsds(CcsdsIdentifier),
    /// A PLATEAU identifier (handbooks and technical reports).
    Plateau(PlateauIdentifier),
    /// A JIS identifier (industrial standards, WES, supplements).
    Jis(JisIdentifier),
    /// An ETSI identifier (standards and the /A //C supplements).
    Etsi(EtsiIdentifier),
    /// An EU identifier (regulation / directive / decision).
    Eu(EuIdentifier),
    /// An arXiv identifier (e-prints, new form and old form).
    Arxiv(ArxivIdentifier),
    /// A WMO identifier (WMO-No. publications, S-100 specs, annexes).
    Wmo(WmoIdentifier),
    /// An IHO identifier (the S/P/M/B/C series, the S-158 checks_for
    /// model). The flavor is ported; its testsuite corpus slice waits
    /// on the S-158 re-key round, so the vendored gate skips it.
    Iho(crate::iho::IhoIdentifier),
}

impl Identifier {
    /// The canonical hash (`_type`-tagged attribute record).
    pub fn to_hash(&self) -> Value {
        match self {
            Identifier::Doi(id) => id.to_hash(),
            Identifier::Isbn(id) => id.to_hash(),
            Identifier::Iana(id) => id.to_hash(),
            Identifier::Oasis(id) => id.to_hash(),
            Identifier::Xsf(id) => id.to_hash(),
            Identifier::Un(id) => id.to_hash(),
            Identifier::Ogc(id) => id.to_hash(),
            Identifier::Ietf(id) => id.to_hash(),
            Identifier::W3c(id) => id.to_hash(),
            Identifier::Api(id) => id.to_hash(),
            Identifier::Amca(id) => id.to_hash(),
            Identifier::Nesc(id) => id.to_hash(),
            Identifier::Ire(id) => id.to_hash(),
            Identifier::Ansi(id) => id.to_hash(),
            Identifier::Ecma(id) => id.to_hash(),
            Identifier::Gb(id) => id.to_hash(),
            Identifier::Gost(id) => id.to_hash(),
            Identifier::CenCenelec(id) => id.to_hash(),
            Identifier::Evs(id) => id.to_hash(),
            Identifier::Ccsds(id) => id.to_hash(),
            Identifier::Plateau(id) => id.to_hash(),
            Identifier::Jis(id) => id.to_hash(),
            Identifier::Etsi(id) => id.to_hash(),
            Identifier::Eu(id) => id.to_hash(),
            Identifier::Arxiv(id) => id.to_hash(),
            Identifier::Wmo(id) => id.to_hash(),
            Identifier::Iho(id) => id.to_hash(),
        }
    }

    /// The canonical human representation.
    pub fn to_human(&self) -> String {
        match self {
            Identifier::Doi(id) => id.to_human(),
            Identifier::Isbn(id) => id.to_human(),
            Identifier::Iana(id) => id.to_human(),
            Identifier::Oasis(id) => id.to_human(),
            Identifier::Xsf(id) => id.to_human(),
            Identifier::Un(id) => id.to_human(),
            Identifier::Ogc(id) => id.to_human(),
            Identifier::Ietf(id) => id.to_human(),
            Identifier::W3c(id) => id.to_human(),
            Identifier::Api(id) => id.to_human(),
            Identifier::Amca(id) => id.to_human(),
            Identifier::Nesc(id) => id.to_human(),
            Identifier::Ire(id) => id.to_human(),
            Identifier::Ansi(id) => id.to_human(),
            Identifier::Ecma(id) => id.to_human(),
            Identifier::Gb(id) => id.to_human(),
            Identifier::Gost(id) => id.to_human(),
            Identifier::CenCenelec(id) => id.to_human(),
            Identifier::Evs(id) => id.to_human(),
            Identifier::Ccsds(id) => id.to_human(),
            Identifier::Plateau(id) => id.to_human(),
            Identifier::Jis(id) => id.to_human(),
            Identifier::Etsi(id) => id.to_human(),
            Identifier::Eu(id) => id.to_human(),
            Identifier::Arxiv(id) => id.to_human(),
            Identifier::Wmo(id) => id.to_human(),
            Identifier::Iho(id) => id.to_human(),
        }
    }

    /// The emitted URN, when the flavor emits one.
    pub fn to_urn(&self) -> Option<String> {
        match self {
            Identifier::Doi(id) => Some(id.to_urn()),
            Identifier::Isbn(id) => Some(id.to_urn()),
            Identifier::Iana(id) => Some(id.to_urn()),
            Identifier::Oasis(id) => Some(id.to_urn()),
            Identifier::Xsf(id) => Some(id.to_urn()),
            Identifier::Un(id) => Some(id.to_urn()),
            Identifier::Ogc(id) => Some(id.to_urn()),
            Identifier::Ietf(id) => Some(id.to_urn()),
            Identifier::W3c(id) => Some(id.to_urn()),
            Identifier::Api(id) => Some(id.to_urn()),
            Identifier::Amca(id) => Some(id.to_urn()),
            Identifier::Nesc(id) => Some(id.to_urn()),
            Identifier::Ire(id) => Some(id.to_urn()),
            Identifier::Ansi(id) => Some(id.to_urn()),
            Identifier::Ecma(id) => Some(id.to_urn()),
            Identifier::Gb(id) => Some(id.to_urn()),
            Identifier::Gost(id) => Some(id.to_urn()),
            Identifier::CenCenelec(id) => Some(id.to_urn()),
            Identifier::Evs(id) => Some(id.to_urn()),
            Identifier::Ccsds(id) => id.to_urn(),
            Identifier::Plateau(id) => Some(id.to_urn()),
            Identifier::Jis(id) => Some(id.to_urn()),
            Identifier::Etsi(id) => Some(id.to_urn()),
            Identifier::Eu(id) => Some(id.to_urn()),
            Identifier::Arxiv(id) => Some(id.to_urn()),
            Identifier::Wmo(id) => Some(id.to_urn()),
            Identifier::Iho(id) => id.to_urn().ok(),
        }
    }

    /// Build from a canonical hash (the polymorphic `_type` dispatch).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::InvalidHash {
                detail: "missing _type".to_string(),
            })?;
        match kind {
            DoiIdentifier::TYPE => Ok(Identifier::Doi(DoiIdentifier::from_hash(hash)?)),
            IsbnIdentifier::TYPE => Ok(Identifier::Isbn(IsbnIdentifier::from_hash(hash)?)),
            IanaIdentifier::TYPE => Ok(Identifier::Iana(IanaIdentifier::from_hash(hash)?)),
            OasisIdentifier::TYPE => Ok(Identifier::Oasis(OasisIdentifier::from_hash(hash)?)),
            XsfIdentifier::TYPE => Ok(Identifier::Xsf(XsfIdentifier::from_hash(hash)?)),
            UnIdentifier::TYPE => Ok(Identifier::Un(UnIdentifier::from_hash(hash)?)),
            OgcIdentifier::TYPE => Ok(Identifier::Ogc(OgcIdentifier::from_hash(hash)?)),
            _ if kind.starts_with("pubid:ietf:") => {
                Ok(Identifier::Ietf(IetfIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with("pubid:w3c:") => {
                Ok(Identifier::W3c(W3cIdentifier::from_hash(hash)?))
            }
            AnsiIdentifier::TYPE => Ok(Identifier::Ansi(AnsiIdentifier::from_hash(hash)?)),
            _ if kind.starts_with(EcmaKind::KIND_PREFIX) => {
                Ok(Identifier::Ecma(EcmaIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(GbKind::KIND_PREFIX) => {
                Ok(Identifier::Gb(GbIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(GostKind::KIND_PREFIX) => {
                Ok(Identifier::Gost(GostIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(CenKind::KIND_PREFIX) => {
                Ok(Identifier::CenCenelec(CenIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(EvsKind::KIND_PREFIX) => {
                Ok(Identifier::Evs(EvsIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(CcsdsKind::KIND_PREFIX) => {
                Ok(Identifier::Ccsds(CcsdsIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(PlateauKind::KIND_PREFIX) => {
                Ok(Identifier::Plateau(PlateauIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(JisKind::KIND_PREFIX) => {
                Ok(Identifier::Jis(JisIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(EtsiKind::KIND_PREFIX) => {
                Ok(Identifier::Etsi(EtsiIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(EuKind::KIND_PREFIX) => {
                Ok(Identifier::Eu(EuIdentifier::from_hash(hash)?))
            }
            ArxivIdentifier::TYPE => Ok(Identifier::Arxiv(ArxivIdentifier::from_hash(hash)?)),
            _ if kind.starts_with(WmoKind::KIND_PREFIX) => {
                Ok(Identifier::Wmo(WmoIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(crate::iho::IhoKind::KIND_PREFIX) => {
                Ok(Identifier::Iho(crate::iho::IhoIdentifier::from_hash(hash)?))
            }
            IreIdentifier::TYPE => Ok(Identifier::Ire(IreIdentifier::from_hash(hash)?)),
            _ if kind.starts_with(ApiKind::KIND_PREFIX) => {
                Ok(Identifier::Api(ApiIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(AmcaKind::KIND_PREFIX) => {
                Ok(Identifier::Amca(AmcaIdentifier::from_hash(hash)?))
            }
            _ if kind.starts_with(NescKind::KIND_PREFIX) => {
                Ok(Identifier::Nesc(NescIdentifier::from_hash(hash)?))
            }
            other => Err(Error::InvalidHash {
                detail: format!("unknown _type {other:?}"),
            }),
        }
    }
}

/// A flavor implementation: parse an input into an [`Identifier`].
pub trait FlavorImpl: Sync {
    /// The flavor name (the testsuite `tests/<name>/` directory).
    fn name(&self) -> &'static str;

    /// Parse an input spelling; `Err` when the input must be rejected.
    fn parse(&self, input: &str) -> Result<Identifier>;
}

/// Macro: one `FlavorImpl` struct per flavor, the implemented-flavor
/// table, and the name -> implementation lookup. Adding a flavor means
/// adding its module and one row here.
macro_rules! flavor_impls {
    ($( $name:literal => $struct_name:ident, $variant:ident, $identifier:ty ),* $(,)?) => {
        $(
            #[doc = concat!("The ", $name, " flavor implementation.")]
            pub struct $struct_name;

            impl FlavorImpl for $struct_name {
                fn name(&self) -> &'static str {
                    $name
                }

                fn parse(&self, input: &str) -> Result<Identifier> {
                    Ok(Identifier::$variant(<$identifier>::parse(input)?))
                }
            }
        )*

        /// The implemented flavors, by testsuite name.
        pub fn implemented_flavors() -> BTreeSet<&'static str> {
            BTreeSet::from([ $( $name, )* ])
        }

        /// The flavor implementation for a testsuite name.
        pub fn flavor_impl(name: &str) -> Option<&'static dyn FlavorImpl> {
            match name {
                $( $name => Some(&$struct_name), )*
                _ => None,
            }
        }
    };
}

flavor_impls!(
    "doi" => DoiImpl, Doi, crate::doi::DoiIdentifier,
    "isbn" => IsbnImpl, Isbn, IsbnIdentifier,
    "iana" => IanaImpl, Iana, IanaIdentifier,
    "oasis" => OasisImpl, Oasis, OasisIdentifier,
    "xsf" => XsfImpl, Xsf, XsfIdentifier,
    "un" => UnImpl, Un, UnIdentifier,
    "ogc" => OgcImpl, Ogc, OgcIdentifier,
    "ietf" => IetfImpl, Ietf, IetfIdentifier,
    "w3c" => W3cImpl, W3c, W3cIdentifier,
    "api" => ApiImpl, Api, ApiIdentifier,
    "amca" => AmcaImpl, Amca, AmcaIdentifier,
    "nesc" => NescImpl, Nesc, NescIdentifier,
    "ire" => IreImpl, Ire, IreIdentifier,
    "ansi" => AnsiImpl, Ansi, AnsiIdentifier,
    "ecma" => EcmaImpl, Ecma, EcmaIdentifier,
    "gb" => GbImpl, Gb, GbIdentifier,
    "gost" => GostImpl, Gost, GostIdentifier,
    "cen_cenelec" => CenCenelecImpl, CenCenelec, CenIdentifier,
    "evs" => EvsImpl, Evs, EvsIdentifier,
    "ccsds" => CcsdsImpl, Ccsds, CcsdsIdentifier,
    "plateau" => PlateauImpl, Plateau, PlateauIdentifier,
    "jis" => JisImpl, Jis, JisIdentifier,
    "etsi" => EtsiImpl, Etsi, EtsiIdentifier,
    "eu" => EuImpl, Eu, EuIdentifier,
    "arxiv" => ArxivImpl, Arxiv, ArxivIdentifier,
    "wmo" => WmoImpl, Wmo, WmoIdentifier,
    "iho" => IhoImpl, Iho, crate::iho::IhoIdentifier,
);

// ---------------------------------------------------------------------------
// Corpus wire contract
// ---------------------------------------------------------------------------

/// A non-normalized alias spelling from a corpus row.
#[derive(Debug, Clone)]
pub struct Alias {
    /// The alias spelling to parse.
    pub spelling: String,
    /// Render/parse rounds needed to reach the canonical human (above 1
    /// only, a documented multi-round normalization chain).
    pub rounds: u32,
}

/// One corpus case, per the testsuite wire contract (schema/
/// test.schema.yaml in pubid-testsuite is the authority).
#[derive(Debug, Clone)]
pub struct CorpusCase {
    /// The row id, e.g. `doi.resource.0001`.
    pub id: String,
    /// The reference implementation's canonical hash, carried as data.
    pub identifier: Option<Value>,
    /// The canonical spelling AND canonical parse input.
    pub human: Option<String>,
    /// The emitted URN, absent when the reference emits none.
    pub urn: Option<String>,
    /// The non-normalized alias spellings.
    pub aliases: Vec<Alias>,
    /// The raw input on debt/negative rows.
    pub input: Option<String>,
    /// The row expects an error (`expect.error`).
    pub expect_error: bool,
    /// Reference-bug debt: nothing to gate on.
    pub quarantined: bool,
    /// The rendered human does not re-parse in the reference itself.
    pub render_unreparseable: bool,
    /// Contract-linked expectation flag; never gated.
    pub review: Option<String>,
}

/// The `_status.yaml` gate.
#[derive(Debug, Clone, Copy)]
pub struct Status {
    /// Whether the flavor's ledger is clean.
    pub clean: bool,
    /// The ledger's known mismatch count.
    pub known_mismatches: u64,
}

/// A flavor's corpus: positive rows, negative rows, and the status gate.
#[derive(Debug, Clone)]
pub struct FlavorCorpus {
    /// The flavor name.
    pub flavor: String,
    /// Rows from the positive corpus files.
    pub positives: Vec<CorpusCase>,
    /// Rows from `_negative.yaml`.
    pub negatives: Vec<CorpusCase>,
    /// The `_status.yaml` gate, when present.
    pub status: Option<Status>,
}

impl FlavorCorpus {
    /// Load a flavor's corpus from a testsuite checkout
    /// (`tests_root` is the checkout's `tests/` directory).
    ///
    /// Underscore-prefixed files other than `_negative.yaml` /
    /// `_status.yaml` are exporter metadata (normalization chains,
    /// unparsed-debt notes), not corpus rows; they are skipped here so
    /// a checkout run reads the same rows the vendored slice carries.
    pub fn from_dir(tests_root: &Path, flavor: &str) -> Result<Self> {
        let dir = tests_root.join(flavor);
        let mut entries: Vec<(String, String)> = Vec::new();
        let list = std::fs::read_dir(&dir).map_err(Error::Io)?;
        for entry in list {
            let entry = entry.map_err(Error::Io)?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".yaml") {
                continue;
            }
            if name.starts_with('_') && name != "_negative.yaml" && name != "_status.yaml" {
                continue;
            }
            let text = std::fs::read_to_string(entry.path()).map_err(Error::Io)?;
            entries.push((name, text));
        }
        entries.sort();
        Self::from_entries(flavor, &entries)
    }

    /// Build a corpus from named YAML documents (embedded fixture use).
    pub fn from_entries(flavor: &str, entries: &[(String, String)]) -> Result<Self> {
        let mut positives = Vec::new();
        let mut negatives = Vec::new();
        let mut status = None;
        for (name, text) in entries {
            let path = format!("{flavor}/{name}");
            if name == "_status.yaml" {
                status = Some(status_from_yaml(text, &path)?);
            } else if name == "_negative.yaml" {
                negatives = cases_from_yaml(text, &path)?;
            } else {
                positives.extend(cases_from_yaml(text, &path)?);
            }
        }
        Ok(Self {
            flavor: flavor.to_string(),
            positives,
            negatives,
            status,
        })
    }
}

/// The vendored testsuite slices (`fixtures/testsuite/<flavor>/`), one
/// embedded corpus loader per flavor: `vendored_corpus(name)` builds
/// the flavor's corpus from `include_str!`ed files, so `cargo test`
/// needs no network.
macro_rules! vendored_corpora {
    ($( $flavor:literal => [ $( ($file:literal, $const_name:ident) ),* $(,)? ] ),* $(,)?) => {
        $(
            $(
                const $const_name: &str = include_str!(concat!(
                    "../fixtures/testsuite/", $flavor, "/", $file
                ));
            )*
        )*

        /// The vendored corpus for a flavor; `None` when the crate
        /// carries no slice for it.
        pub fn vendored_corpus(flavor: &str) -> Option<Result<FlavorCorpus>> {
            let entries: Vec<(String, String)> = match flavor {
                $(
                    $flavor => vec![ $( ($file.to_string(), $const_name.to_string()) ),* ],
                )*
                _ => return None,
            };
            Some(FlavorCorpus::from_entries(flavor, &entries))
        }

        /// The flavor names with a vendored corpus slice.
        pub const VENDORED_CORPORA: &[&str] = &[ $( $flavor, )* ];
    };
}

vendored_corpora!(
    "doi" => [("resource.yaml", DOI_RESOURCE), ("_negative.yaml", DOI_NEGATIVE), ("_status.yaml", DOI_STATUS)],
    "isbn" => [("book.yaml", ISBN_BOOK), ("_negative.yaml", ISBN_NEGATIVE), ("_status.yaml", ISBN_STATUS)],
    "iana" => [("registry.yaml", IANA_REGISTRY), ("_negative.yaml", IANA_NEGATIVE), ("_status.yaml", IANA_STATUS)],
    "oasis" => [("standard.yaml", OASIS_STANDARD), ("_status.yaml", OASIS_STATUS)],
    "xsf" => [("xep.yaml", XSF_XEP), ("_status.yaml", XSF_STATUS)],
    "un" => [("document.yaml", UN_DOCUMENT), ("_negative.yaml", UN_NEGATIVE), ("_status.yaml", UN_STATUS)],
    "ogc" => [("document.yaml", OGC_DOCUMENT), ("_negative.yaml", OGC_NEGATIVE), ("_status.yaml", OGC_STATUS)],
    "ietf" => [
        ("bcp.yaml", IETF_BCP), ("fyi.yaml", IETF_FYI), ("internet_draft.yaml", IETF_INTERNET_DRAFT),
        ("rfc.yaml", IETF_RFC), ("std.yaml", IETF_STD), ("_status.yaml", IETF_STATUS),
    ],
    "w3c" => [
        ("candidate_recommendation.yaml", W3C_CANDIDATE_RECOMMENDATION),
        ("candidate_recommendation_draft.yaml", W3C_CANDIDATE_RECOMMENDATION_DRAFT),
        ("draft_note.yaml", W3C_DRAFT_NOTE),
        ("note.yaml", W3C_NOTE),
        ("obsolete_recommendation.yaml", W3C_OBSOLETE_RECOMMENDATION),
        ("proposed_edited_recommendation.yaml", W3C_PROPOSED_EDITED_RECOMMENDATION),
        ("proposed_recommendation.yaml", W3C_PROPOSED_RECOMMENDATION),
        ("recommendation.yaml", W3C_RECOMMENDATION),
        ("standard.yaml", W3C_STANDARD),
        ("superseded_recommendation.yaml", W3C_SUPERSEDED_RECOMMENDATION),
        ("working_draft.yaml", W3C_WORKING_DRAFT),
        ("_status.yaml", W3C_STATUS),
    ],
    "api" => [
        ("bulletin.yaml", API_BULLETIN),
        ("continuous_operations_standard.yaml", API_CONTINUOUS_OPERATIONS_STANDARD),
        ("mpms.yaml", API_MPMS),
        ("publication.yaml", API_PUBLICATION),
        ("recommended_practice.yaml", API_RECOMMENDED_PRACTICE),
        ("specification.yaml", API_SPECIFICATION),
        ("standard.yaml", API_STANDARD),
        ("technical_report.yaml", API_TECHNICAL_REPORT),
        ("typeless_standard.yaml", API_TYPELESS_STANDARD),
        ("_status.yaml", API_STATUS),
    ],
    "amca" => [
        ("interpretation.yaml", AMCA_INTERPRETATION),
        ("publication.yaml", AMCA_PUBLICATION),
        ("standard.yaml", AMCA_STANDARD),
        ("_negative.yaml", AMCA_NEGATIVE),
        ("_status.yaml", AMCA_STATUS),
    ],
    "nesc" => [
        ("nesc_edition.yaml", NESC_EDITION),
        ("nesc_handbook.yaml", NESC_HANDBOOK),
        ("nesc_preprint.yaml", NESC_PREPRINT),
        ("nesc_redline.yaml", NESC_REDLINE),
        ("nesc_standard.yaml", NESC_STANDARD),
    ],
    "ire" => [("ire.yaml", IRE_IRE)],
    "ansi" => [("standard.yaml", ANSI_STANDARD), ("_status.yaml", ANSI_STATUS)],
    "ecma" => [
        ("memento.yaml", ECMA_MEMENTO),
        ("standard.yaml", ECMA_STANDARD),
        ("technical_report.yaml", ECMA_TECHNICAL_REPORT),
        ("_status.yaml", ECMA_STATUS),
    ],
    "gb" => [
        ("all_parts.yaml", GB_ALL_PARTS),
        ("standard.yaml", GB_STANDARD),
        ("_negative.yaml", GB_NEGATIVE),
        ("_status.yaml", GB_STATUS),
    ],
    "gost" => [
        ("harmonized.yaml", GOST_HARMONIZED),
        ("identical_adoption.yaml", GOST_IDENTICAL_ADOPTION),
        ("interstate_standard.yaml", GOST_INTERSTATE_STANDARD),
        ("national_standard.yaml", GOST_NATIONAL_STANDARD),
        ("_status.yaml", GOST_STATUS),
    ],
    "cen_cenelec" => [
        ("amendment.yaml", CEN_AMENDMENT),
        ("adopted_european_norm.yaml", CEN_ADOPTED_EUROPEAN_NORM),
        ("cen_report.yaml", CEN_CEN_REPORT),
        ("cen_workshop_agreement.yaml", CEN_CEN_WORKSHOP_AGREEMENT),
        ("consolidated_identifier.yaml", CEN_CONSOLIDATED_IDENTIFIER),
        ("corrigendum.yaml", CEN_CORRIGENDUM),
        ("european_norm.yaml", CEN_EUROPEAN_NORM),
        ("european_prestandard.yaml", CEN_EUROPEAN_PRESTANDARD),
        ("european_specification.yaml", CEN_EUROPEAN_SPECIFICATION),
        ("fragment.yaml", CEN_FRAGMENT),
        ("guide.yaml", CEN_GUIDE),
        ("harmonization_document.yaml", CEN_HARMONIZATION_DOCUMENT),
        ("technical_report.yaml", CEN_TECHNICAL_REPORT),
        ("technical_specification.yaml", CEN_TECHNICAL_SPECIFICATION),
        ("_negative.yaml", CEN_NEGATIVE),
        ("_status.yaml", CEN_STATUS),
    ],
    "evs" => [
        ("estonian_standard.yaml", EVS_ESTONIAN_STANDARD),
        ("international_adoption.yaml", EVS_INTERNATIONAL_ADOPTION),
        ("national_adoption.yaml", EVS_NATIONAL_ADOPTION),
        ("_status.yaml", EVS_STATUS),
    ],
    "ccsds" => [
        ("base.yaml", CCSDS_BASE),
        ("corrigendum.yaml", CCSDS_CORRIGENDUM),
        ("_status.yaml", CCSDS_STATUS),
    ],
    "plateau" => [
        ("handbook.yaml", PLATEAU_HANDBOOK),
        ("technical_report.yaml", PLATEAU_TECHNICAL_REPORT),
        ("_status.yaml", PLATEAU_STATUS),
    ],
    "jis" => [
        ("amendment.yaml", JIS_AMENDMENT),
        ("japanese_industrial_standard.yaml", JIS_JAPANESE_INDUSTRIAL_STANDARD),
        ("welding_engineering_standard.yaml", JIS_WELDING_ENGINEERING_STANDARD),
        ("welding_engineering_technical_report.yaml", JIS_WELDING_ENGINEERING_TECHNICAL_REPORT),
        ("welding_engineering_technical_specification.yaml", JIS_WELDING_ENGINEERING_TECHNICAL_SPECIFICATION),
        ("_status.yaml", JIS_STATUS),
    ],
    "etsi" => [
        ("amendment.yaml", ETSI_AMENDMENT),
        ("corrigendum.yaml", ETSI_CORRIGENDUM),
        ("etsi_standard.yaml", ETSI_ETSI_STANDARD),
        ("_status.yaml", ETSI_STATUS),
    ],
    "eu" => [
        ("regulation.yaml", EU_REGULATION),
        ("directive.yaml", EU_DIRECTIVE),
        ("decision.yaml", EU_DECISION),
        ("_negative.yaml", EU_NEGATIVE),
        ("_status.yaml", EU_STATUS),
    ],
    "arxiv" => [
        ("eprint.yaml", ARXIV_EPRINT),
        ("_negative.yaml", ARXIV_NEGATIVE),
        ("_status.yaml", ARXIV_STATUS),
    ],
    "wmo" => [
        ("annex.yaml", WMO_ANNEX),
        ("publication.yaml", WMO_PUBLICATION),
        ("standard.yaml", WMO_STANDARD),
        ("_negative.yaml", WMO_NEGATIVE),
        ("_status.yaml", WMO_STATUS),
    ],
    "iho" => [
        ("bibliographic.yaml", IHO_BIBLIOGRAPHIC),
        ("circular_letter.yaml", IHO_CIRCULAR_LETTER),
        ("miscellaneous.yaml", IHO_MISCELLANEOUS),
        ("publication.yaml", IHO_PUBLICATION),
        ("standard.yaml", IHO_STANDARD),
        ("_negative.yaml", IHO_NEGATIVE),
        ("_status.yaml", IHO_STATUS),
    ],
);

/// The doi vendored slice (the bootstrap flavor's accessor, kept for
/// parity).
pub fn doi_vendored() -> Result<FlavorCorpus> {
    vendored_corpus("doi").expect("doi slice is vendored")
}

/// Read the `_status.yaml` gate.
fn status_from_yaml(text: &str, path: &str) -> Result<Status> {
    let doc = YamlLoader::load_from_str(text)
        .map_err(|err| corpus_error(path, format!("invalid YAML: {err}")))?
        .into_iter()
        .find(|doc| !matches!(doc, Yaml::Null | Yaml::BadValue))
        .ok_or_else(|| corpus_error(path, "no status document"))?;
    let clean = doc["clean"]
        .as_bool()
        .ok_or_else(|| corpus_error(path, "status has no clean flag"))?;
    let known_mismatches = doc["known_mismatches"].as_i64().unwrap_or(0).max(0) as u64;
    Ok(Status {
        clean,
        known_mismatches,
    })
}

// ---------------------------------------------------------------------------
// YAML -> corpus rows
// ---------------------------------------------------------------------------

fn cases_from_yaml(text: &str, path: &str) -> Result<Vec<CorpusCase>> {
    let docs = YamlLoader::load_from_str(text)
        .map_err(|err| corpus_error(path, format!("invalid YAML: {err}")))?;
    let mut cases = Vec::new();
    for doc in &docs {
        let rows = match doc {
            Yaml::Array(rows) => rows.as_slice(),
            Yaml::Null | Yaml::BadValue => continue,
            Yaml::Hash(_) => std::slice::from_ref(doc),
            other => {
                return Err(corpus_error(
                    path,
                    format!("unexpected document shape {other:?}"),
                ))
            }
        };
        for row in rows {
            cases.push(row_to_case(row, path)?);
        }
    }
    Ok(cases)
}

/// Map one wire row to a [`CorpusCase`].
fn row_to_case(row: &Yaml, path: &str) -> Result<CorpusCase> {
    let hash = row
        .as_hash()
        .ok_or_else(|| corpus_error(path, "row is not a mapping"))?;
    let field = |key: &str| hash.get(&Yaml::String(key.to_string()));

    let id = field("id")
        .and_then(Yaml::as_str)
        .ok_or_else(|| corpus_error(path, "row has no string id"))?
        .to_string();

    let identifier = field("identifier").map(yaml_to_json);
    let representations = field("representations");
    let human = representations
        .and_then(|r| r["human"].as_str())
        .map(str::to_string);
    let urn = representations
        .and_then(|r| r["urn"].as_str())
        .map(str::to_string);

    let aliases = match field("non_normalized_aliases") {
        None | Some(Yaml::Null) | Some(Yaml::BadValue) => Vec::new(),
        Some(Yaml::Array(items)) => items
            .iter()
            .map(|alias| {
                let spelling = alias["spelling"]
                    .as_str()
                    .ok_or_else(|| corpus_error(path, format!("{id}: alias has no spelling")))?
                    .to_string();
                let rounds = alias["rounds"].as_i64().unwrap_or(1).max(1) as u32;
                Ok(Alias { spelling, rounds })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(other) => {
            return Err(corpus_error(
                path,
                format!("{id}: non_normalized_aliases is {other:?}"),
            ))
        }
    };

    let expect_error = !matches!(
        field("expect").map(|expect| &expect["error"]),
        None | Some(Yaml::Null) | Some(Yaml::BadValue)
    );

    let quarantined = identifier.is_none() && !expect_error;
    let review = field("review").and_then(Yaml::as_str).map(str::to_string);

    Ok(CorpusCase {
        id,
        identifier,
        human,
        urn,
        aliases,
        input: field("input").and_then(Yaml::as_str).map(str::to_string),
        expect_error,
        quarantined,
        render_unreparseable: matches!(field("render_unreparseable"), Some(Yaml::Boolean(true))),
        review,
    })
}

/// Convert a YAML node to JSON for the hash comparison. YAML reals go
/// through f64; the corpus holds strings and integers in practice.
fn yaml_to_json(value: &Yaml) -> Value {
    match value {
        Yaml::Null | Yaml::BadValue | Yaml::Alias(_) => Value::Null,
        Yaml::Boolean(b) => Value::Bool(*b),
        Yaml::Integer(i) => Value::Number((*i).into()),
        Yaml::Real(text) => text
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Yaml::String(s) => Value::String(s.clone()),
        Yaml::Array(items) => Value::Array(items.iter().map(yaml_to_json).collect()),
        Yaml::Hash(map) => {
            let mut object = serde_json::Map::new();
            for (key, member) in map {
                let key = match key {
                    Yaml::String(s) => s.clone(),
                    Yaml::Integer(i) => i.to_string(),
                    other => format!("{other:?}"),
                };
                object.insert(key, yaml_to_json(member));
            }
            Value::Object(object)
        }
    }
}

fn corpus_error(path: &str, detail: impl Into<String>) -> Error {
    Error::Corpus {
        path: path.to_string(),
        detail: detail.into(),
    }
}

// ---------------------------------------------------------------------------
// The checker
// ---------------------------------------------------------------------------

/// Per-category measurement counters (the honest counts a run reports).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Successful parses of positive inputs and alias spellings.
    pub parsed: u64,
    /// Rejected negative inputs (expected-error rows that failed to
    /// parse, as required).
    pub rejected: u64,
    /// Human renders compared against the corpus.
    pub rendered: u64,
    /// Canonical hashes deep-compared against the corpus.
    pub hash_compared: u64,
    /// URNs compared against the corpus.
    pub urn_compared: u64,
    /// Deserialize round-trips checked (`from_hash(to_hash())`).
    pub deserialize_checked: u64,
}

/// Check one corpus case: parse → render → compare. Returns the
/// mismatch list (empty means passing).
///
/// Mirrors pubid-ts `checkCase` field for field: canonical hash, human,
/// urn, every non-normalized alias (multi-round chains re-walk the
/// render/parse round per `rounds`), then the deserialize round-trip.
fn check_case(case: &CorpusCase, flavor: &dyn FlavorImpl, tally: &mut Tally) -> Vec<String> {
    let mut mismatches = Vec::new();
    let label = &case.id;

    let Some(human) = case.human.as_deref() else {
        mismatches.push(format!("{} has no representations.human", case.id));
        return mismatches;
    };

    let identifier = match flavor.parse(human) {
        Ok(identifier) => {
            tally.parsed += 1;
            identifier
        }
        Err(err) => {
            mismatches.push(format!("{label} raised {err}"));
            return mismatches;
        }
    };

    let hash = identifier.to_hash();
    if let Some(expected) = &case.identifier {
        tally.hash_compared += 1;
        if hash != *expected {
            mismatches.push(format!("{label} canonical hash"));
        }
    }

    tally.rendered += 1;
    if identifier.to_human() != human {
        mismatches.push(format!("{label} human"));
    }

    if let Some(expected_urn) = &case.urn {
        match identifier.to_urn() {
            Some(urn) => {
                tally.urn_compared += 1;
                if &urn != expected_urn {
                    mismatches.push(format!("{label} urn"));
                }
            }
            None => mismatches.push(format!("{label} urn absent")),
        }
    }

    for alias in &case.aliases {
        let chain = (0..alias.rounds).try_fold(alias.spelling.clone(), |input, _| {
            flavor.parse(&input).map(|parsed| {
                tally.parsed += 1;
                parsed.to_human()
            })
        });
        match chain {
            Ok(rendered) => {
                tally.rendered += 1;
                if rendered != human {
                    mismatches.push(format!("{label} alias {}", alias.spelling));
                }
            }
            Err(err) => {
                mismatches.push(format!("{label} alias {} raised {err}", alias.spelling));
            }
        }
    }

    match Identifier::from_hash(&hash) {
        Ok(rebuilt) => {
            tally.deserialize_checked += 1;
            if rebuilt.to_hash() != hash {
                mismatches.push(format!("{label} deserialize"));
            }
        }
        Err(err) => mismatches.push(format!("{label} deserialize raised {err}")),
    }

    mismatches
}

/// The conformance verdict for one flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every gated case passed.
    Pass,
    /// A clean flavor has failures.
    Fail,
    /// The flavor's ledger is dirty; its shape is recorded, not gated.
    Ledger,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Pass => write!(f, "pass"),
            Outcome::Fail => write!(f, "fail"),
            Outcome::Ledger => write!(f, "ledger"),
        }
    }
}

/// The run report for one flavor.
#[derive(Debug, Clone)]
pub struct FlavorReport {
    /// The flavor name.
    pub flavor: String,
    /// The verdict.
    pub outcome: Outcome,
    /// Gated positive cases that were checked.
    pub cases: u64,
    /// Expected-error rows checked.
    pub errors: u64,
    /// Quarantined (reference-debt) rows skipped.
    pub quarantined: u64,
    /// Review-flagged rows skipped (never gated).
    pub review: u64,
    /// Negative rows that carry no error expectation (the exporter's
    /// reclassify alarms; counted, not gated).
    pub reclassified: u64,
    /// Positive rows that must parse but were skipped as unimplemented.
    pub unimplemented: u64,
    /// The measurement counters.
    pub tally: Tally,
    /// The ledger's documented known-mismatch count (0 when clean or
    /// absent) — the ceiling a ledgered flavor's failures must stay
    /// under to be at parity with the reference's documented state.
    pub known_mismatches: u64,
    /// Pending-marked gated cases that still mismatch (recorded debt,
    /// not gated).
    pub pending: u64,
    /// Pending-marked gated cases that now PASS — the cleanup alarm:
    /// the marker must be removed.
    pub pending_satisfied: u64,
    /// The ids behind [`FlavorReport::pending_satisfied`], in row
    /// order, for the alarm lines.
    pub pending_satisfied_ids: Vec<String>,
    /// The mismatch list.
    pub failures: Vec<String>,
}

/// Run one flavor's corpus through its implementation. Pending marks
/// (`pending`) apply where the corpus may still record expectations
/// the implementation has outgrown — the testsuite-checkout runs; a
/// `None` runs every gated case unconditionally.
pub fn run_flavor(
    corpus: &FlavorCorpus,
    flavor: &dyn FlavorImpl,
    pending: Option<&PendingRegistry>,
) -> FlavorReport {
    let mut tally = Tally::default();
    let mut failures = Vec::new();
    let mut cases = 0u64;
    let mut errors = 0u64;
    let mut quarantined = 0u64;
    let mut review = 0u64;
    let mut reclassified = 0u64;
    let mut pending_marked = 0u64;
    let mut pending_satisfied = 0u64;
    let mut pending_satisfied_ids = Vec::new();

    for case in &corpus.positives {
        if case.review.is_some() {
            review += 1;
            continue;
        }
        if case.expect_error {
            errors += 1;
            if let Some(input) = &case.input {
                match flavor.parse(input) {
                    Ok(_) => failures.push(format!("{} unexpectedly parsed", case.id)),
                    Err(_) => tally.rejected += 1,
                }
            }
            continue;
        }
        if case.quarantined || case.render_unreparseable {
            quarantined += 1;
            continue;
        }
        cases += 1;
        let marked = pending
            .and_then(|registry| registry.for_case(&case.id))
            .is_some();
        let before = failures.len();
        failures.extend(check_case(case, flavor, &mut tally));
        if marked {
            if failures.len() > before {
                // Pending debt: the case still mismatches - recorded,
                // dropped from the failures that gate.
                failures.truncate(before);
                pending_marked += 1;
            } else {
                // Pending satisfied: the cleanup alarm names the id -
                // the marker must be removed.
                pending_satisfied += 1;
                pending_satisfied_ids.push(case.id.clone());
            }
        }
    }

    for row in &corpus.negatives {
        if !row.expect_error {
            reclassified += 1;
            continue;
        }
        let Some(input) = &row.input else {
            continue;
        };
        match flavor.parse(input) {
            Ok(_) => failures.push(format!("{} unexpectedly parsed", row.id)),
            Err(_) => tally.rejected += 1,
        }
    }

    let clean = corpus.status.map(|status| status.clean).unwrap_or(true);
    let known_mismatches = corpus
        .status
        .map(|status| status.known_mismatches)
        .unwrap_or(0);
    let outcome = if clean {
        if failures.is_empty() {
            Outcome::Pass
        } else {
            Outcome::Fail
        }
    } else {
        Outcome::Ledger
    };

    FlavorReport {
        flavor: corpus.flavor.clone(),
        outcome,
        cases,
        errors,
        quarantined,
        review,
        reclassified,
        unimplemented: 0,
        tally,
        known_mismatches,
        pending: pending_marked,
        pending_satisfied,
        pending_satisfied_ids,
        failures,
    }
}

/// The report for a flavor with no implementation yet: full corpus
/// footprint, never gates.
pub fn unimplemented_report(corpus: &FlavorCorpus) -> FlavorReport {
    FlavorReport {
        flavor: corpus.flavor.clone(),
        outcome: Outcome::Pass,
        cases: corpus
            .positives
            .iter()
            .filter(|case| case.human.is_some())
            .count() as u64,
        errors: corpus
            .negatives
            .iter()
            .filter(|row| row.expect_error)
            .count() as u64,
        quarantined: 0,
        review: 0,
        reclassified: corpus
            .negatives
            .iter()
            .filter(|row| !row.expect_error)
            .count() as u64,
        unimplemented: 1,
        tally: Tally::default(),
        known_mismatches: 0,
        pending: 0,
        pending_satisfied: 0,
        pending_satisfied_ids: Vec::new(),
        failures: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Implementation-side pending markers
// ---------------------------------------------------------------------------

/// One pending entry: a corpus case this implementation explicitly
/// cannot handle yet. Visible debt — never a silent failure, never a
/// hidden pass.
#[derive(Debug, Clone)]
pub struct PendingEntry {
    /// One line, the root cause.
    pub reason: String,
    /// The contract id / hand-off the unmark waits on.
    pub reference: String,
}

/// The pending registry (`conformance/pending.yaml` at the
/// implementation repo root). Mirrors the gem's
/// `Pubid::Conformance::Pending`: entries need a reason and a ref, a
/// pending case that passes raises the pending-satisfied alarm, and a
/// mismatch inside a pending case is recorded as pending, not as a
/// failure.
#[derive(Debug, Clone, Default)]
pub struct PendingRegistry {
    entries: Vec<(String, PendingEntry)>,
}

impl PendingRegistry {
    /// Parse and validate a pending.yaml document. Entries without a
    /// reason or a ref are rejected at load.
    pub fn from_yaml(text: &str) -> Result<Self> {
        let mut entries = Vec::new();
        for doc in YamlLoader::load_from_str(text)
            .map_err(|err| Error::Corpus {
                path: "pending.yaml".to_string(),
                detail: format!("invalid YAML: {err}"),
            })?
            .into_iter()
        {
            let Yaml::Hash(map) = doc else {
                continue;
            };
            for (pattern, meta) in map {
                let Yaml::String(pattern) = pattern else {
                    return Err(pending_error("entry pattern is not a string"));
                };
                let Yaml::Hash(meta) = meta else {
                    return Err(pending_error(format!("{pattern}: entry is not a mapping")));
                };
                let field = |key: &str| {
                    meta.get(&Yaml::String(key.to_string()))
                        .and_then(Yaml::as_str)
                };
                let Some(reason) = field("reason") else {
                    return Err(pending_error(format!("{pattern}: entry needs a reason")));
                };
                let Some(reference) = field("ref") else {
                    return Err(pending_error(format!("{pattern}: entry needs a ref")));
                };
                entries.push((
                    pattern.to_string(),
                    PendingEntry {
                        reason: reason.to_string(),
                        reference: reference.to_string(),
                    },
                ));
            }
        }
        Ok(Self { entries })
    }

    /// The entry for a case id: the exact match, else the first
    /// pattern that matches (`*` and `?` wildcards, like
    /// `File.fnmatch` for the subset the file uses).
    pub fn for_case(&self, case_id: &str) -> Option<&PendingEntry> {
        self.entries
            .iter()
            .find(|(pattern, _)| pattern == case_id || glob_match(pattern, case_id))
            .map(|(_, entry)| entry)
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn pending_error(detail: impl Into<String>) -> Error {
    Error::Corpus {
        path: "pending.yaml".to_string(),
        detail: detail.into(),
    }
}

/// `File.fnmatch` subset: literal characters plus `*` (any run) and
/// `?` (one character). The registry holds literal ids in practice.
fn glob_match(pattern: &str, text: &str) -> bool {
    fn inner(mut p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') => {
                p = &p[1..];
                for skip in 0..=t.len() {
                    if inner(p, &t[skip..]) {
                        return true;
                    }
                }
                false
            }
            Some(b'?') => !t.is_empty() && inner(&p[1..], &t[1..]),
            Some(&c) => !t.is_empty() && t[0] == c && inner(&p[1..], &t[1..]),
        }
    }
    inner(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod pending_tests {
    use super::PendingRegistry;

    const REGISTRY: &str = "---\n\
        ansi.standard.0001: {reason: \"moved expectation\", since: \"2026-11-20\", ref: \"hand-off x\"}\n\
        \"ansi.*.0042\": {reason: \"glob row\", ref: \"hand-off y\"}\n";

    #[test]
    fn loads_exact_and_wildcard_entries() {
        let registry = PendingRegistry::from_yaml(REGISTRY).expect("loads");
        assert!(!registry.is_empty());
        assert_eq!(
            registry
                .for_case("ansi.standard.0001")
                .expect("exact")
                .reason,
            "moved expectation"
        );
        assert_eq!(
            registry
                .for_case("ansi.standard.0042")
                .expect("glob")
                .reference,
            "hand-off y"
        );
        assert!(registry.for_case("doi.resource.0001").is_none());
    }

    #[test]
    fn rejects_entries_without_reason_or_ref() {
        assert!(PendingRegistry::from_yaml("---\nansi.standard.0001: {ref: x}\n").is_err());
        assert!(PendingRegistry::from_yaml("---\nansi.standard.0001: {reason: y}\n").is_err());
    }

    #[test]
    fn an_empty_document_is_an_empty_registry() {
        let registry = PendingRegistry::from_yaml("---\n").expect("loads");
        assert!(registry.is_empty());
        assert!(registry.for_case("ansi.standard.0001").is_none());
    }
}
