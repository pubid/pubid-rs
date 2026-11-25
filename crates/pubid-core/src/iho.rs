//! The iho flavor: the International Hydrographic Organization's
//! letter series (S/P/M/B/C) and their full component tail — edition,
//! supplement, part, annex, appendix (+ subnumber), volume, issue,
//! with-year qualifier, and the S-4 loose-leaf spec citation — ported
//! from the gem's `Pubid::Iho` (the parg artifact drives the parse;
//! this layer casts, validates, renders and emits URNs).
//!
//! ## The S-158 series model (pubid#774/#775/#776 — the gem is the
//! reference, pubid 8133a782)
//!
//! S-158 "Validation Checks — Introduction and Structure" (Ed 1.0.0,
//! clause 2.1) is a FRAMEWORK publication defining a series of separate
//! check publications. Each member is designated `S-158:<NNN>` — the
//! S-100 product number whose validation checks the publication
//! specifies — NOT a part and NOT an opaque number. The typed facet is
//! `product` (`S-158:102` → `number = 158, product = 102`), and
//! only the S-158 family carries it (the colon form binds on every
//! series at the grammar level; the anchor assert rejects `S-44:5` —
//! the member designator is the S-158 series device):
//!
//!   IHO S-158 1.0.0            the framework (no product)
//!   IHO S-158:102 1.0.0        the S-102 check specification (PRIMARY
//!                              designator form)
//!   IHO S-158 0.2.0 Part S-102 the draft-era expanded spelling
//!   IHO S-158 Part S-102 0.2.0 the draft-era citation-order spelling
//!   IHO S-158-S-102 0.2.0      the draft-era compact spelling
//!   IHO S-158 Part 102         the draft-era bare-number spelling
//!
//! The four draft-era spellings parse to the SAME identity as
//! non-normalized aliases (`product = 102`): the draft-era part
//! value — a series reference (`S-102`, any of S/P/M/B/C) or a bare
//! member number (`102`) — rebinds onto `product`, and the edition
//! that follows the part face is the publication's OWN edition (the
//! `parted-` mis-bind of #775 is gone). A lettered or compound part
//! value (`Part A`, `Part 4a`) is a genuine part face and never
//! rebinds; a series-reference part beside a BOUND member also stays a
//! plain part (`S-158:100 Part 2` keeps the part).
//!
//! The spelling face ([`IhoIdentifier::product_face`]) records the
//! draft-era part value AS WRITTEN, so the render keeps the input's
//! spelling — the face always prints in the Part slot, AFTER the
//! edition (the declaration order of #775; the citation order never
//! moves a face). Equality and both URN schemes use the canonical
//! shape; the face is never serialized (the `original_abbr` pattern),
//! and only the parse path restores it — every URN decode and
//! `from_hash` renders the canonical colon designator.
//!
//! ## URN schemes
//!
//! `to_urn` emits the IHO MRN guidance namespace (S-100WG4, "MRN
//! Guidelines for IHO") for the series the guidance gives a
//! publication type — S/P/M:
//!
//!   urn:mrn:iho:pub:<type>:<name>[:<dotted edition>][:<atoms>...]
//!
//! - `<type>`: `spec` (S, Standards and Specifications) / `per` (P,
//!   the periodic publications) / `reg` (M, Miscellaneous Base
//!   Regulatory Publications). The B (Bibliographic) and C (Circular
//!   Letter) series have NO guidance type code — until IHO assigns one
//!   they keep the legacy positional `urn:iho:` emission below (which
//!   stays decodable for every series, never emitted for S/P/M).
//! - `<name>`: `sNNN` — the lowercase series letter + number (the
//!   guidance's own s57/s98 spelling). The S-158 member joins the name
//!   segment hyphenated (`s158-102` — the fleet extension for compound
//!   publication numbers) so the edition keeps its canonical 4th
//!   position.
//! - `<edition>`: the bare dotted x.y.z, NOT ed-prefixed (the S-98
//!   precedent); absent for an edition-non-specific reference.
//! - The trailing atoms are the lowercase marker tokens with
//!   case-preserved values (identity law: `part-A` and `part-a` are
//!   distinct documents): the base with-year qualifier keeps `year-`
//!   right after the edition, then supplement (`sup-`), part
//!   (`part-`), the S-4 citation (`part-B:spec-401:specsub-…:
//!   specvar-…` — the letter rides `part-`, the section is derived,
//!   never emitted), annex (`annex-`), appendix (`appendix-`, the
//!   subnumber hyphen-joining its appendix as one compound segment),
//!   volume (`vol-`) and issue (`issue-`). A component's own edition
//!   rides as `ed-` right after it (the duality law). Segment order is
//!   the canonical face order, never the citation order — the
//!   citation-order instability of #775 is structurally impossible
//!   here: every construction path funnels through one ordered
//!   emitter.
//!
//! The retired positional `urn:iho:` scheme (the B/C emission) and the
//! legacy pubid-URN shapes (`158~3A102`, `part-S-102:parted-0.2.0`,
//! the pre-rename `product-` atom) stay decodable, never emitted.

use serde_json::Value;

use crate::artifact::iho_artifact;
use crate::bound::{hash_str, optional_bound_str, optional_hash_str};
use crate::error::{Error, Result};

/// The concrete IHO series (`Pubid::Iho::Identifiers::*` in the
/// reference): the series letter dispatches the class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IhoKind {
    /// Standards and Specifications (`IHO S-44`).
    #[default]
    Standard,
    /// Publication, the periodical series (`IHO P-1 Vol. 30`).
    Publication,
    /// Miscellaneous Publication (`IHO M-2 Sup 1`).
    Miscellaneous,
    /// Bibliographic Publication (`IHO B-4`).
    Bibliographic,
    /// Circular Letter (`IHO C-13`).
    CircularLetter,
}

impl IhoKind {
    /// The shared `_type` prefix every iho tag carries.
    pub const KIND_PREFIX: &'static str = "pubid:iho:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            IhoKind::Standard => "pubid:iho:standard",
            IhoKind::Publication => "pubid:iho:publication",
            IhoKind::Miscellaneous => "pubid:iho:miscellaneous",
            IhoKind::Bibliographic => "pubid:iho:bibliographic",
            IhoKind::CircularLetter => "pubid:iho:circular-letter",
        }
    }

    /// The series letter the human faces and the grammar share.
    pub fn letter(self) -> char {
        match self {
            IhoKind::Standard => 'S',
            IhoKind::Publication => 'P',
            IhoKind::Miscellaneous => 'M',
            IhoKind::Bibliographic => 'B',
            IhoKind::CircularLetter => 'C',
        }
    }

    /// The pubid-URN type key (`Pubid::Iho::PubidUrnSchema`).
    fn urn_type_key(self) -> &'static str {
        match self {
            IhoKind::Standard => "std",
            IhoKind::Publication => "pub",
            IhoKind::Miscellaneous => "m",
            IhoKind::Bibliographic => "b",
            IhoKind::CircularLetter => "cl",
        }
    }

    /// The MRN guidance publication type per series letter
    /// (`UrnGenerator::MRN_TYPES`): S = Standards and Specifications
    /// → spec (the owner ruling names the guidance code explicitly);
    /// P = the periodic publications → per; M = Miscellaneous (Base
    /// Regulatory Publications) → reg. B and C have no guidance code
    /// (`None` — the legacy emission until IHO assigns one).
    fn mrn_type(self) -> Option<&'static str> {
        match self {
            IhoKind::Standard => Some("spec"),
            IhoKind::Publication => Some("per"),
            IhoKind::Miscellaneous => Some("reg"),
            IhoKind::Bibliographic | IhoKind::CircularLetter => None,
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        match tag {
            "pubid:iho:standard" => Some(IhoKind::Standard),
            "pubid:iho:publication" => Some(IhoKind::Publication),
            "pubid:iho:miscellaneous" => Some(IhoKind::Miscellaneous),
            "pubid:iho:bibliographic" => Some(IhoKind::Bibliographic),
            "pubid:iho:circular-letter" => Some(IhoKind::CircularLetter),
            _ => None,
        }
    }

    fn from_letter(letter: char) -> Option<Self> {
        match letter {
            'S' => Some(IhoKind::Standard),
            'P' => Some(IhoKind::Publication),
            'M' => Some(IhoKind::Miscellaneous),
            'B' => Some(IhoKind::Bibliographic),
            'C' => Some(IhoKind::CircularLetter),
            _ => None,
        }
    }
}

/// The IHO identifier.
#[derive(Debug, Clone, Default)]
pub struct IhoIdentifier {
    /// The concrete series class.
    pub kind: IhoKind,
    /// The document number (`"44"`, `"158"`). The grammar binds the
    /// S-158 colon designator as the typed `product` capture, so a
    /// parsed number never carries a colon run; a decoded legacy shape
    /// (`158~3A102`, a stored pre-migration row) stays verbatim
    /// (untouched legacy shape — the reference's `normalize_checks_alias`
    /// re-keys the draft-era PART model only).
    pub number: String,
    /// The S-100 product number whose validation checks this S-158
    /// publication specifies (`"102"`). S-158 family only (asserted on
    /// the parse and URN decode paths; a stored hash is the
    /// reference's permissive `from_hash`).
    pub product: Option<String>,
    /// The spelling face of the member designator (facets-spelling-
    /// only, equality-ignored): the parse records the draft-era part
    /// value as written ("S-102", "102") so the render keeps the
    /// input's spelling; `None` = the canonical colon designator.
    /// Deserialization and every URN decode restore no face —
    /// equality/URN use the canonical shape.
    pub product_face: Option<String>,
    /// The dotted x.y.z edition (`"1.0.0"`); absent = edition-less.
    pub edition: Option<String>,
    /// The with-year qualifier (`"2024"`, parenthesized beside an
    /// edition — the only base year face).
    pub year: Option<String>,
    pub supplement: Option<String>,
    pub supplement_edition: Option<String>,
    pub supplement_year: Option<String>,
    pub part: Option<String>,
    pub part_edition: Option<String>,
    pub part_year: Option<String>,
    pub annex: Option<String>,
    pub annex_edition: Option<String>,
    pub annex_year: Option<String>,
    pub appendix: Option<String>,
    pub appendix_subnumber: Option<String>,
    pub appendix_edition: Option<String>,
    pub appendix_year: Option<String>,
    /// The serial locator: the yearbook year IS a volume.
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub volume_edition: Option<String>,
    pub volume_year: Option<String>,
    /// The S-4 loose-leaf spec leaf number (`"401"` of B-401); the
    /// citation letter rides [`Self::part`].
    pub spec: Option<String>,
    pub spec_sub: Option<String>,
    pub spec_variant: Option<String>,
}

impl PartialEq for IhoIdentifier {
    /// Identity ignores the spelling face: every S-158 spelling,
    /// citation face order and edition position is one document.
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.number == other.number
            && self.product == other.product
            && self.edition == other.edition
            && self.year == other.year
            && self.supplement == other.supplement
            && self.supplement_edition == other.supplement_edition
            && self.supplement_year == other.supplement_year
            && self.part == other.part
            && self.part_edition == other.part_edition
            && self.part_year == other.part_year
            && self.annex == other.annex
            && self.annex_edition == other.annex_edition
            && self.annex_year == other.annex_year
            && self.appendix == other.appendix
            && self.appendix_subnumber == other.appendix_subnumber
            && self.appendix_edition == other.appendix_edition
            && self.appendix_year == other.appendix_year
            && self.volume == other.volume
            && self.issue == other.issue
            && self.volume_edition == other.volume_edition
            && self.volume_year == other.volume_year
            && self.spec == other.spec
            && self.spec_sub == other.spec_sub
            && self.spec_variant == other.spec_variant
    }
}

impl Eq for IhoIdentifier {}

/// The attribute set a parse or URN decode produces, before the
/// shared validation and the S-158 re-key run.
#[derive(Debug, Default)]
struct Attrs {
    kind: Option<IhoKind>,
    number: Option<String>,
    product: Option<String>,
    edition: Option<String>,
    year: Option<String>,
    supplement: Option<String>,
    supplement_edition: Option<String>,
    supplement_year: Option<String>,
    part: Option<String>,
    part_edition: Option<String>,
    part_year: Option<String>,
    annex: Option<String>,
    annex_edition: Option<String>,
    annex_year: Option<String>,
    appendix: Option<String>,
    appendix_subnumber: Option<String>,
    appendix_edition: Option<String>,
    appendix_year: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    volume_edition: Option<String>,
    volume_year: Option<String>,
    spec: Option<String>,
    spec_sub: Option<String>,
    spec_variant: Option<String>,
}

/// The construction path, deciding the two product-side behaviors the
/// reference splits between its builder and its lutaml deserialization
/// (`Builder#resolve_checks_alias!` vs `Builder.normalize_checks_alias`
/// on `from_hash`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Construction {
    /// The parse path (`Builder.build`): the checks anchor is asserted
    /// and the draft-era face is captured from the part value.
    Parse,
    /// The URN decode paths (the gem's `UrnParser` builds through
    /// `Builder.build`, then drops the face): the checks anchor is
    /// asserted, the canonical colon face restores.
    Urn,
    /// The deserialization path (the gem's `from_hash` +
    /// `normalize_checks_alias`): NO checks anchor assert (a stored
    /// hash carrying `product` on another anchor is the reference's
    /// permissive decode), and a member year yields to a base year.
    FromHash,
}

/// A citation letter is one of S-4's lettered Parts.
const SPEC_LETTERS: [char; 3] = ['A', 'B', 'C'];

/// Components an S-4 spec citation cannot carry: the citation is a
/// leaf of the Part, not a wrappable publication.
const SPEC_FOREIGN: [&str; 6] = [
    "supplement",
    "annex",
    "appendix",
    "appendix_subnumber",
    "volume",
    "issue",
];

impl IhoIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// The S-158 framework number: the only series member that turns
    /// a series reference into `product`.
    const S158: &'static str = "158";

    /// The S-4 loose-leaf anchor every spec citation rides.
    const S4_NUMBER: &'static str = "4";

    /// Parse an input spelling — a human form or one of the three
    /// URN schemes (MRN guidance, the positional `urn:iho:`, pubid
    /// scheme) — into an identifier.
    pub fn parse(input: &str) -> Result<Self> {
        let lowered = input.to_lowercase();
        if lowered.starts_with("urn:mrn:iho:") {
            Self::parse_mrn(input)
        } else if lowered.starts_with("urn:pubid:iho:") {
            Self::parse_pubid_urn(input)
        } else if lowered.starts_with("urn:iho:") {
            Self::parse_legacy_urn(input)
        } else {
            Self::parse_human(input)
        }
    }

    /// Parse a human spelling through the vendored artifact (the
    /// input strips like the reference's parse entry).
    fn parse_human(input: &str) -> Result<Self> {
        let artifact = iho_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input.trim())?;
        let attrs = attrs_from_bound(&bound)?;
        Self::assemble(attrs, Construction::Parse)
    }

    /// The shared construction path: validates the decoded shape and
    /// applies the S-158 unification, then builds the identifier.
    /// Every construction path (human parse, MRN, the two retired URN
    /// schemes, `from_hash`) funnels through here, so one identity
    /// results from every spelling.
    fn assemble(mut attrs: Attrs, construction: Construction) -> Result<Self> {
        // The series letter: an implicit S-4 anchor (a bare citation
        // branch parse carries no series capture — "IHO B-401").
        let kind = attrs.kind.unwrap_or(IhoKind::Standard);
        if attrs.spec.is_some() && attrs.kind.is_none() {
            attrs.number = Some(Self::S4_NUMBER.to_string());
        }
        let number = attrs.number.take().ok_or_else(|| Error::InvalidHash {
            detail: "iho parse has no number".to_string(),
        })?;

        // The citation is a leaf of its Part: the anchor is
        // load-bearing, the letter is one of S-4's lettered Parts, and
        // no component edition or wrappable component rides it.
        if attrs.spec.is_some() {
            if kind != IhoKind::Standard || number != Self::S4_NUMBER {
                return Err(Error::InvalidHash {
                    detail: format!(
                        "an S-4 spec citation requires the S-4 anchor: {}-{} cannot carry \
                         spec {}",
                        kind.letter(),
                        number,
                        attrs.spec.clone().unwrap_or_default(),
                    ),
                });
            }
            if !attrs
                .part
                .as_deref()
                .map(|p| p.len() == 1 && SPEC_LETTERS.contains(&p.chars().next().unwrap()))
                .unwrap_or(false)
            {
                return Err(Error::InvalidHash {
                    detail: format!(
                        "an S-4 spec citation needs a lettered Part (A/B/C), not {:?}",
                        attrs.part
                    ),
                });
            }
            if attrs.part_edition.is_some() || attrs.part_year.is_some() {
                return Err(Error::InvalidHash {
                    detail: "a spec citation rides the S-4 edition, not a part edition".to_string(),
                });
            }
            for foreign in SPEC_FOREIGN {
                if owned_attr(&attrs, foreign).is_some() {
                    return Err(Error::InvalidHash {
                        detail: format!(
                            "an S-4 spec citation is a leaf of its Part and cannot carry {foreign}"
                        ),
                    });
                }
            }
        }

        // The two publication modes are mutually exclusive: a base
        // edition and a component edition cannot ride together.
        if attrs.edition.is_some() {
            for (name, value) in [
                ("part_edition", &attrs.part_edition),
                ("supplement_edition", &attrs.supplement_edition),
                ("annex_edition", &attrs.annex_edition),
                ("appendix_edition", &attrs.appendix_edition),
                ("volume_edition", &attrs.volume_edition),
            ] {
                if value.is_some() {
                    return Err(Error::InvalidHash {
                        detail: format!(
                            "a base edition and a component edition are mutually exclusive: \
                             {name} cannot ride edition {:?}",
                            attrs.edition
                        ),
                    });
                }
            }
        }

        // The S-158 unification (pubid#774/#775/#776): the colon
        // designator is the primary form, the draft-era part spellings
        // (a series reference or the bare member number) re-key onto
        // product. Only the S-158 family moves; a lettered or
        // compound part value is a genuine part face.
        let mut product = attrs.product.take();
        let mut product_face = None;
        if product.is_some() {
            // The member designator is the S-158 series device: the
            // grammar binds the colon shape for every series; the
            // anchor is semantic. Asserted on the builder-driven paths
            // only — the reference's from_hash is the permissive
            // lutaml decode.
            if construction != Construction::FromHash
                && (kind != IhoKind::Standard || number != Self::S158)
            {
                return Err(Error::InvalidHash {
                    detail: format!(
                        "the product member designator is the S-158 series device: \
                         {}-{}:{} cannot carry it",
                        kind.letter(),
                        number,
                        product.clone().unwrap_or_default(),
                    ),
                });
            }
        } else if kind == IhoKind::Standard && number == Self::S158 {
            if let Some(part) = attrs.part.take() {
                if let Some(member) = checks_digits_of(&part) {
                    // The draft-era spelling cites the check
                    // publication's own edition/year after the part
                    // (pubid#775's mis-bind): the slots are the
                    // publication's own. Both-present duplicates
                    // raise; the fill is ||= (the reference keeps the
                    // base on from_hash, raising only the edition).
                    let member_edition = attrs.part_edition.take();
                    let member_year = attrs.part_year.take();
                    if attrs.edition.is_some() && member_edition.is_some() {
                        return Err(Error::InvalidHash {
                            detail: format!(
                                "duplicate edition: the base edition {:?} and the member \
                                 edition {:?} cannot both ride S-158:{member}",
                                attrs.edition, member_edition,
                            ),
                        });
                    }
                    if construction != Construction::FromHash
                        && attrs.year.is_some()
                        && member_year.is_some()
                    {
                        return Err(Error::InvalidHash {
                            detail: format!(
                                "duplicate year: the base year {:?} and the member year \
                                 {:?} cannot both ride S-158:{member}",
                                attrs.year, member_year,
                            ),
                        });
                    }
                    product = Some(member);
                    if construction == Construction::Parse {
                        // The spelling face records the part value as
                        // written so the render keeps the input's
                        // spelling (facets-spelling-only).
                        product_face = Some(part);
                    }
                    if attrs.edition.is_none() {
                        attrs.edition = member_edition;
                    }
                    if attrs.year.is_none() {
                        attrs.year = member_year;
                    }
                } else {
                    // A lettered/numbered part on the framework stays
                    // a part (only the S-158 member faces re-key).
                    attrs.part = Some(part);
                }
            }
        }
        Ok(Self {
            kind,
            number,
            product,
            product_face,
            edition: attrs.edition,
            year: attrs.year,
            supplement: attrs.supplement,
            supplement_edition: attrs.supplement_edition,
            supplement_year: attrs.supplement_year,
            part: attrs.part,
            part_edition: attrs.part_edition,
            part_year: attrs.part_year,
            annex: attrs.annex,
            annex_edition: attrs.annex_edition,
            annex_year: attrs.annex_year,
            appendix: attrs.appendix,
            appendix_subnumber: attrs.appendix_subnumber,
            appendix_edition: attrs.appendix_edition,
            appendix_year: attrs.appendix_year,
            volume: attrs.volume,
            issue: attrs.issue,
            volume_edition: attrs.volume_edition,
            volume_year: attrs.volume_year,
            spec: attrs.spec,
            spec_sub: attrs.spec_sub,
            spec_variant: attrs.spec_variant,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record,
    /// the flavor `key_value` shape; absent fields drop). The
    /// deserialization applies the same S-158 alias unification the
    /// parse path applies (`Builder.normalize_checks_alias` — a stored
    /// row carrying the draft-era part model decodes to the product
    /// identity), restores no spelling face, and runs no checks anchor
    /// assert (the reference's permissive lutaml decode).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(IhoKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown iho _type {:?}", hash.get("_type")),
            })?;
        let attrs = Attrs {
            kind: Some(kind),
            number: Some(hash_str(hash, "number")?.to_string()),
            product: optional_hash_str(hash, "product").map(str::to_string),
            edition: optional_hash_str(hash, "edition").map(str::to_string),
            year: optional_hash_str(hash, "year").map(str::to_string),
            supplement: optional_hash_str(hash, "supplement").map(str::to_string),
            supplement_edition: optional_hash_str(hash, "supplement_edition").map(str::to_string),
            supplement_year: optional_hash_str(hash, "supplement_year").map(str::to_string),
            part: optional_hash_str(hash, "part").map(str::to_string),
            part_edition: optional_hash_str(hash, "part_edition").map(str::to_string),
            part_year: optional_hash_str(hash, "part_year").map(str::to_string),
            annex: optional_hash_str(hash, "annex").map(str::to_string),
            annex_edition: optional_hash_str(hash, "annex_edition").map(str::to_string),
            annex_year: optional_hash_str(hash, "annex_year").map(str::to_string),
            appendix: optional_hash_str(hash, "appendix").map(str::to_string),
            appendix_subnumber: optional_hash_str(hash, "appendix_subnumber").map(str::to_string),
            appendix_edition: optional_hash_str(hash, "appendix_edition").map(str::to_string),
            appendix_year: optional_hash_str(hash, "appendix_year").map(str::to_string),
            volume: optional_hash_str(hash, "volume").map(str::to_string),
            issue: optional_hash_str(hash, "issue").map(str::to_string),
            volume_edition: optional_hash_str(hash, "volume_edition").map(str::to_string),
            volume_year: optional_hash_str(hash, "volume_year").map(str::to_string),
            spec: optional_hash_str(hash, "spec").map(str::to_string),
            spec_sub: optional_hash_str(hash, "spec_sub").map(str::to_string),
            spec_variant: optional_hash_str(hash, "spec_variant").map(str::to_string),
        };
        Self::assemble(attrs, Construction::FromHash)
    }

    /// The canonical hash: the flavor `key_value` shape in declaration
    /// order, absent fields dropped. The publisher ("IHO") and the
    /// spelling face never serialize.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        let fields: [(&str, Option<&String>); 23] = [
            ("product", self.product.as_ref()),
            ("edition", self.edition.as_ref()),
            ("year", self.year.as_ref()),
            ("supplement", self.supplement.as_ref()),
            ("supplement_edition", self.supplement_edition.as_ref()),
            ("supplement_year", self.supplement_year.as_ref()),
            ("part", self.part.as_ref()),
            ("part_edition", self.part_edition.as_ref()),
            ("part_year", self.part_year.as_ref()),
            ("annex", self.annex.as_ref()),
            ("annex_edition", self.annex_edition.as_ref()),
            ("annex_year", self.annex_year.as_ref()),
            ("appendix", self.appendix.as_ref()),
            ("appendix_subnumber", self.appendix_subnumber.as_ref()),
            ("appendix_edition", self.appendix_edition.as_ref()),
            ("appendix_year", self.appendix_year.as_ref()),
            ("volume", self.volume.as_ref()),
            ("volume_edition", self.volume_edition.as_ref()),
            ("volume_year", self.volume_year.as_ref()),
            ("issue", self.issue.as_ref()),
            ("spec", self.spec.as_ref()),
            ("spec_sub", self.spec_sub.as_ref()),
            ("spec_variant", self.spec_variant.as_ref()),
        ];
        for (name, value) in fields {
            if let Some(value) = value {
                object.insert(name.into(), value.clone().into());
            }
        }
        Value::Object(object)
    }

    /// The canonical human (the reference's `Renderer`): the code,
    /// then the faces in print order — edition BEFORE the part; each
    /// independently-publishable component carries its own edition
    /// (+ parenthesized year) right after it; the citation face
    /// replaces the Part face; the appendix joins its subnumber; the
    /// issue rides parenthesized after its volume; the base year
    /// prints only as the parenthesized with-year qualifier.
    ///
    /// The S-158 member designator prints the canonical colon face
    /// (`S-158:102`) when no spelling face is set; the draft-era face
    /// (`product_face`, the part value as written) takes the Part
    /// slot — after the edition, the declaration order of #775 (the
    /// citation order never moves a face).
    pub fn to_human(&self) -> String {
        let mut out = format!("IHO {}-{}", self.kind.letter(), self.number);
        if let Some(checks) = &self.product {
            if self.product_face.is_none() {
                out.push_str(&format!(":{checks}"));
            }
        }
        push_face(&mut out, &self.edition, " {}");
        push_face(&mut out, &self.supplement, " Sup {}");
        push_face(&mut out, &self.supplement_edition, " {}");
        push_face(&mut out, &self.supplement_year, " ({})");
        if let Some(face) = &self.product_face {
            // The draft-era face takes the Part slot (the reference's
            // face_value: the member is not a part, the slot is).
            out.push_str(&format!(" Part {face}"));
        } else if self.spec.is_none() {
            push_face(&mut out, &self.part, " Part {}");
            push_face(&mut out, &self.part_edition, " {}");
            push_face(&mut out, &self.part_year, " ({})");
        }
        // The citation face REPLACES the Part face (the citation
        // letter IS the part).
        if self.spec.is_some() {
            out.push(' ');
            out.push_str(&self.spec_citation());
        }
        push_face(&mut out, &self.annex, " Annex {}");
        push_face(&mut out, &self.annex_edition, " {}");
        push_face(&mut out, &self.annex_year, " ({})");
        push_face(&mut out, &self.appendix, " Appendix {}");
        push_face(&mut out, &self.appendix_subnumber, "-{}");
        push_face(&mut out, &self.appendix_edition, " {}");
        push_face(&mut out, &self.appendix_year, " ({})");
        push_face(&mut out, &self.volume, " Vol. {}");
        push_face(&mut out, &self.issue, " ({})");
        push_face(&mut out, &self.volume_edition, " {}");
        push_face(&mut out, &self.volume_year, " ({})");
        push_face(&mut out, &self.year, " ({})");
        out
    }

    /// The S-4 citation the contents print: `B-401`, `B-122.1`,
    /// `B-521(I)`.
    fn spec_citation(&self) -> String {
        let mut citation = format!(
            "{}-{}",
            self.part.as_deref().unwrap_or_default(),
            self.spec.as_deref().unwrap_or_default()
        );
        if let Some(sub) = &self.spec_sub {
            citation.push_str(&format!(".{sub}"));
        }
        if let Some(variant) = &self.spec_variant {
            citation.push_str(&format!("({variant})"));
        }
        citation
    }
}

/// The member number of a draft-era part face: the digits of a series
/// reference (`S-102`, any of S/P/M/B/C) or of the bare number face
/// (`102`). A lettered or compound part value (`A`, `4a`) is a genuine
/// part face and never rebinds (`Builder.checks_digits_of`).
fn checks_digits_of(part: &str) -> Option<String> {
    if let Some((head, digits)) = part.split_once('-') {
        if head.len() == 1 && "SPMBC".contains(head) && is_digits(digits) {
            return Some(digits.to_string());
        }
        return None;
    }
    if is_digits(part) {
        Some(part.to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Bound-map cast
// ---------------------------------------------------------------------------

/// The attribute set from the artifact's flat bound map (the builder's
/// cast): the typed `product` binding (the colon designator binds
/// for every series; the anchor assert is semantic), spec-citation
/// resolution and the edition shorthand padding.
fn attrs_from_bound(bound: &Value) -> Result<Attrs> {
    let mut attrs = Attrs {
        kind: optional_bound_str(bound, "type")
            .and_then(|t| t.chars().next())
            .and_then(IhoKind::from_letter),
        number: optional_bound_str(bound, "number").map(str::to_string),
        product: optional_bound_str(bound, "product").map(str::to_string),
        year: optional_bound_str(bound, "year").map(str::to_string),
        supplement: optional_bound_str(bound, "supplement").map(str::to_string),
        supplement_edition: optional_bound_str(bound, "supplement_edition").map(str::to_string),
        supplement_year: optional_bound_str(bound, "supplement_year").map(str::to_string),
        part: optional_bound_str(bound, "part").map(str::to_string),
        part_edition: optional_bound_str(bound, "part_edition").map(str::to_string),
        part_year: optional_bound_str(bound, "part_year").map(str::to_string),
        annex: optional_bound_str(bound, "annex").map(str::to_string),
        annex_edition: optional_bound_str(bound, "annex_edition").map(str::to_string),
        annex_year: optional_bound_str(bound, "annex_year").map(str::to_string),
        appendix: optional_bound_str(bound, "appendix").map(str::to_string),
        appendix_subnumber: optional_bound_str(bound, "appendix_subnumber").map(str::to_string),
        appendix_edition: optional_bound_str(bound, "appendix_edition").map(str::to_string),
        appendix_year: optional_bound_str(bound, "appendix_year").map(str::to_string),
        volume: optional_bound_str(bound, "volume").map(str::to_string),
        issue: optional_bound_str(bound, "issue").map(str::to_string),
        volume_edition: optional_bound_str(bound, "volume_edition").map(str::to_string),
        volume_year: optional_bound_str(bound, "volume_year").map(str::to_string),
        spec_sub: optional_bound_str(bound, "spec_sub").map(str::to_string),
        spec_variant: optional_bound_str(bound, "spec_variant").map(str::to_string),
        ..Attrs::default()
    };

    // The official IHO edition is the dotted x.y.z; the `v1`/`v5.1`
    // shorthand is a parse-only alias that fills the same attribute.
    attrs.edition = match optional_bound_str(bound, "edition") {
        Some(edition) => Some(edition.to_string()),
        None => optional_bound_str(bound, "edition_shorthand").map(pad_edition_shorthand),
    };

    // The spec citation: the letter folds into `part` (a Part-rule
    // capture must agree), the spelled section becomes the spec alone
    // and only cross-checks the citation's derived section.
    if let Some(letter) = optional_bound_str(bound, "spec_part") {
        let letter = letter.to_string();
        match &attrs.part {
            Some(existing) if *existing != letter => {
                return Err(Error::InvalidHash {
                    detail: format!(
                        "the citation letter {letter} conflicts with part {existing:?}"
                    ),
                });
            }
            _ => attrs.part = Some(letter),
        }
    }
    if let Some(spec) = optional_bound_str(bound, "spec") {
        attrs.spec = Some(spec.to_string());
    }
    if let Some(spelled) = optional_bound_str(bound, "spec_section") {
        if attrs.spec.is_none() {
            attrs.spec = Some(spelled.to_string());
        } else {
            let spec = attrs.spec.as_deref().unwrap_or_default();
            let derived = spec_section_of(spec);
            if spelled != derived {
                return Err(Error::InvalidHash {
                    detail: format!(
                        "the spelled section {spelled} disagrees with the citation {:?}-{spec}",
                        attrs.part
                    ),
                });
            }
        }
    }
    Ok(attrs)
}

/// The S-4 section a spec belongs to: the hundreds digit, zero-padded
/// (`401` -> `400`); derived, never stored.
fn spec_section_of(spec: &str) -> String {
    format!("{}00", spec.chars().next().unwrap_or('0'))
}

/// Append one human face when its component is present.
fn push_face(out: &mut String, value: &Option<String>, face: &str) {
    if let Some(value) = value {
        out.push_str(&face.replace("{}", value));
    }
}

/// The `v1`/`v5.1` shorthand pads to the official dotted edition
/// (`1` -> `1.0.0`, `5.1` -> `5.1.0`).
fn pad_edition_shorthand(shorthand: &str) -> String {
    let mut groups = shorthand.split('.');
    let major = groups.next().unwrap_or("0");
    let minor = groups.next().unwrap_or("0");
    format!("{major}.{minor}.0")
}

fn owned_attr<'a>(attrs: &'a Attrs, field: &str) -> Option<&'a String> {
    match field {
        "supplement" => attrs.supplement.as_ref(),
        "annex" => attrs.annex.as_ref(),
        "appendix" => attrs.appendix.as_ref(),
        "appendix_subnumber" => attrs.appendix_subnumber.as_ref(),
        "volume" => attrs.volume.as_ref(),
        "issue" => attrs.issue.as_ref(),
        _ => None,
    }
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// The official IHO edition shape: exactly three dotted digit groups
/// (x.y.z).
fn is_dotted_edition(text: &str) -> bool {
    let groups: Vec<&str> = text.split('.').collect();
    groups.len() == 3 && groups.iter().all(|group| is_digits(group))
}

fn is_year(text: &str) -> bool {
    text.len() == 4 && is_digits(text)
}

fn is_upper_letter(value: &str) -> bool {
    value.len() == 1 && value.bytes().all(|b| b.is_ascii_uppercase())
}

/// The part value vocabulary: a letter, digits (with an optional
/// letter tail, `4a`), or a series reference (`S-102`).
fn is_part_value(value: &str) -> bool {
    if is_upper_letter(value) {
        return true;
    }
    if let Some((head, tail)) = value.split_once('-') {
        return head.len() == 1 && "SPMBC".contains(head) && is_digits(tail);
    }
    let split = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    is_digits(&value[..split]) && value[split..].bytes().all(|b| b.is_ascii_alphabetic())
}

// ---------------------------------------------------------------------------
// URN emission: the MRN guidance form (S/P/M) and the legacy positional
// form (B/C) — `UrnGenerator`
// ---------------------------------------------------------------------------

/// The guidance-form trailing atoms in containment order
/// (`UrnGenerator::MRN_SEGMENTS`) — `[component, marker]` pairs, the
/// base with-year qualifier right after the edition, then each
/// component with its own edition/year right after it (`ed-`). The
/// Appendix subnumber hyphen-joins its appendix (one compound
/// segment); the product member rides the name segment; the S-4
/// citation's letter rides `part-`.
const MRN_SEGMENTS: [(&str, &str); 20] = [
    ("year", "year"),
    ("supplement", "sup"),
    ("supplement_edition", "ed"),
    ("supplement_year", "year"),
    ("part", "part"),
    ("part_edition", "ed"),
    ("part_year", "year"),
    ("spec", "spec"),
    ("spec_sub", "specsub"),
    ("spec_variant", "specvar"),
    ("annex", "annex"),
    ("annex_edition", "ed"),
    ("annex_year", "year"),
    ("appendix", "appendix"),
    ("appendix_edition", "ed"),
    ("appendix_year", "year"),
    ("volume", "vol"),
    ("issue", "issue"),
    ("volume_edition", "ed"),
    ("volume_year", "year"),
];

/// `[component, marker]` pairs of the legacy positional scheme
/// (`UrnGenerator::SEGMENTS`), in the containment/canonical-face
/// order. A component edition/year emits right after its component
/// (the `ed-` segment attaches to the node it follows, recording the
/// separately-published mode). `None` markers emit their component as
/// a BARE atom (the appendix subnumber directly after its appendix).
const LEGACY_EMISSION: [(&str, Option<&str>); 22] = [
    ("edition", Some("ed")),
    ("year", Some("year")),
    ("supplement", Some("sup")),
    ("supplement_edition", Some("ed")),
    ("supplement_year", Some("year")),
    ("part", Some("part")),
    ("part_edition", Some("ed")),
    ("part_year", Some("year")),
    ("spec", Some("spec")),
    ("spec_sub", Some("specsub")),
    ("spec_variant", Some("specvar")),
    ("annex", Some("annex")),
    ("annex_edition", Some("ed")),
    ("annex_year", Some("year")),
    ("appendix", Some("appendix")),
    ("appendix_subnumber", None),
    ("appendix_edition", Some("ed")),
    ("appendix_year", Some("year")),
    ("volume", Some("vol")),
    ("issue", Some("issue")),
    ("volume_edition", Some("ed")),
    ("volume_year", Some("year")),
];

impl IhoIdentifier {
    /// The official URN: the adopted IHO MRN guidance form for S/P/M,
    /// the legacy positional form for B/C (no guidance type code).
    pub fn to_urn(&self) -> Result<String> {
        match self.kind.mrn_type() {
            Some(mrn_type) => Ok(self.to_mrn_urn(mrn_type)),
            None => Ok(self.to_legacy_urn()),
        }
    }

    /// The guidance form: type, name (sNNN / s158-NNN), the bare
    /// dotted edition, then the trailing atoms in containment order.
    fn to_mrn_urn(&self, mrn_type: &str) -> String {
        let mut segments = vec![
            "urn:mrn:iho:pub".to_string(),
            mrn_type.to_string(),
            self.mrn_name(),
        ];
        if let Some(edition) = &self.edition {
            segments.push(edition.clone());
        }
        for (component, marker) in MRN_SEGMENTS {
            if let Some(value) = attr_value(self, component) {
                if component == "appendix" {
                    // The Appendix subnumber hyphen-joins its appendix
                    // (one compound segment, RFC 3986-safe).
                    match &self.appendix_subnumber {
                        Some(sub) => segments.push(format!("appendix-{value}-{sub}")),
                        None => segments.push(format!("appendix-{value}")),
                    }
                } else {
                    segments.push(format!("{marker}-{value}"));
                }
            }
        }
        segments.join(":")
    }

    /// The lowercase name form (the guidance's "s57"/"s98" spelling).
    /// The S-158 member joins hyphenated so the edition stays in its
    /// canonical 4th position: s158-102.
    fn mrn_name(&self) -> String {
        let mut name = format!("{}{}", self.kind.letter().to_lowercase(), self.number);
        if let Some(checks) = &self.product {
            name.push_str(&format!("-{checks}"));
        }
        name
    }

    /// The legacy positional form (B/C today; the shape every legacy
    /// `urn:iho:` URN decodes from): the lowercased series letter, the
    /// number verbatim, then the marker segments in containment order.
    fn to_legacy_urn(&self) -> String {
        let mut segments = vec![
            "urn:iho".to_string(),
            self.kind.letter().to_lowercase().to_string(),
            self.number.clone(),
        ];
        for (component, marker) in LEGACY_EMISSION {
            let Some(value) = attr_value(self, component) else {
                continue;
            };
            match marker {
                Some(marker) => segments.push(format!("{marker}-{value}")),
                // The bare atom: the appendix subnumber directly after
                // its appendix.
                None => segments.push(value.clone()),
            }
        }
        segments.join(":")
    }

    /// Parse an MRN URN: the `pub` namespace, the guidance publication
    /// type, the lowercase name (the S-158 member hyphenated), the
    /// bare dotted edition, then the trailing atoms through the shared
    /// marker machinery. The body is NOT case-folded: values are
    /// case-preserved (`appendix-D-2`, `part-B`); only the type token
    /// lowercases. Every value re-reads verbatim and the decoded shape
    /// rebuilds through the shared construction path (the canonical
    /// colon face, the checks anchor asserted).
    fn parse_mrn(urn: &str) -> Result<Self> {
        let body = urn
            .strip_prefix("urn:mrn:iho:pub:")
            .ok_or_else(|| urn_error(format!("invalid IHO MRN: {urn:?}")))?;
        let segments: Vec<&str> = body.split(':').collect();
        let [type_segment, rest @ ..] = segments.as_slice() else {
            return Err(urn_error(format!("invalid IHO MRN: {urn:?}")));
        };
        let letter = match *type_segment {
            "spec" => IhoKind::Standard,
            "per" => IhoKind::Publication,
            "reg" => IhoKind::Miscellaneous,
            other => {
                return Err(urn_error(format!(
                    "unknown IHO publication type {other:?} in {urn:?}"
                )))
            }
        };
        let [name, rest @ ..] = rest else {
            return Err(urn_error(format!("invalid IHO MRN: {urn:?}")));
        };
        let (number, member) = parse_mrn_name(name, urn)?;
        let mut attrs = Attrs {
            kind: Some(letter),
            number: Some(number.to_string()),
            product: member.map(str::to_string),
            ..Attrs::default()
        };
        // The bare dotted edition in the canonical 4th segment.
        let mut rest = rest.to_vec();
        if rest.first().is_some_and(|s| is_dotted_edition(s)) {
            attrs.edition = Some(rest.remove(0).to_string());
        }
        consume_segments(&mut attrs, &rest, urn)?;
        Self::assemble(attrs, Construction::Urn)
    }
}

/// The publication name: `sNNN` (lowercase), or `s158-NNN` with the
/// S-158 member bound as its own component. The member designator
/// rides the S-158 anchor; another series rejects through the anchor
/// assert (`MRN_NAME_RE`).
fn parse_mrn_name<'a>(name: &'a str, urn: &str) -> Result<(&'a str, Option<&'a str>)> {
    let mut chars = name.chars();
    let letter = chars.next().unwrap_or_default();
    if !"spmbc".contains(letter) {
        return Err(urn_error(format!(
            "unknown IHO publication name {name:?} in {urn:?}"
        )));
    }
    let rest = chars.as_str();
    let (number, member) = match rest.split_once('-') {
        Some((number, member)) => (number, Some(member)),
        None => (rest, None),
    };
    if !is_digits(number) || member.is_some_and(|m| !is_digits(m)) {
        return Err(urn_error(format!(
            "unknown IHO publication name {name:?} in {urn:?}"
        )));
    }
    Ok((number, member))
}

// ---------------------------------------------------------------------------
// The shared URN segment machinery (the gem's UrnParser
// consume_segments!/consume_segment!, both emitted schemes)
// ---------------------------------------------------------------------------

/// A marker value's own pattern (the grammar's production rule).
type ValuePattern = fn(&str) -> bool;

/// Marker => attribute stem, with the value pattern each marker's slot
/// accepts (`UrnParser::MARKERS`; the patterns mirror the grammar's
/// own production rules — upper-case letters are case-preserved, the
/// appendix allows the hyphen-joined compound `D-2`, the part allows
/// the digits-then-letters compound `4a` beside a lone letter and the
/// series references).
const MARKERS: [(&str, &str, ValuePattern); 11] = [
    ("ed", "edition", is_dotted_edition),
    ("sup", "supplement", is_digits),
    ("part", "part", is_part_value),
    ("spec", "spec", is_spec_value),
    ("specsub", "spec_sub", is_digits),
    ("specvar", "spec_variant", is_upper_letters),
    ("annex", "annex", is_upper_letter),
    ("appendix", "appendix", is_appendix_value),
    ("vol", "volume", is_digits),
    ("issue", "issue", is_digits),
    ("year", "year", is_year_value),
];

/// The positional markers: their attribute depends on the node they
/// follow (the component-edition duality).
const POSITIONAL_STEMS: [&str; 2] = ["edition", "year"];

/// The S-4 spec node and its children: specsub-/specvar- are only
/// allowed after their parent, and an S-4 spec rides its publication's
/// edition — no ed-/year- inside a spec.
const SPEC_STEMS: [&str; 3] = ["spec", "spec_sub", "spec_variant"];
const SPEC_PARENTS: [&str; 2] = ["spec", "spec_sub"];

/// The positional context: the component the upcoming ed-/year- would
/// attach to, and the owner of the edition just consumed (a year
/// belongs to a component only when it follows that component's own
/// edition).
#[derive(Default)]
struct SegmentCtx {
    component: Option<&'static str>,
    edition_owner: Option<&'static str>,
}

fn consume_segments(attrs: &mut Attrs, segments: &[&str], urn: &str) -> Result<()> {
    let mut ctx = SegmentCtx::default();
    let mut index = 0;
    while index < segments.len() {
        let segment = segments[index];
        index += 1;
        index += consume_segment(attrs, segment, &segments[index..], &mut ctx, urn)?;
    }
    Ok(())
}

/// Consume one segment; returns how many FOLLOWING segments were
/// claimed with it (the appendix bare subnumber).
fn consume_segment(
    attrs: &mut Attrs,
    segment: &str,
    following: &[&str],
    ctx: &mut SegmentCtx,
    urn: &str,
) -> Result<usize> {
    // A marker segment (lowercase marker, case-preserved value).
    if let Some((marker, stem, value)) = split_marker(segment) {
        return consume_marker(attrs, marker, stem, value, following, ctx, urn);
    }
    // Legacy dotted keys (deprecated; decodable, never emitted): part.
    // annex./suppl./ap./appendix./vol./issue. and the single-letter y.
    // year tag — under the serial-locator unification a year IS a
    // volume, so the y. value decodes into the volume. A non-key
    // dotted segment falls through to the bare a.b.c edition below
    // (the gem's LEGACY_RE demands a lowercase key first).
    if let Some((key, value)) = segment.split_once('.') {
        if let Some(stem) = legacy_key_stem(key) {
            // The legacy appendix stored the subnumber inside one
            // string ("appendix.D-2"): two components.
            if stem == "appendix" {
                if let Some((head, sub)) = value.split_once('-') {
                    if head.len() == 1 && is_digits(sub) {
                        store_attr(attrs, "appendix", head, urn)?;
                        store_attr(attrs, "appendix_subnumber", sub, urn)?;
                        return Ok(0);
                    }
                }
            }
            store_attr(attrs, stem, value, urn)?;
            return Ok(0);
        }
    }
    // A bare a.b.c segment is the legacy edition encoding.
    if is_dotted_edition(segment) {
        store_attr(attrs, "edition", segment, urn)?;
        return Ok(0);
    }
    Err(unexpected_segment(segment, urn))
}

/// Consume one marker segment; returns how many FOLLOWING segments
/// were claimed with it (the appendix bare subnumber).
fn consume_marker(
    attrs: &mut Attrs,
    marker: &'static str,
    stem: &'static str,
    value: &str,
    following: &[&str],
    ctx: &mut SegmentCtx,
    urn: &str,
) -> Result<usize> {
    let pattern = MARKERS
        .iter()
        .find(|(name, _, _)| *name == marker)
        .map(|(_, _, valid)| *valid)
        .expect("split_marker only yields table markers");
    if !pattern(value) {
        return Err(urn_error(format!(
            "invalid {marker} value {value:?} in {urn:?}"
        )));
    }
    // specsub-/specvar- are children of the spec atom — allowed only
    // in their parent's slot.
    if (stem == "spec_sub" || stem == "spec_variant")
        && !SPEC_PARENTS.contains(&ctx.component.unwrap_or_default())
    {
        return Err(urn_error(format!(
            "{marker}- is only allowed after spec- in {urn:?}"
        )));
    }
    // The positional markers: the edition attaches to the component it
    // follows (the base when none does), a year to the edition owner.
    // An S-4 spec carries neither — it rides its publication's
    // edition.
    if POSITIONAL_STEMS.contains(&stem) {
        if SPEC_STEMS.contains(&ctx.component.unwrap_or_default()) {
            return Err(urn_error(format!(
                "an S-4 spec rides its publication's edition: {marker}- cannot follow a \
                 spec segment in {urn:?}"
            )));
        }
        if stem == "edition" {
            let owner = ctx.component;
            let key = owner.map_or("edition", component_edition_key);
            store_attr(attrs, key, value, urn)?;
            ctx.edition_owner = owner;
        } else {
            let owner = ctx.edition_owner;
            let key = owner.map_or("year", component_year_key);
            store_attr(attrs, key, value, urn)?;
            ctx.edition_owner = None;
        }
        return Ok(0);
    }
    // The hyphen-joined compound appendix value ("D-2", the MRN
    // emission) splits into the appendix and its subnumber; the legacy
    // emission instead carries the subnumber as the bare atom consumed
    // below.
    if stem == "appendix" {
        if let Some((head, sub)) = value.split_once('-') {
            if is_upper_letter(head) && is_digits(sub) {
                store_attr(attrs, "appendix", head, urn)?;
                store_attr(attrs, "appendix_subnumber", sub, urn)?;
                ctx.component = Some("appendix");
                ctx.edition_owner = None;
                return Ok(0);
            }
        }
    }
    store_attr(attrs, stem, value, urn)?;
    ctx.component = marker_component(stem);
    ctx.edition_owner = None;
    // A bare digit directly after its appendix is the subnumber (the
    // legacy emission's only bare atom after a marker; a year is
    // always marker-tagged).
    if stem == "appendix" {
        if let Some(next) = following.first() {
            if is_digits(next) {
                store_attr(attrs, "appendix_subnumber", next, urn)?;
                return Ok(1);
            }
        }
    }
    Ok(0)
}

/// The legacy dotted-key stems (`UrnParser::LEGACY_KEYS`); `None` for
/// any non-key name (the dotted segment then falls through).
fn legacy_key_stem(key: &str) -> Option<&'static str> {
    match key {
        "part" => Some("part"),
        "annex" => Some("annex"),
        "suppl" => Some("supplement"),
        "ap" | "appendix" => Some("appendix"),
        "vol" => Some("volume"),
        "issue" => Some("issue"),
        "y" => Some("volume"),
        _ => None,
    }
}

fn marker_component(stem: &str) -> Option<&'static str> {
    match stem {
        "supplement" => Some("supplement"),
        "part" => Some("part"),
        "annex" => Some("annex"),
        "appendix" => Some("appendix"),
        "volume" => Some("volume"),
        _ => None,
    }
}

fn component_edition_key(component: &str) -> &'static str {
    match component {
        "supplement" => "supplement_edition",
        "part" => "part_edition",
        "annex" => "annex_edition",
        "appendix" => "appendix_edition",
        "volume" => "volume_edition",
        _ => "edition",
    }
}

fn component_year_key(component: &str) -> &'static str {
    match component {
        "supplement" => "supplement_year",
        "part" => "part_year",
        "annex" => "annex_year",
        "appendix" => "appendix_year",
        "volume" => "volume_year",
        _ => "year",
    }
}

/// `marker-value` → `(marker, stem, value)`: the exact table marker
/// before the first dash, its attribute stem, the case-preserved rest.
fn split_marker(segment: &str) -> Option<(&'static str, &'static str, &str)> {
    let (marker, value) = segment.split_once('-')?;
    MARKERS
        .iter()
        .find(|(name, _, _)| *name == marker)
        .map(|(name, stem, _)| (*name, *stem, value))
}

fn is_spec_value(value: &str) -> bool {
    value.len() == 3 && value.starts_with(['1', '2', '3', '4', '5', '6']) && is_digits(value)
}

fn is_upper_letters(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_uppercase())
}

/// The appendix value vocabulary: an upper letter, digits, or the
/// hyphen-joined compound `D-2` (the MRN emission).
fn is_appendix_value(value: &str) -> bool {
    if is_upper_letter(value) || is_digits(value) {
        return true;
    }
    match value.split_once('-') {
        Some((head, sub)) => is_upper_letter(head) && is_digits(sub),
        None => false,
    }
}

fn is_year_value(value: &str) -> bool {
    (value.starts_with("19") || value.starts_with("20")) && is_year(value)
}

/// Store one decoded attribute; a second value for the same component
/// is a duplicate segment (the gem's `store!` duplicate check).
fn store_attr(attrs: &mut Attrs, key: &str, value: &str, urn: &str) -> Result<()> {
    if attrs_field(attrs, key).is_some() {
        return Err(urn_error(format!(
            "duplicate {key} segment {value:?} in {urn:?}"
        )));
    }
    set_attr(attrs, key, value.to_string());
    Ok(())
}

fn attrs_field<'a>(attrs: &'a Attrs, field: &str) -> Option<&'a String> {
    match field {
        "product" => attrs.product.as_ref(),
        "edition" => attrs.edition.as_ref(),
        "year" => attrs.year.as_ref(),
        "supplement" => attrs.supplement.as_ref(),
        "supplement_edition" => attrs.supplement_edition.as_ref(),
        "supplement_year" => attrs.supplement_year.as_ref(),
        "part" => attrs.part.as_ref(),
        "part_edition" => attrs.part_edition.as_ref(),
        "part_year" => attrs.part_year.as_ref(),
        "spec" => attrs.spec.as_ref(),
        "spec_sub" => attrs.spec_sub.as_ref(),
        "spec_variant" => attrs.spec_variant.as_ref(),
        "annex" => attrs.annex.as_ref(),
        "annex_edition" => attrs.annex_edition.as_ref(),
        "annex_year" => attrs.annex_year.as_ref(),
        "appendix" => attrs.appendix.as_ref(),
        "appendix_subnumber" => attrs.appendix_subnumber.as_ref(),
        "appendix_edition" => attrs.appendix_edition.as_ref(),
        "appendix_year" => attrs.appendix_year.as_ref(),
        "volume" => attrs.volume.as_ref(),
        "issue" => attrs.issue.as_ref(),
        "volume_edition" => attrs.volume_edition.as_ref(),
        "volume_year" => attrs.volume_year.as_ref(),
        _ => None,
    }
}

fn set_attr(attrs: &mut Attrs, field: &str, value: String) {
    match field {
        "product" => attrs.product = Some(value),
        "edition" => attrs.edition = Some(value),
        "year" => attrs.year = Some(value),
        "supplement" => attrs.supplement = Some(value),
        "supplement_edition" => attrs.supplement_edition = Some(value),
        "supplement_year" => attrs.supplement_year = Some(value),
        "part" => attrs.part = Some(value),
        "part_edition" => attrs.part_edition = Some(value),
        "part_year" => attrs.part_year = Some(value),
        "spec" => attrs.spec = Some(value),
        "spec_sub" => attrs.spec_sub = Some(value),
        "spec_variant" => attrs.spec_variant = Some(value),
        "annex" => attrs.annex = Some(value),
        "annex_edition" => attrs.annex_edition = Some(value),
        "annex_year" => attrs.annex_year = Some(value),
        "appendix" => attrs.appendix = Some(value),
        "appendix_subnumber" => attrs.appendix_subnumber = Some(value),
        "appendix_edition" => attrs.appendix_edition = Some(value),
        "appendix_year" => attrs.appendix_year = Some(value),
        "volume" => attrs.volume = Some(value),
        "issue" => attrs.issue = Some(value),
        "volume_edition" => attrs.volume_edition = Some(value),
        "volume_year" => attrs.volume_year = Some(value),
        _ => unreachable!("guarded by attrs_field/store_attr callers"),
    }
}

fn unexpected_segment(segment: &str, urn: &str) -> Error {
    urn_error(format!("unexpected segment {segment:?} in {urn:?}"))
}

fn urn_error(detail: impl Into<String>) -> Error {
    Error::Urn(detail.into())
}

// ---------------------------------------------------------------------------
// Retired positional urn:iho: scheme (the B/C emission; decodable for
// every series, never emitted for S/P/M)
// ---------------------------------------------------------------------------

impl IhoIdentifier {
    /// The positional scheme (`urn:iho:<type>:<number>:...`), kept
    /// decodable for the pre-migration forms — the emitted URN is the
    /// MRN above for S/P/M and this same shape for B/C. The first bare
    /// number segment is the docnumber; the next bare digit segment
    /// binds the S-158 member number (product — the member as its
    /// own segment, never glued into the number); every other
    /// component is a marker-prefixed segment validated against its
    /// own pattern; `ed-`/`year-` are positional (the edition attaches
    /// to the node it follows); unknown, reordered, duplicate or junk
    /// segments reject. The draft-era `part-S-102` segments decode
    /// into `product` through the shared construction path.
    fn parse_legacy_urn(urn: &str) -> Result<Self> {
        let body = urn
            .strip_prefix("urn:iho:")
            .ok_or_else(|| urn_error(format!("invalid IHO URN: {urn:?}")))?;
        let mut segments: Vec<&str> = body.split(':').collect();
        let type_token = segments
            .first()
            .ok_or_else(|| urn_error(format!("invalid IHO URN: {urn:?}")))?
            .to_lowercase();
        let kind = matches!(type_token.as_str(), "s" | "p" | "m" | "b" | "c")
            .then(|| {
                IhoKind::from_letter(
                    type_token
                        .chars()
                        .next()
                        .unwrap_or_default()
                        .to_ascii_uppercase(),
                )
            })
            .flatten()
            .ok_or_else(|| urn_error(format!("unknown IHO series {type_token:?}")))?;
        segments.remove(0);
        let mut attrs = Attrs {
            kind: Some(kind),
            number: Some(consume_legacy_number(&mut segments, urn)?),
            ..Attrs::default()
        };
        // The further bare digit segment binds the S-158 member number
        // (product — the member as its own segment).
        if segments.first().is_some_and(|s| is_digits(s)) {
            attrs.product = Some(segments.remove(0).to_string());
        }
        consume_segments(&mut attrs, &segments, urn)?;
        Self::assemble(attrs, Construction::Urn)
    }
}

/// The first bare number segment is the docnumber (`1/21` — digits
/// with `:` `/` `-` suffix runs); anything else is missing.
fn consume_legacy_number(segments: &mut Vec<&str>, urn: &str) -> Result<String> {
    let is_number = segments
        .first()
        .is_some_and(|segment| is_legacy_number(segment));
    if !is_number {
        return Err(urn_error(format!("missing docnumber in {urn:?}")));
    }
    Ok(segments.remove(0).to_string())
}

/// The docnumber's own charset: digits with `:` `/` `-` suffix runs.
fn is_legacy_number(segment: &str) -> bool {
    let mut groups = segment.split([':', '/', '-']);
    let head = groups.next().unwrap_or_default();
    if !is_digits(head) {
        return false;
    }
    groups.all(is_digits)
}

// ---------------------------------------------------------------------------
// pubid-scheme URN (urn:pubid:iho:...)
// ---------------------------------------------------------------------------

/// The pubid-URN atoms in schema declaration order (`(field, marker)`
/// pairs; the canonical segment order — citation face order never
/// moves a segment). `product` leads: the member atom sits closest
/// to the document number (`Pubid::Iho::PubidUrnSchema`).
const PUBID_ATOMS: [(&str, &str); 23] = [
    ("product", "product"),
    ("edition", "ed"),
    ("year", "year"),
    ("supplement", "sup"),
    ("supplement_edition", "suped"),
    ("supplement_year", "supyear"),
    ("part", "part"),
    ("part_edition", "parted"),
    ("part_year", "party"),
    ("spec", "spec"),
    ("spec_sub", "specsub"),
    ("spec_variant", "specvar"),
    ("annex", "annex"),
    ("annex_edition", "anned"),
    ("annex_year", "annyear"),
    ("appendix", "appendix"),
    ("appendix_subnumber", "appsub"),
    ("appendix_edition", "apped"),
    ("appendix_year", "appyear"),
    ("volume", "vol"),
    ("issue", "issue"),
    ("volume_edition", "voled"),
    ("volume_year", "voly"),
];

/// The legacy pubid-URN atoms: the markers the rename retired, kept
/// decodable and never emitted (the `checks-` marker of the pre-rename
/// schema decodes onto `product` — gem #783). A legacy atom stores
/// through the same field as its replacement, so the duplicate check
/// rejects any mix of the two spellings for one component.
const LEGACY_PUBID_ATOMS: [(&str, &str); 1] = [("product", "checks")];

impl IhoIdentifier {
    /// The pubid-scheme URN (`urn:pubid:iho:<type>:<number>[:atoms]`):
    /// always round-trips, path-safe, one URN per identifier. The
    /// atoms follow the schema declaration order; values escape
    /// non-segment characters with `~XX` (a legacy number's colon run
    /// escapes as `~3A`).
    pub fn to_pubid_urn(&self) -> String {
        let mut segments = vec![
            "urn:pubid:iho".to_string(),
            self.kind.urn_type_key().to_string(),
            pubid_escape(&self.number),
        ];
        for (field, marker) in PUBID_ATOMS {
            let Some(value) = attr_value(self, field) else {
                continue;
            };
            segments.push(format!("{marker}-{}", pubid_escape(value)));
        }
        segments.join(":")
    }

    /// Parse a pubid-scheme URN: the type key, the (escaped) number,
    /// then the atoms in schema order, each validated against its
    /// field's pattern; duplicates and unknown segments reject. The
    /// pre-rename `product-` marker decodes onto `product` (decodable,
    /// never emitted — gem #783). The decoded shape rebuilds through
    /// the shared construction path (the draft-era `part-S-102` shapes
    /// re-key into `product`; the reference decodes this scheme through
    /// the permissive `from_hash` — no checks anchor assert).
    fn parse_pubid_urn(urn: &str) -> Result<Self> {
        let body = urn
            .strip_prefix("urn:pubid:iho:")
            .ok_or_else(|| urn_error(format!("invalid pubid IHO URN: {urn:?}")))?;
        let segments: Vec<&str> = body.split(':').collect();
        let [type_key, number_segment, rest @ ..] = segments.as_slice() else {
            return Err(urn_error(format!("invalid pubid IHO URN: {urn:?}")));
        };
        let kind = match *type_key {
            "std" => IhoKind::Standard,
            "pub" => IhoKind::Publication,
            "m" => IhoKind::Miscellaneous,
            "b" => IhoKind::Bibliographic,
            "cl" => IhoKind::CircularLetter,
            other => return Err(urn_error(format!("unknown pubid IHO type key {other:?}"))),
        };
        let mut attrs = Attrs {
            kind: Some(kind),
            number: Some(pubid_unescape(number_segment).ok_or_else(|| {
                urn_error(format!(
                    "invalid pubid IHO number segment: {number_segment:?}"
                ))
            })?),
            ..Attrs::default()
        };
        for segment in rest {
            let (marker, value) = segment
                .split_once('-')
                .ok_or_else(|| unexpected_segment(segment, urn))?;
            let value = pubid_unescape(value)
                .ok_or_else(|| urn_error(format!("invalid pubid escape in {segment:?}")))?;
            let field = PUBID_ATOMS
                .iter()
                .find(|(_, atom)| *atom == marker)
                .or_else(|| LEGACY_PUBID_ATOMS.iter().find(|(_, atom)| *atom == marker))
                .map(|(field, _)| *field)
                .ok_or_else(|| unexpected_segment(segment, urn))?;
            if attrs_field(&attrs, field).is_some() {
                return Err(urn_error(format!("duplicate {marker} segment: {value:?}")));
            }
            if !pubid_value_valid(field, &value) {
                return Err(urn_error(format!(
                    "invalid pubid value {value:?} for {field}"
                )));
            }
            set_attr(&mut attrs, field, value);
        }
        Self::assemble(attrs, Construction::FromHash)
    }
}

/// The same field read off a built identifier (the URN emitters).
fn attr_value<'a>(id: &'a IhoIdentifier, field: &str) -> Option<&'a String> {
    match field {
        "product" => id.product.as_ref(),
        "edition" => id.edition.as_ref(),
        "year" => id.year.as_ref(),
        "supplement" => id.supplement.as_ref(),
        "supplement_edition" => id.supplement_edition.as_ref(),
        "supplement_year" => id.supplement_year.as_ref(),
        "part" => id.part.as_ref(),
        "part_edition" => id.part_edition.as_ref(),
        "part_year" => id.part_year.as_ref(),
        "spec" => id.spec.as_ref(),
        "spec_sub" => id.spec_sub.as_ref(),
        "spec_variant" => id.spec_variant.as_ref(),
        "annex" => id.annex.as_ref(),
        "annex_edition" => id.annex_edition.as_ref(),
        "annex_year" => id.annex_year.as_ref(),
        "appendix" => id.appendix.as_ref(),
        "appendix_subnumber" => id.appendix_subnumber.as_ref(),
        "appendix_edition" => id.appendix_edition.as_ref(),
        "appendix_year" => id.appendix_year.as_ref(),
        "volume" => id.volume.as_ref(),
        "issue" => id.issue.as_ref(),
        "volume_edition" => id.volume_edition.as_ref(),
        "volume_year" => id.volume_year.as_ref(),
        _ => None,
    }
}

fn pubid_value_valid(field: &str, value: &str) -> bool {
    match field {
        "product" => is_digits(value),
        "edition" | "part_edition" | "supplement_edition" | "annex_edition"
        | "appendix_edition" | "volume_edition" => is_dotted_edition(value),
        "year" | "supplement_year" | "part_year" | "annex_year" | "appendix_year"
        | "volume_year" => is_year_value(value),
        "supplement" => is_digits(value),
        "part" => is_part_value(value),
        "spec" => is_spec_value(value),
        "spec_sub" => is_digits(value),
        "spec_variant" => is_upper_letters(value),
        "annex" => is_upper_letter(value),
        "appendix" => is_upper_letter(value) || is_digits(value),
        "appendix_subnumber" => is_digits(value),
        "volume" | "issue" => is_digits(value),
        _ => false,
    }
}

/// The pubid-URN escape: characters outside the segment-safe set
/// escape as `~XX` (uppercase hex; a legacy number's colon run
/// escapes as `~3A`).
fn pubid_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_') {
            out.push(byte as char);
        } else {
            out.push('~');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

/// The pubid-URN unescape; `None` on a malformed escape.
fn pubid_unescape(segment: &str) -> Option<String> {
    let mut out = String::with_capacity(segment.len());
    let mut chars = segment.chars();
    while let Some(c) = chars.next() {
        if c != '~' {
            out.push(c);
            continue;
        }
        let hex = chars.next()?;
        let low = chars.next()?;
        let byte = u8::from_str_radix(&format!("{hex}{low}"), 16).ok()?;
        out.push(byte as char);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::IhoIdentifier;
    use crate::iho::IhoKind;
    use serde_json::json;

    fn parse(input: &str) -> IhoIdentifier {
        IhoIdentifier::parse(input).unwrap_or_else(|err| panic!("{input}: {err}"))
    }

    fn urn(input: &str) -> String {
        parse(input)
            .to_urn()
            .unwrap_or_else(|err| panic!("{input}: {err}"))
    }

    fn urn_str(id: &IhoIdentifier) -> String {
        id.to_urn().unwrap_or_else(|err| panic!("urn: {err}"))
    }

    // -----------------------------------------------------------------
    // The S-158 series model (pubid#774/#775/#776)
    // -----------------------------------------------------------------

    #[test]
    fn the_colon_form_is_the_primary_designator() {
        for (input, checks) in [
            ("IHO S-158:102 1.0.0", "102"),
            ("S-158:102 1.0.0", "102"),
            ("IHO S-158:98 1.0.0", "98"),
            ("IHO S-158:100 1.0.0", "100"),
            ("IHO S-158:101 1.1.0", "101"),
            ("IHO S-158:130 1.0.0", "130"),
        ] {
            let id = parse(input);
            assert_eq!(id.kind, IhoKind::Standard, "{input}");
            assert_eq!(id.number, "158", "{input}");
            assert_eq!(id.product.as_deref(), Some(checks), "{input}");
            assert_eq!(id.product_face, None, "{input}");
            assert_eq!(id.part, None, "{input}");
        }
        // The colon face is the primary designator: byte-exact.
        let id = parse("IHO S-158:102 1.0.0");
        assert_eq!(id.to_human(), "IHO S-158:102 1.0.0");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:iho:standard",
                "number": "158",
                "product": "102",
                "edition": "1.0.0",
            })
        );
        // The publisher-less spelling parses to the same identity.
        assert_eq!(parse("S-158:102 1.0.0"), id);
    }

    #[test]
    fn the_framework_has_no_product() {
        let id = parse("IHO S-158 1.0.0");
        assert_eq!(id.product, None);
        assert_eq!(id.to_human(), "IHO S-158 1.0.0");
        assert_eq!(urn("IHO S-158 1.0.0"), "urn:mrn:iho:pub:spec:s158:1.0.0");
        assert_eq!(
            id.to_hash(),
            json!({ "_type": "pubid:iho:standard", "number": "158", "edition": "1.0.0" })
        );
    }

    #[test]
    fn the_draft_era_spellings_unify_to_one_identity() {
        // The colon primary and every draft-era spelling are ONE
        // document; the draft-era faces keep the part value as written
        // on render (after the edition — the declaration order), the
        // compact hyphen normalizes to the expanded face, and equality
        // and both URN schemes use the canonical shape.
        let colon = parse("IHO S-158:102 0.2.0");
        let expanded = parse("IHO S-158 0.2.0 Part S-102");
        let reversed = parse("IHO S-158 Part S-102 0.2.0");
        let compact = parse("IHO S-158-S-102 0.2.0");
        let bare = parse("IHO S-158 Part 102 0.2.0");

        for id in [&expanded, &reversed, &compact, &bare] {
            assert_eq!(id, &colon, "{}", id.to_human());
            assert_eq!(id.number, "158");
            assert_eq!(id.product.as_deref(), Some("102"));
            assert_eq!(id.part, None);
            assert_eq!(id.part_edition, None);
            assert_eq!(id.edition.as_deref(), Some("0.2.0"));
        }
        // Render keeps each spelling's own face (byte-exact), the
        // compact hyphen normalizes to the expanded face.
        assert_eq!(colon.to_human(), "IHO S-158:102 0.2.0");
        assert_eq!(expanded.to_human(), "IHO S-158 0.2.0 Part S-102");
        assert_eq!(reversed.to_human(), "IHO S-158 0.2.0 Part S-102");
        assert_eq!(compact.to_human(), "IHO S-158 0.2.0 Part S-102");
        assert_eq!(bare.to_human(), "IHO S-158 0.2.0 Part 102");
        // The canonical shape is order-stable: identical URNs.
        assert_eq!(
            colon.to_hash(),
            json!({
                "_type": "pubid:iho:standard",
                "number": "158",
                "product": "102",
                "edition": "0.2.0",
            })
        );
        for id in [&expanded, &reversed, &compact, &bare] {
            assert_eq!(id.to_hash(), colon.to_hash());
            assert_eq!(urn_str(id), urn_str(&colon));
            assert_eq!(id.to_pubid_urn(), colon.to_pubid_urn());
        }
        // The face values are the part values as written.
        assert_eq!(expanded.product_face.as_deref(), Some("S-102"));
        assert_eq!(bare.product_face.as_deref(), Some("102"));
        // An edition-less draft-era spelling keeps its face with
        // nothing before it.
        assert_eq!(
            parse("IHO S-158 Part S-102").to_human(),
            "IHO S-158 Part S-102"
        );
        assert_eq!(parse("IHO S-158 Part 102").to_human(), "IHO S-158 Part 102");
    }

    #[test]
    fn the_mis_bound_edition_folds_into_the_publication() {
        // pubid#775: `Part S-102 0.2.0` used to bind the edition onto
        // the part (the pubid-URN `parted-` atom). The re-key makes it
        // the publication's own edition, and the emitted pubid-URN
        // carries no parted- atom on this family.
        let id = parse("IHO S-158 Part S-102 0.2.0");
        assert_eq!(id.edition.as_deref(), Some("0.2.0"));
        assert_eq!(id.part_edition, None);
        assert!(!id.to_pubid_urn().contains("parted-"));
        assert_eq!(
            id.to_pubid_urn(),
            "urn:pubid:iho:std:158:product-102:ed-0.2.0"
        );
        // The legacy mis-bound form still decodes into the same
        // identity, never emitted.
        let legacy = parse("urn:pubid:iho:std:158:part-S-102:parted-0.2.0");
        assert_eq!(legacy, id);
        assert_eq!(
            legacy.to_pubid_urn(),
            "urn:pubid:iho:std:158:product-102:ed-0.2.0"
        );
        // The member year folds the same way.
        let dated = parse("IHO S-158 Part S-102 0.2.0 (2023)");
        assert_eq!(dated.edition.as_deref(), Some("0.2.0"));
        assert_eq!(dated.year.as_deref(), Some("2023"));
        assert_eq!(
            urn_str(&dated),
            "urn:mrn:iho:pub:spec:s158-102:0.2.0:year-2023"
        );
    }

    #[test]
    fn the_duplicate_member_edition_or_year_rejects() {
        for input in [
            "IHO S-158 0.2.0 Part S-102 1.0.0",
            "IHO S-158 (2024) Part S-102 (2023)",
        ] {
            assert!(
                IhoIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn the_retired_part_segment_decodes_never_emits() {
        // The `part-S-102` URN segment dies on this family: both
        // retired orders decode into product, and every emit is
        // the MRN member form.
        for legacy in [
            "urn:iho:s:158:ed-0.2.0:part-S-102",
            "urn:iho:s:158:part-S-102:ed-0.2.0",
        ] {
            let id = parse(legacy);
            assert_eq!(id.product.as_deref(), Some("102"), "{legacy}");
            assert_eq!(id.part, None, "{legacy}");
            assert_eq!(id.edition.as_deref(), Some("0.2.0"), "{legacy}");
            assert_eq!(
                urn_str(&id),
                "urn:mrn:iho:pub:spec:s158-102:0.2.0",
                "{legacy}"
            );
        }
        // The escaped colon number of the mis-bound pubid-URN stays
        // the opaque legacy number (the reference re-keys the
        // draft-era PART model only) and round-trips verbatim.
        let escaped = parse("urn:pubid:iho:std:158~3A102:ed-1.0.0");
        assert_eq!(escaped.product, None);
        assert_eq!(escaped.number, "158:102");
        assert_eq!(
            escaped.to_pubid_urn(),
            "urn:pubid:iho:std:158~3A102:ed-1.0.0"
        );
    }

    #[test]
    fn the_member_and_a_genuine_part_coexist() {
        // A series-reference part beside a BOUND member stays a plain
        // part (the rebind runs only while product is unbound), and
        // the numbered part beside the member keeps its own edition
        // (the part's own, separately-published).
        let part_ref = parse("IHO S-158:102 1.0.0 Part S-103");
        assert_eq!(part_ref.product.as_deref(), Some("102"));
        assert_eq!(part_ref.part.as_deref(), Some("S-103"));
        assert_eq!(part_ref.to_human(), "IHO S-158:102 1.0.0 Part S-103");

        let numbered = parse("IHO S-158:100 Part 2 1.0.0");
        assert_eq!(numbered.product.as_deref(), Some("100"));
        assert_eq!(numbered.part.as_deref(), Some("2"));
        assert_eq!(numbered.part_edition.as_deref(), Some("1.0.0"));
        assert_eq!(numbered.edition, None);
        assert_eq!(
            urn_str(&numbered),
            "urn:mrn:iho:pub:spec:s158-100:part-2:ed-1.0.0"
        );
        assert_eq!(
            numbered.to_pubid_urn(),
            "urn:pubid:iho:std:158:product-100:part-2:parted-1.0.0"
        );
    }

    #[test]
    fn the_mrn_target_atoms() {
        assert_eq!(
            urn("IHO S-158:102 1.0.0"),
            "urn:mrn:iho:pub:spec:s158-102:1.0.0"
        );
        assert_eq!(
            urn("IHO S-158:101 1.1.0"),
            "urn:mrn:iho:pub:spec:s158-101:1.1.0"
        );
        assert_eq!(
            urn("IHO S-158:98 1.0.0"),
            "urn:mrn:iho:pub:spec:s158-98:1.0.0"
        );
        assert_eq!(urn("IHO S-158 1.0.0"), "urn:mrn:iho:pub:spec:s158:1.0.0");
        assert_eq!(urn("IHO S-158:100"), "urn:mrn:iho:pub:spec:s158-100");
        // The dotted edition keeps its canonical position after the
        // name segment, member included.
        assert_eq!(
            urn("IHO S-158:102 1.1.0"),
            "urn:mrn:iho:pub:spec:s158-102:1.1.0"
        );
        assert_eq!(
            parse("IHO S-158:102 1.0.0").to_pubid_urn(),
            "urn:pubid:iho:std:158:product-102:ed-1.0.0"
        );
    }

    #[test]
    fn rejects_the_malformed_and_misanchored_shapes() {
        for input in [
            "S-158:",
            "S-158:abc",
            "S-158: 1.0.0",
            "IHO S-158:",
            "IHO S-158 0.2.0 Part S-102 1.0.0", // dual edition modes
            "IHO S-44:5",                       // the member on another anchor
            "IHO S-98:2024",                    // ditto, any other series
        ] {
            assert!(
                IhoIdentifier::parse(input).is_err(),
                "{input} must be rejected"
            );
        }
    }

    #[test]
    fn the_member_designator_and_a_volume_stay_distinct() {
        assert_ne!(urn("IHO S-158:2024"), urn("IHO S-158 Vol. 2024"),);
        assert_eq!(urn("IHO S-158:2024"), "urn:mrn:iho:pub:spec:s158-2024");
        assert_eq!(
            urn("IHO S-158 Vol. 2024"),
            "urn:mrn:iho:pub:spec:s158:vol-2024"
        );
    }

    // -----------------------------------------------------------------
    // MRN round-trip (the guidance emission, whole flavor)
    // -----------------------------------------------------------------

    #[test]
    fn every_mrn_rebuilds_its_identifier() {
        for input in [
            "IHO S-44 5.0.0",
            "IHO S-44",
            "IHO S-158 1.0.0",
            "IHO S-158:102 1.0.0",
            "IHO S-158:101 1.1.0",
            "IHO S-158 Vol. 2024",
            "IHO S-100 5.0.0 Part 2 (2024)",
            "IHO S-5 Part B",
            "IHO S-98 Part A 1.0.0",
            "IHO S-122 1.0.0 Appendix D-2",
            "IHO P-5 Vol. 2024",
            "IHO P-1 Vol. 30 (2)",
            "IHO P-6 Vol. 2 1.0.1",
            "IHO M-2 Sup 1",
            "IHO C-13 1.0.0",
            "IHO B-4 2.19.0",
            "IHO S-4 4.10.0 B-401",
            "IHO S-66 Sup 4",
        ] {
            let id = parse(input);
            let emitted = id.to_urn().unwrap_or_else(|err| panic!("{input}: {err}"));
            let round =
                IhoIdentifier::parse(&emitted).unwrap_or_else(|err| panic!("{emitted}: {err}"));
            assert_eq!(round, id, "{input} -> {emitted}");
            assert_eq!(urn_str(&round), emitted, "{input}");
            assert_eq!(round.to_human(), id.to_human(), "{input} -> {emitted}");
        }
    }

    #[test]
    fn the_mrn_guidance_types_per_series() {
        // S -> spec, P -> per, M -> reg; the B/C series keep the
        // legacy positional emission (no guidance type code).
        assert_eq!(urn("IHO S-98 1.0.0"), "urn:mrn:iho:pub:spec:s98:1.0.0");
        assert_eq!(urn("IHO P-1 1.0.0"), "urn:mrn:iho:pub:per:p1:1.0.0");
        assert_eq!(urn("IHO M-1 2.1.0"), "urn:mrn:iho:pub:reg:m1:2.1.0");
        assert_eq!(urn("IHO B-4 2.19.0"), "urn:iho:b:4:ed-2.19.0");
        assert_eq!(urn("IHO C-13 1.0.0"), "urn:iho:c:13:ed-1.0.0");
    }

    #[test]
    fn mrn_trailing_atoms_are_case_preserved_markers() {
        // The base year qualifier rides right after the edition; the
        // trailing atoms are the lowercase marker tokens with
        // case-preserved values.
        assert_eq!(
            urn("IHO S-100 5.0.0 Part 2 (2024)"),
            "urn:mrn:iho:pub:spec:s100:5.0.0:year-2024:part-2"
        );
        assert_eq!(
            urn("IHO S-100 1.0.0 (2024) Part A"),
            "urn:mrn:iho:pub:spec:s100:1.0.0:year-2024:part-A"
        );
        assert_eq!(
            urn("IHO S-98 1.0.0 Part A"),
            "urn:mrn:iho:pub:spec:s98:1.0.0:part-A"
        );
        assert_eq!(
            urn("IHO S-100 1.0.0 Part 4a"),
            "urn:mrn:iho:pub:spec:s100:1.0.0:part-4a"
        );
        // A component's own edition follows its segment (the
        // separately-published mode).
        assert_eq!(
            urn("IHO S-98 Part A 1.0.0 (2024)"),
            "urn:mrn:iho:pub:spec:s98:part-A:ed-1.0.0:year-2024"
        );
        assert_eq!(
            urn("IHO P-6 Vol. 2 1.0.1"),
            "urn:mrn:iho:pub:per:p6:vol-2:ed-1.0.1"
        );
        assert_eq!(
            urn("IHO P-6 1.0.0 Vol. 2"),
            "urn:mrn:iho:pub:per:p6:1.0.0:vol-2"
        );
        // The Appendix subnumber hyphen-joins its appendix.
        assert_eq!(
            urn("IHO S-122 1.0.0 Appendix D-2"),
            "urn:mrn:iho:pub:spec:s122:1.0.0:appendix-D-2"
        );
        assert_eq!(
            urn("IHO S-52 4.0.0 Appendix 1"),
            "urn:mrn:iho:pub:spec:s52:4.0.0:appendix-1"
        );
        // The serial locators: the yearbook year IS a volume.
        assert_eq!(urn("IHO P-5 Vol. 2024"), "urn:mrn:iho:pub:per:p5:vol-2024");
        assert_eq!(
            urn("IHO P-1 Vol. 30 (2)"),
            "urn:mrn:iho:pub:per:p1:vol-30:issue-2"
        );
    }

    #[test]
    fn mrn_citation_atoms() {
        // The citation letter rides part- (case preserved), the spec
        // number and its sub/variant the atoms after it; the section
        // is derived, never emitted.
        assert_eq!(
            urn("IHO S-4 B-401"),
            "urn:mrn:iho:pub:spec:s4:part-B:spec-401"
        );
        assert_eq!(
            urn("IHO S-4 4.10.0 B-401"),
            "urn:mrn:iho:pub:spec:s4:4.10.0:part-B:spec-401"
        );
        assert_eq!(
            urn("IHO S-4 B-122.1"),
            "urn:mrn:iho:pub:spec:s4:part-B:spec-122:specsub-1"
        );
        assert_eq!(
            urn("IHO S-4 B-521(I)"),
            "urn:mrn:iho:pub:spec:s4:part-B:spec-521:specvar-I"
        );
        assert_eq!(
            urn("IHO S-4 B-351.1(I)"),
            "urn:mrn:iho:pub:spec:s4:part-B:spec-351:specsub-1:specvar-I"
        );
        assert_eq!(
            urn("IHO S-4 B-400"),
            "urn:mrn:iho:pub:spec:s4:part-B:spec-400"
        );
        // The loose forms are the same identity: one URN.
        assert_eq!(urn("B-401"), urn("IHO S-4 B-401"));
        assert_eq!(urn("S-4 Part B, Section 400"), urn("IHO S-4 B-400"));
        assert!(!urn("IHO S-4 B-401").contains("section"));
    }

    #[test]
    fn mrn_segment_order_is_the_canonical_face_order() {
        // Citation face order never moves a segment: both legal
        // spellings of one document emit one URN.
        let first = parse("IHO S-158 0.2.0 Part S-102");
        let second = parse("IHO S-158 Part S-102 0.2.0");
        assert_eq!(urn_str(&first), urn_str(&second));
        assert_eq!(urn_str(&first), "urn:mrn:iho:pub:spec:s158-102:0.2.0");
    }

    #[test]
    fn rejects_the_bad_mrn_urns() {
        for urn in [
            "urn:mrn:iho:pub:spec:s100-2:1.0.0",          // member off s158
            "urn:mrn:iho:pub:spec:s44-5:1.0.0",           // ditto
            "urn:mrn:iho:pub:spec:x158:1.0.0",            // unknown series letter
            "urn:mrn:iho:pub:spec:S158:1.0.0",            // uppercase name
            "urn:mrn:iho:pub:bathy:s44:5.0.0",            // not a guidance type
            "urn:mrn:iho:pub:spec:",                      // no name
            "urn:mrn:iho:pub:spec:s44:junk",              // unknown segment
            "urn:mrn:iho:pub:spec:s44:5.0.0:6.0.0",       // duplicate edition
            "urn:mrn:iho:pub:spec:s44:parta:partb",       // duplicate part
            "urn:mrn:iho:pub:spec:s4:4.10.0:b-401:b-402", // no citation atoms in MRN
            "urn:mrn:iho:pub:spec:s57:3:1",               // the bare correction slot
            "urn:mrn:iho:spec:s44:5.0.0",                 // not the pub namespace
            "urn:mrn:wmo:pub:spec:s44:5.0.0",             // another namespace
            "urn:mrn:iho:pub:spec:s4:4.10.0:b-401:ed-1.0.0", // junk before a marker
            "urn:mrn:iho:pub:spec:s158-abc:1.0.0",        // malformed member
            "urn:mrn:iho:pub:spec:s44:year20",            // not a year
            "urn:mrn:iho:pub:spec:s122:1.0.0:appendix-d-2", // letters stay case-preserved
            "urn:mrn:iho:pub:spec:s122:1.0.0:appendix-D:2:3", // stray bare atom
        ] {
            assert!(IhoIdentifier::parse(urn).is_err(), "{urn} must be rejected");
        }
        // The bare-name partial form decodes (edition-less); the
        // positional ed- marker is a legal trailing atom (the
        // separately-published shapes decode like the legacy scheme).
        let partial = parse("urn:mrn:iho:pub:spec:s44");
        assert_eq!(partial.number, "44");
        assert_eq!(partial.edition, None);
        let separate = parse("urn:mrn:iho:pub:spec:s98:part-A:ed-1.0.0:year-2024");
        assert_eq!(separate.to_human(), "IHO S-98 Part A 1.0.0 (2024)");
    }

    #[test]
    fn decodes_the_guidance_form_rows() {
        // The reference's urn_parser_spec decode table.
        for (mrn, text) in [
            ("urn:mrn:iho:pub:spec:s44:5.0.0", "IHO S-44 5.0.0"),
            ("urn:mrn:iho:pub:spec:s158:1.0.0", "IHO S-158 1.0.0"),
            ("urn:mrn:iho:pub:spec:s158-102:1.0.0", "IHO S-158:102 1.0.0"),
            ("urn:mrn:iho:pub:spec:s158-100", "IHO S-158:100"),
            (
                "urn:mrn:iho:pub:spec:s158-100:part-2:ed-1.0.0",
                "IHO S-158:100 Part 2 1.0.0",
            ),
            ("urn:mrn:iho:pub:reg:m2:sup-1", "IHO M-2 Sup 1"),
            (
                "urn:mrn:iho:pub:per:p1:vol-30:issue-2",
                "IHO P-1 Vol. 30 (2)",
            ),
            (
                "urn:mrn:iho:pub:spec:s100:5.0.0:year-2024:part-2",
                "IHO S-100 5.0.0 Part 2 (2024)",
            ),
            (
                "urn:mrn:iho:pub:spec:s122:1.0.0:appendix-D-2",
                "IHO S-122 1.0.0 Appendix D-2",
            ),
            (
                "urn:mrn:iho:pub:spec:s4:4.10.0:part-B:spec-401",
                "IHO S-4 4.10.0 B-401",
            ),
            (
                "urn:mrn:iho:pub:spec:s98:part-A:ed-1.0.0:year-2024",
                "IHO S-98 Part A 1.0.0 (2024)",
            ),
            ("urn:mrn:iho:pub:spec:s98:1.0.0", "IHO S-98 1.0.0"),
        ] {
            let id = parse(mrn);
            assert_eq!(id.to_human(), text, "{mrn}");
            assert_eq!(urn_str(&id), mrn, "{mrn}");
        }
    }

    // -----------------------------------------------------------------
    // pubid-URN
    // -----------------------------------------------------------------

    #[test]
    fn the_pubid_urn_atoms_follow_the_schema_order() {
        let id = parse("IHO S-100 5.0.0 Part 2 (2024)");
        assert_eq!(
            id.to_pubid_urn(),
            "urn:pubid:iho:std:100:ed-5.0.0:year-2024:part-2"
        );
        let citation = parse("IHO S-4 4.10.0 B-401");
        assert_eq!(
            citation.to_pubid_urn(),
            "urn:pubid:iho:std:4:ed-4.10.0:part-B:spec-401"
        );
        // The reference's pubid_urn_spec table.
        assert_eq!(
            parse("IHO S-98 1.0.0 Part A").to_pubid_urn(),
            "urn:pubid:iho:std:98:ed-1.0.0:part-A"
        );
        assert_eq!(
            parse("IHO S-122 1.0.0 Appendix D-2").to_pubid_urn(),
            "urn:pubid:iho:std:122:ed-1.0.0:appendix-D:appsub-2"
        );
        assert_eq!(
            parse("IHO P-5 Vol. 2024").to_pubid_urn(),
            "urn:pubid:iho:pub:5:vol-2024"
        );
        assert_eq!(
            parse("IHO S-100 Sup 1 2.0.0").to_pubid_urn(),
            "urn:pubid:iho:std:100:sup-1:suped-2.0.0"
        );
        assert_eq!(
            parse("IHO S-52 Appendix 1 4.0.0").to_pubid_urn(),
            "urn:pubid:iho:std:52:appendix-1:apped-4.0.0"
        );
        assert_eq!(
            parse("IHO M-1 2.1.0").to_pubid_urn(),
            "urn:pubid:iho:m:1:ed-2.1.0"
        );
        assert_eq!(
            parse("IHO B-1 1.0.0").to_pubid_urn(),
            "urn:pubid:iho:b:1:ed-1.0.0"
        );
        assert_eq!(
            parse("IHO C-13 1.0.0").to_pubid_urn(),
            "urn:pubid:iho:cl:13:ed-1.0.0"
        );
        assert_eq!(
            parse("IHO S-158:100").to_pubid_urn(),
            "urn:pubid:iho:std:158:product-100"
        );
    }

    #[test]
    fn every_pubid_urn_round_trips() {
        for input in [
            "IHO S-44 5.0.0",
            "IHO S-158 1.0.0",
            "IHO S-158:102 1.0.0",
            "IHO S-158 0.2.0 Part S-102",
            "IHO S-158:100 Part 2 1.0.0",
            "IHO S-100 5.0.0 Part 2 (2024)",
            "IHO S-122 1.0.0 Appendix D-2",
            "IHO P-1 Vol. 30 (2)",
            "IHO M-2 Sup 1",
            "IHO S-4 4.10.0 B-401",
            "IHO S-98 Part A 1.0.0",
            "IHO C-13 1.0.0",
        ] {
            let id = parse(input);
            let pubid_urn = id.to_pubid_urn();
            let round =
                IhoIdentifier::parse(&pubid_urn).unwrap_or_else(|err| panic!("{pubid_urn}: {err}"));
            assert_eq!(round, id, "{input} -> {pubid_urn}");
            assert_eq!(round.to_pubid_urn(), pubid_urn, "{input}");
        }
    }

    #[test]
    fn rejects_the_bad_pubid_urns() {
        for urn in [
            "urn:pubid:iho:std:158:product-102:product-103", // duplicate
            "urn:pubid:iho:std:158:product-102:checks-103",  // legacy duplicate
            "urn:pubid:iho:std:158:product-abc",             // not digits
            "urn:pubid:iho:std:158:checks-abc",              // ditto, legacy atom
            "urn:pubid:iho:std:158:junk-1",                  // unknown atom
            "urn:pubid:iho:zz:158",                          // unknown type key
            "urn:pubid:iho:std:158:ed-1.0",                  // not dotted
            "urn:pubid:iho",                                 // truncated
        ] {
            assert!(IhoIdentifier::parse(urn).is_err(), "{urn} must be rejected");
        }
    }

    #[test]
    fn the_pre_rename_checks_atom_decodes_never_emits() {
        // The `checks-` atom of the pre-rename schema (gem #783) stays
        // decodable onto `product` and never emits: every emit is the
        // `product-` atom.
        let id = parse("urn:pubid:iho:std:158:checks-102:ed-1.0.0");
        assert_eq!(id.product.as_deref(), Some("102"));
        assert_eq!(id.to_human(), "IHO S-158:102 1.0.0");
        assert_eq!(
            id.to_pubid_urn(),
            "urn:pubid:iho:std:158:product-102:ed-1.0.0"
        );
        assert_eq!(id, parse("IHO S-158:102 1.0.0"));
        // The legacy atom stores through the same field: one spelling
        // per component.
        assert!(IhoIdentifier::parse("urn:pubid:iho:std:158:checks-102:checks-103").is_err());
    }

    // -----------------------------------------------------------------
    // The positional urn:iho: scheme (the B/C emission; decodable for
    // every series, never emitted for S/P/M)
    // -----------------------------------------------------------------

    #[test]
    fn the_retired_scheme_still_decodes() {
        for (legacy, expected) in [
            ("urn:iho:s:44:ed-5.0.0", "IHO S-44 5.0.0"),
            ("urn:iho:s:100:ed-5.0.0:part-2", "IHO S-100 5.0.0 Part 2"),
            (
                "urn:iho:s:122:ed-1.0.0:appendix-D:2",
                "IHO S-122 1.0.0 Appendix D-2",
            ),
            ("urn:iho:m:2:sup-1", "IHO M-2 Sup 1"),
            ("urn:iho:p:5:vol-2024", "IHO P-5 Vol. 2024"),
            ("urn:iho:p:1:vol-30:issue-2", "IHO P-1 Vol. 30 (2)"),
            (
                "urn:iho:s:4:ed-4.10.0:part-B:spec-401",
                "IHO S-4 4.10.0 B-401",
            ),
            ("urn:iho:s:158:ed-1.0.0", "IHO S-158 1.0.0"),
            ("urn:iho:s:158:100:ed-1.0.0", "IHO S-158:100 1.0.0"),
            ("urn:iho:s:158:100", "IHO S-158:100"),
            (
                "urn:iho:s:158:100:part-2:ed-1.0.0",
                "IHO S-158:100 Part 2 1.0.0",
            ),
            ("urn:iho:s:158:2024", "IHO S-158:2024"),
            ("urn:iho:s:158:vol-2024", "IHO S-158 Vol. 2024"),
            // The positional ed- attach: a component edition lands on
            // its component, a bare base edition on the base.
            ("urn:iho:p:6:vol-2:ed-1.0.1", "IHO P-6 Vol. 2 1.0.1"),
            ("urn:iho:p:6:ed-1.0.0:vol-2", "IHO P-6 1.0.0 Vol. 2"),
            // The draft-era part-S-102 URN decodes to the product
            // identity; the render keeps the Part face only on the
            // parse path — a URN decode renders the canonical colon.
            ("urn:iho:s:158:part-S-102:ed-0.2.0", "IHO S-158:102 0.2.0"),
            (
                "urn:iho:s:122:appendix-D:2:ed-1.0.0",
                "IHO S-122 Appendix D-2 1.0.0",
            ),
            ("urn:iho:b:4:ed-2.19.0", "IHO B-4 2.19.0"),
        ] {
            let id = parse(legacy);
            assert_eq!(id.to_human(), expected, "{legacy}");
        }
    }

    #[test]
    fn the_legacy_spellings_still_decode() {
        // Legacy 2.0.x forms: decodable, never emitted.
        for (legacy, expected) in [
            ("urn:iho:p:5:y.2024", "IHO P-5 Vol. 2024"),
            ("urn:iho:s:158:y.2024", "IHO S-158 Vol. 2024"),
            ("urn:iho:p:1:vol.30:issue.2", "IHO P-1 Vol. 30 (2)"),
            ("urn:iho:s:66:suppl.1:2.0.0", "IHO S-66 2.0.0 Sup 1"),
            ("urn:iho:s:65:ap.A:1.0.0", "IHO S-65 1.0.0 Appendix A"),
            ("urn:iho:s:100:part.4a:1.0.0", "IHO S-100 1.0.0 Part 4a"),
            ("urn:iho:s:100:annex.A:5.2.0", "IHO S-100 5.2.0 Annex A"),
            ("urn:iho:b:1:1.0.0", "IHO B-1 1.0.0"),
            (
                "urn:iho:s:122:ed-1.0.0:appendix.D-2",
                "IHO S-122 1.0.0 Appendix D-2",
            ),
            // The hyphen-joined compound appendix marker (the MRN
            // emission's shape) decodes through the legacy parser too.
            (
                "urn:iho:s:122:1.0.0:appendix-D-2",
                "IHO S-122 1.0.0 Appendix D-2",
            ),
            ("urn:iho:s:4:part-B:spec-401", "IHO S-4 B-401"),
            ("urn:iho:s:4:part-A:spec-401", "IHO S-4 A-401"),
        ] {
            let id = parse(legacy);
            assert_eq!(id.to_human(), expected, "{legacy}");
        }
    }

    #[test]
    fn rejects_the_bad_retired_urns() {
        for urn in [
            "urn:iho:s:158:junk",
            "urn:iho:s:158:ed-1.0",              // not dotted
            "urn:iho:s:158:ed-5.0.0:ed-6.0.0",   // duplicate edition
            "urn:iho:x:158",                     // unknown series
            "urn:iho:s",                         // no docnumber
            "urn:iho:s:158:part-S-102:spec-401", // the citation needs a letter
            "urn:iho:s:100:part-A1",             // invalid part value
            "urn:iho:s:100:annex-BC",            // invalid annex value
            "urn:iho:s:100:year-19",             // invalid year value
            "urn:iho:s:100:vol-30:vol-31",       // duplicate volume
            "urn:iho:s:122:appendix-D:2:3",      // stray bare atom
            "urn:iho:s:158:100:200",             // a third bare segment
            "urn:iho:s:44:5",                    // the member on another anchor
        ] {
            assert!(IhoIdentifier::parse(urn).is_err(), "{urn} must be rejected");
        }
    }

    // -----------------------------------------------------------------
    // The untouched families
    // -----------------------------------------------------------------

    #[test]
    fn the_other_faces_keep_their_model() {
        let part = parse("IHO S-100 5.0.0 Part 2 (2024)");
        assert_eq!(part.part.as_deref(), Some("2"));
        assert_eq!(part.year.as_deref(), Some("2024"));
        assert_eq!(part.to_hash()["part"], json!("2"));

        let compact = parse("IHO S-5-B");
        assert_eq!(compact.part.as_deref(), Some("B"));
        assert_eq!(compact.to_human(), "IHO S-5 Part B");

        let volume = parse("IHO P-5 Vol. 2024");
        assert_eq!(volume.kind, IhoKind::Publication);
        assert_eq!(volume.volume.as_deref(), Some("2024"));

        let issue = parse("IHO P-1 Vol. 30 (2)");
        assert_eq!(issue.volume.as_deref(), Some("30"));
        assert_eq!(issue.issue.as_deref(), Some("2"));

        let supplement = parse("IHO M-2 Sup 1");
        assert_eq!(supplement.kind, IhoKind::Miscellaneous);
        assert_eq!(supplement.supplement.as_deref(), Some("1"));
        assert_eq!(parse("IHO M-2 Suppl 3").to_human(), "IHO M-2 Sup 3");

        let separately = parse("IHO S-98 Part A 1.0.0");
        // A series-reference model outside S-158: the component
        // edition stays the component's own.
        assert_eq!(separately.part.as_deref(), Some("A"));
        assert_eq!(separately.part_edition.as_deref(), Some("1.0.0"));
        assert_eq!(separately.to_human(), "IHO S-98 Part A 1.0.0");

        let appendix = parse("IHO S-122 1.0.0 Appendix D-2");
        assert_eq!(appendix.appendix.as_deref(), Some("D"));
        assert_eq!(appendix.appendix_subnumber.as_deref(), Some("2"));

        let shorthand = parse("IHO S-100 v5.1");
        assert_eq!(shorthand.edition.as_deref(), Some("5.1.0"));
        assert_eq!(shorthand.to_human(), "IHO S-100 5.1.0");

        let letter = parse("IHO C-13 1.0.0");
        assert_eq!(letter.kind, IhoKind::CircularLetter);

        let citation = parse("IHO S-4 4.10.0 B-401");
        assert_eq!(citation.part.as_deref(), Some("B"));
        assert_eq!(citation.spec.as_deref(), Some("401"));
        assert_eq!(citation.to_human(), "IHO S-4 4.10.0 B-401");

        let bare = parse("IHO B-401");
        assert_eq!(bare.kind, IhoKind::Standard);
        assert_eq!(bare.number, "4");
        assert_eq!(bare.to_human(), "IHO S-4 B-401");

        // The spelled section is the section-level citation (the `00`
        // slot): one identity with `IHO S-4 B-400`.
        let long = parse("IHO S-4 Part B, Section 400");
        assert_eq!(long, parse("IHO S-4 B-400"));
    }

    #[test]
    fn the_dual_edition_modes_stay_mutually_exclusive() {
        assert!(IhoIdentifier::parse("IHO S-100 1.0.0 Part 2 3.0.0").is_err());
        assert!(IhoIdentifier::parse("IHO P-6 1.0.0 Vol. 2 1.0.1").is_err());
    }

    // -----------------------------------------------------------------
    // Hash round-trips and the spelling face
    // -----------------------------------------------------------------

    #[test]
    fn from_hash_round_trips_and_ignores_the_face() {
        for input in [
            "IHO S-44 5.0.0",
            "IHO S-158 1.0.0",
            "IHO S-158:102 1.0.0",
            "IHO S-158 0.2.0 Part S-102",
            "IHO S-158 Part S-102 0.2.0",
            "IHO S-100 5.0.0 Part 2 (2024)",
            "IHO P-1 Vol. 30 (2)",
            "IHO S-4 4.10.0 B-401",
            "IHO S-98 Part A 1.0.0",
            "IHO S-122 1.0.0 Appendix D-2",
        ] {
            let id = parse(input);
            let hash = id.to_hash();
            let rebuilt =
                IhoIdentifier::from_hash(&hash).unwrap_or_else(|err| panic!("{hash}: {err}"));
            assert_eq!(rebuilt.to_hash(), hash, "{input}");
            // The canonical hash carries no spelling face, so the
            // rebuild renders the canonical colon designator for S-158.
            if id.product.is_some() {
                assert_eq!(rebuilt.product_face, None, "{input}");
            }
        }
    }

    #[test]
    fn a_draft_era_hash_re_keys_on_deserialization() {
        // The legacy serialized shape (part: S-102) re-keys into
        // product — the deserialization side of #774.
        let id = IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "158",
            "edition": "0.2.0",
            "part": "S-102",
        }))
        .expect("from_hash");
        assert_eq!(id.product.as_deref(), Some("102"));
        assert_eq!(id.part, None);
        assert_eq!(id.product_face, None);
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:iho:standard",
                "number": "158",
                "product": "102",
                "edition": "0.2.0",
            })
        );
        assert_eq!(id, parse("IHO S-158:102 0.2.0"));
        // The bare-number draft-era row re-keys too.
        let bare = IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "158",
            "part": "102",
        }))
        .expect("from_hash");
        assert_eq!(bare.product.as_deref(), Some("102"));
        // The member edition folds onto the publication.
        let mis_bound = IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "158",
            "part": "S-102",
            "part_edition": "0.2.0",
        }))
        .expect("from_hash");
        assert_eq!(mis_bound.edition.as_deref(), Some("0.2.0"));
        assert_eq!(mis_bound.part_edition, None);
        // A duplicate base + member edition raises.
        assert!(IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "158",
            "edition": "1.0.0",
            "part": "S-102",
            "part_edition": "0.2.0",
        }))
        .is_err());
    }

    #[test]
    fn from_hash_is_the_permissive_decode() {
        // The reference's from_hash runs the part-model unification
        // only: no checks anchor assert (a stored hash may carry
        // product on another anchor) and no mutual-exclusion raise
        // when product and part both ride.
        let off_anchor = IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "100",
            "product": "2",
        }))
        .expect("from_hash is permissive off the S-158 anchor");
        assert_eq!(off_anchor.product.as_deref(), Some("2"));

        let both = IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "158",
            "product": "102",
            "part": "S-103",
        }))
        .expect("from_hash keeps both components");
        assert_eq!(both.product.as_deref(), Some("102"));
        assert_eq!(both.part.as_deref(), Some("S-103"));
        // A member year yields to a base year on deserialization.
        let years = IhoIdentifier::from_hash(&json!({
            "_type": "pubid:iho:standard",
            "number": "158",
            "year": "2024",
            "part": "S-102",
            "part_year": "2023",
        }))
        .expect("from_hash");
        assert_eq!(years.year.as_deref(), Some("2024"));
        assert_eq!(years.part_year, None);
    }

    #[test]
    fn a_numbered_part_rides_the_colon_member() {
        // An untouched part face beside the checks member is a legal
        // shape: only the member faces re-key.
        let id = parse("IHO S-158:102 1.0.0 Part 2");
        assert_eq!(id.number, "158");
        assert_eq!(id.product.as_deref(), Some("102"));
        assert_eq!(id.part.as_deref(), Some("2"));
        assert_eq!(id.to_human(), "IHO S-158:102 1.0.0 Part 2");
        assert_eq!(urn_str(&id), "urn:mrn:iho:pub:spec:s158-102:1.0.0:part-2");
        let round = IhoIdentifier::parse(urn_str(&id).as_str()).expect("round");
        assert_eq!(round, id);
        let rebuilt = IhoIdentifier::from_hash(&id.to_hash()).expect("from_hash");
        assert_eq!(rebuilt.to_hash(), id.to_hash());
    }

    #[test]
    fn the_citation_requires_the_s4_anchor() {
        // A citation beside a non-S-4 anchor is a builder-level
        // rejection (the artifact never binds it): the artifact
        // rejects the shape outright.
        assert!(IhoIdentifier::parse("IHO S-44 B-401").is_err());
    }

    #[test]
    fn the_urn_schemes_agree_on_one_identity() {
        // Every scheme decodes to the same document.
        let human = parse("IHO S-158:102 1.0.0");
        let from_mrn = parse("urn:mrn:iho:pub:spec:s158-102:1.0.0");
        let from_retired = parse("urn:iho:s:158:102:ed-1.0.0");
        let from_pubid = parse("urn:pubid:iho:std:158:product-102:ed-1.0.0");
        assert_eq!(from_mrn, human);
        assert_eq!(from_retired, human);
        assert_eq!(from_pubid, human);
        assert_eq!(from_mrn.to_pubid_urn(), human.to_pubid_urn());
        assert_eq!(urn_str(&from_pubid), urn_str(&human));
    }

    #[test]
    fn the_spelling_face_never_reaches_the_hash_or_urns() {
        let colon = parse("IHO S-158:102 0.2.0");
        let expanded = parse("IHO S-158 0.2.0 Part S-102");
        assert_ne!(colon.product_face, expanded.product_face);
        assert_eq!(colon.to_hash(), expanded.to_hash());
        assert_eq!(urn_str(&colon), urn_str(&expanded));
        assert_eq!(colon.to_pubid_urn(), expanded.to_pubid_urn());
        assert_eq!(colon, expanded);
    }

    #[test]
    fn a_series_reference_part_keeps_its_slot_off_s158() {
        // The untouched legacy shape: a series-reference part on
        // another series stays a part (only the S-158 family re-keys).
        let other = parse("IHO S-98 Part S-102");
        assert_eq!(other.number, "98");
        assert_eq!(other.part.as_deref(), Some("S-102"));
        assert_eq!(other.product, None);
        // Any of the S/P/M/B/C series references re-keys on S-158.
        let widened = parse("IHO S-158 1.0.0 Part P-5");
        assert_eq!(widened.product.as_deref(), Some("5"));
        assert_eq!(widened.part, None);
    }
}
