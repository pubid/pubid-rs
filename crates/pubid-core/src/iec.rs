//! The iec flavor: the consolidated-form surface (`IEC 60601-1:2005+AMD1:2012+COR1:2012`,
//! the `+AMD+COR` composition chains the IEC catalogue emits), mirroring
//! pubid gem 2.0.56 at the consolidated-chains round (pubid#788, gem PR
//! #795).
//!
//! The scope is the chain composition, not the whole flavor: the base
//! document faces the chains ride on (the IS/TR/TS/PAS/SRD heads with
//! their copublishers), the chain supplements (Amendment / Corrigendum /
//! Interpretation-Sheet members with their ordinal and year), the VAP
//! deliverable wrapper (`CSV`, the consolidated edition), the attached
//! supplement on top of a chain or a single (`/AMD1:2012`,
//! `CSV/COR1:2014`), and the algebra the gem round landed —
//! [`IecIdentifier::drop_supplements`], the structural excludes and
//! their [`IecIdentifier::matches`] forms, [`IecIdentifier::edition_of`],
//! and the wrapper MR slugs (`plus-`-marked chain members, the Vap
//! deliverable codes, the supplement head composed on the base's own
//! slug — the part-drop collision fix).
//!
//! Every other IEC surface (drafts, fragments, working documents, TRFs,
//! all-parts, language faces, dotted designations) is out of scope: the
//! builder rejects those shapes with [`Error::InvalidHash`] naming the
//! capture, so an unsupported face is a loud rejection, never a silent
//! identity loss. The trailing-edition reject (gem #788, the #727
//! ruling) is parse-only: a bare chain carrying an edition facet is
//! rejected at build (no IEC source prints one and the canonical render
//! cannot re-parse), while [`IecIdentifier::from_hash`] stays open — a
//! stored member-edition chain still deserializes.
//!
//! The artifact is the contract: parse runs through the vendored
//! pubid-grammar envelope (`artifacts/iec.json`, byte-identical to the
//! gem's `data/parg/iec.json`), and the URN layout is the published
//! `urn:iec:std:` positional scheme the relaton-data-iec corpus pins:
//!
//! ```text
//! urn:iec:std:{authority}:{number}[-{part}[-{subpart}]]
//!     :{date}:{type}:{deliverable}:{language}{adjuncts}
//! ```
//!
//! where the adjuncts are `plus:amd:{n}:{y}` per chain member (the
//! `plus` relation marker the printed form spells `+`) and
//! `:{token}:{n}:{y}` per attached supplement. The bound
//! `number_with_part` capture splits on `-` into number/part/subpart
//! (the designation's segments); chains carry no dotted designations.

use serde_json::Value;

use crate::artifact::iec_artifact;
use crate::bound::{bound_str, capture_in, optional_at, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete iec class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IecKind {
    InternationalStandard,
    TechnicalReport,
    TechnicalSpecification,
    PubliclyAvailableSpecification,
    SocietalTechnologyTrendReport,
    Amendment,
    Corrigendum,
    InterpretationSheet,
}

use IecKind::*;

impl IecKind {
    /// The polymorphic `_type` tag (the gem's IEC_TYPE_MAP spellings).
    pub fn type_tag(self) -> &'static str {
        match self {
            InternationalStandard => "pubid:iec:international-standard",
            TechnicalReport => "pubid:iec:technical-report",
            TechnicalSpecification => "pubid:iec:technical-specification",
            PubliclyAvailableSpecification => "pubid:iec:publicly-available-specification",
            SocietalTechnologyTrendReport => "pubid:iec:societal-technology-trend-report",
            Amendment => "pubid:iec:amendment",
            Corrigendum => "pubid:iec:corrigendum",
            InterpretationSheet => "pubid:iec:interpretation-sheet",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [
            InternationalStandard,
            TechnicalReport,
            TechnicalSpecification,
            PubliclyAvailableSpecification,
            SocietalTechnologyTrendReport,
            Amendment,
            Corrigendum,
            InterpretationSheet,
        ]
        .into_iter()
        .find(|kind| kind.type_tag() == tag)
    }

    /// The head type token a bound `type_with_stage` carries (published
    /// heads only — the chain surface). `InternationalStandard` also
    /// answers the explicit `IS` token.
    fn from_head_token(token: &str) -> Option<Self> {
        match token.to_uppercase().as_str() {
            "TR" => Some(TechnicalReport),
            "TS" => Some(TechnicalSpecification),
            "PAS" => Some(PubliclyAvailableSpecification),
            "SRD" => Some(SocietalTechnologyTrendReport),
            "IS" => Some(InternationalStandard),
            _ => None,
        }
    }

    /// The supplement token a bound `supplement_type` /
    /// `supp_type_with_stage` capture carries.
    fn from_supplement_token(token: &str) -> Option<Self> {
        match token.to_uppercase().as_str() {
            "AMD" => Some(Amendment),
            "COR" => Some(Corrigendum),
            "ISH" => Some(InterpretationSheet),
            _ => None,
        }
    }

    /// A chain / attached supplement class.
    fn is_supplement(self) -> bool {
        matches!(self, Amendment | Corrigendum | InterpretationSheet)
    }

    /// The printed head abbreviation (`IEC TR 60721-4-1`); the IS head
    /// renders none.
    fn head_abbr(self) -> Option<&'static str> {
        match self {
            TechnicalReport => Some("TR"),
            TechnicalSpecification => Some("TS"),
            PubliclyAvailableSpecification => Some("PAS"),
            SocietalTechnologyTrendReport => Some("SRD"),
            _ => None,
        }
    }

    /// The printed supplement abbreviation (`/AMD1:2012`, `+COR1:2012`).
    fn supplement_abbr(self) -> &'static str {
        match self {
            Amendment => "AMD",
            Corrigendum => "COR",
            InterpretationSheet => "ISH",
            _ => "",
        }
    }

    /// The official-URN type slot token (`TYPE_SLOT_CODES`); the IS
    /// head and the supplements render the slot empty (`is` never
    /// occupies the slot).
    fn urn_type_slot(self) -> &'static str {
        match self {
            TechnicalReport => "tr",
            TechnicalSpecification => "ts",
            PubliclyAvailableSpecification => "pas",
            SocietalTechnologyTrendReport => "srd",
            _ => "",
        }
    }

    /// The official-URN adjunct token (`ADJUNCT_TOKENS`).
    fn urn_adjunct_token(self) -> &'static str {
        match self {
            Amendment => "amd",
            Corrigendum => "cor",
            InterpretationSheet => "ish",
            _ => "",
        }
    }
}

/// The base document a chain composes onto (`IEC 60601-1:2005`): the
/// publisher group, the designation segments, the date and — on a
/// single document, never on a chain member — the edition facet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecDocument {
    /// The concrete class (drives `_type`, the head abbreviation and
    /// the URN type slot).
    pub kind: IecKind,
    /// The publisher body (`IEC`, `ISO`, `CISPR`).
    pub publisher: String,
    /// The copublishers (`IEC` in `ISO/IEC`, `IEEE` in `IEC/IEEE`).
    pub copublishers: Vec<String>,
    /// The document number (`60601`).
    pub number: String,
    /// The dash part (`1` in `60601-1`).
    pub part: Option<String>,
    /// The dash subpart (`41` in `60034-18-41`).
    pub subpart: Option<String>,
    /// The publication date as captured (`2005`).
    pub date: Option<String>,
    /// The edition facet (`2` in `IEC 60068:2005 ED2`) — a single
    /// document's own face; the chain surfaces never set it.
    pub edition: Option<String>,
}

/// A chain / attached supplement: the class, the ordinal and the year
/// (`AMD1:2012`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IecSupplement {
    /// The concrete supplement class.
    pub kind: IecKind,
    /// The supplement ordinal (`1` in `AMD1:2012`).
    pub number: String,
    /// The supplement year (`2012`).
    pub date: Option<String>,
}

/// The iec identifier over the scoped surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IecIdentifier {
    /// A base document (`Identifiers::Base` in the gem).
    Document(IecDocument),
    /// An attached supplement (`SupplementIdentifier`): the class and
    /// ordinal, over the identifier it supplements.
    Supplement {
        /// The supplement's own faces.
        supplement: IecSupplement,
        /// The supplemented identifier (a document, a chain, or the VAP
        /// wrapper of either — `CSV/COR1:2014`).
        base: Box<IecIdentifier>,
    },
    /// The VAP deliverable wrapper (`VapIdentifier`): the codes (`CSV`,
    /// the consolidated edition) and the edition facet, over the
    /// composed chain or a single document.
    Vap {
        /// The deliverable codes (`CSV`; a multi-code face is
        /// `EXV-CMV`).
        codes: Vec<String>,
        /// The edition facet of the deliverable face (`CSV ED1`).
        edition: Option<String>,
        /// The wrapped identifier.
        base: Box<IecIdentifier>,
    },
    /// The consolidated chain (`ConsolidatedIdentifier`): the base
    /// member first, then the supplement members — the printed `+`
    /// composition.
    Consolidated {
        /// The base member (a document on the parse path).
        head: Box<IecIdentifier>,
        /// The supplement members (bare records — the composition is
        /// positional, the members carry no base of their own).
        supplements: Vec<IecSupplement>,
    },
}

/// The structural exclusion keys, mirroring the gem's `exclude` /
/// `matches?(ignore:)` vocabulary over the scoped surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ignore {
    /// Reduce a supplemented identifier to what it supplements
    /// (`drop_supplements`).
    Amendment,
    /// The same peel under the supplement spelling.
    Supplement,
    /// Drop the publication date(s), nested identifiers included.
    Date,
    /// Drop the edition facet(s), nested identifiers included.
    Edition,
}

/// The deliverable markers occupying the official-URN deliverable slot
/// (the gem's `DELIVERABLES`).
const DELIVERABLES: [&str; 6] = ["cmv", "csv", "exv", "prv", "rlv", "ser"];

/// The gem's consolidated-edition reject message (the #727 ruling
/// shape): the attestation record the rejection carries.
const CONSOLIDATED_EDITION_MESSAGE: &str =
    "the bare consolidated chain carries a trailing edition \
     (e.g. \"IEC 60068:2005+AMD1:2012 ED2\"): no IEC source prints one — 0 of the 33,070 primary \
     docidentifiers in relaton-data-iec carry an edition facet on a chain — and the canonical \
     render prints the edition onto the base designation (\"IEC 60068:2005 ED2+AMD1:2012\"), \
     which cannot be re-parsed. The consolidated edition is the CSV face \
     (\"IEC 60034-18-41:2014+AMD1:2019 CSV\", the OBP PDF named {ed1.1}); where the printed \
     edition appears it rides the CSV face (\"IEC 60068:2005+AMD1:2012 CSV ED1\"), which parses \
     and round-trips";

impl IecDocument {
    /// The designation segments the bound `number_with_part` splits
    /// into: number, part, subpart. A fourth dash segment glues back
    /// into the subpart (three segments is the deepest designation the
    /// flavor's numbers use).
    fn segments(number_with_part: &str) -> (String, Option<String>, Option<String>) {
        let mut parts = number_with_part.split('-');
        let number = parts.next().unwrap_or_default().to_string();
        let part = parts.next().map(str::to_string);
        let rest: Vec<&str> = parts.collect();
        let subpart = match rest.len() {
            0 => None,
            _ => Some(rest.join("-")),
        };
        (number, part, subpart)
    }

    /// The publisher portion: the publisher group joined `/`, then the
    /// head abbreviation with a space (`IEC TR`, `ISO/IEC`).
    fn publisher_portion(&self) -> String {
        let mut group = vec![self.publisher.clone()];
        group.extend(self.copublishers.iter().cloned());
        let mut portion = group.join("/");
        if let Some(abbr) = self.kind.head_abbr() {
            portion.push(' ');
            portion.push_str(abbr);
        }
        portion
    }

    /// The number portion: a leading space, the designation segments
    /// joined `-`, then `:{date}` (the gem's base `number_portion`
    /// prepends the publisher separator).
    fn number_portion(&self) -> String {
        let mut portion = format!(" {}", self.designation());
        if let Some(date) = &self.date {
            portion.push(':');
            portion.push_str(date);
        }
        portion
    }

    /// The designation the document carries: number-part-subpart, the
    /// segments as captured.
    fn designation(&self) -> String {
        let mut designation = vec![self.number.clone()];
        if let Some(part) = &self.part {
            designation.push(part.clone());
        }
        if let Some(subpart) = &self.subpart {
            designation.push(subpart.clone());
        }
        designation.join("-")
    }

    /// The official-URN authority slot: the publisher group's last
    /// whitespace tokens joined `-` (`iso-iec`, `iec-ieee`, `cispr`).
    fn authority(&self) -> String {
        let mut group = vec![self.publisher.clone()];
        group.extend(self.copublishers.iter().cloned());
        group
            .iter()
            .filter_map(|body| body.split_whitespace().last())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// The official-URN number segment: the designation with spaces
    /// folded to hyphens (the gem's multi-word rule).
    fn number_segment(&self) -> String {
        self.designation().replace([' ', '\t'], "-")
    }

    /// The MR slug segments (the gem's SEGMENTS hooks over the scoped
    /// surface): publisher, type, designation, year, edition.
    fn mr_segments(&self) -> Vec<String> {
        let mut segments = vec![self.publisher.to_lowercase().replace('/', "-")];
        // mr_type: the type code, omitted for the IS head (`is` never
        // occupies the slot).
        let type_code = self.kind.urn_type_slot();
        if !type_code.is_empty() {
            segments.push(type_code.to_string());
        }
        segments.push(self.designation().to_lowercase());
        if let Some(date) = &self.date {
            segments.push(date.to_lowercase());
        }
        if let Some(edition) = &self.edition {
            segments.push(format!("ed{edition}"));
        }
        segments
    }
}

impl IecSupplement {
    /// The bare member hash a consolidated chain serializes (no
    /// `base` — the composition is positional).
    fn member_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), self.number.clone().into());
        if let Some(date) = &self.date {
            object.insert("year".into(), date.clone().into());
        }
        Value::Object(object)
    }

    /// The human member face on a chain: `+AMD1:2012` (the date rides
    /// only when present).
    fn chain_face(&self) -> String {
        let mut face = format!("+{}", self.kind.supplement_abbr());
        face.push_str(&self.number);
        if let Some(date) = &self.date {
            face.push(':');
            face.push_str(date);
        }
        face
    }

    /// The MR suffix (`amd.1.2012`).
    fn mr_suffix(&self) -> String {
        let mut segments = vec![
            self.kind.urn_adjunct_token().to_string(),
            self.number.clone(),
        ];
        if let Some(date) = &self.date {
            segments.push(date.clone());
        }
        segments.join(".")
    }
}

impl IecIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact and run the
    /// builder cast over the bound captures.
    pub fn parse(input: &str) -> Result<Self> {
        let artifact = iec_artifact()?;
        let bound = artifact.parse_and_bind(Self::ENTRY, input)?;
        Self::build(&bound)
    }

    /// The builder cast over a bound hash.
    pub fn build(bound: &Value) -> Result<Self> {
        reject_out_of_scope(bound)?;
        let head = build_document(bound)?;
        let chain = chain_members(bound)?;
        let vap = optional_at(bound, "vap_suffix").map(deliverable_codes);
        let edition = optional_at(bound, "edition").map(str::to_string);
        let attached = attached_supplement(bound)?;

        // The trailing-edition reject (gem #788): a BARE chain with an
        // edition facet. The edition-bearing CSV face keeps parsing —
        // there the edition rides the Vap layer, not a chain member.
        if !chain.is_empty() && vap.is_none() && edition.is_some() {
            return Err(Error::InvalidHash {
                detail: CONSOLIDATED_EDITION_MESSAGE.to_string(),
            });
        }

        let inner = if chain.is_empty() {
            let mut document = head;
            // The edition belongs to the layer whose face prints it: a
            // bare single document's own facet.
            if vap.is_none() {
                document.edition = edition.clone();
            }
            IecIdentifier::Document(document)
        } else {
            IecIdentifier::Consolidated {
                head: Box::new(IecIdentifier::Document(head)),
                supplements: chain,
            }
        };
        let wrapped = match vap {
            Some(codes) => IecIdentifier::Vap {
                codes,
                edition,
                base: Box::new(inner),
            },
            None => inner,
        };
        Ok(match attached {
            Some(supplement) => IecIdentifier::Supplement {
                supplement,
                base: Box::new(wrapped),
            },
            None => wrapped,
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    /// Deliberately open where [`IecIdentifier::parse`] rejects: a
    /// stored member-edition chain deserializes (the parse-only
    /// reject's stored-data compatibility).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        match hash.get("_type").and_then(Value::as_str) {
            Some("pubid:iec:consolidated-identifier") => Self::consolidated_from_hash(hash),
            Some("pubid:iec:vap-identifier") => Self::vap_from_hash(hash),
            _ => Self::leaf_from_hash(hash),
        }
    }

    /// The document / attached-supplement shapes off a canonical hash
    /// (the `_type` dispatch over [`IecKind`]).
    fn leaf_from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(IecKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown iec _type {:?}", hash.get("_type")),
            })?;
        let read = |field: &'static str| optional_hash_str(hash, field).map(str::to_string);
        match kind {
            InternationalStandard
            | TechnicalReport
            | TechnicalSpecification
            | PubliclyAvailableSpecification
            | SocietalTechnologyTrendReport => Ok(IecIdentifier::Document(IecDocument {
                kind,
                publisher: read("publisher").unwrap_or_else(|| "IEC".to_string()),
                copublishers: hash
                    .get("copublishers")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                number: read("number").ok_or(Error::MissingAttribute { capture: "number" })?,
                part: read("part"),
                subpart: read("subpart"),
                date: read("year"),
                edition: read("edition"),
            })),
            Amendment | Corrigendum | InterpretationSheet => {
                let base = hash
                    .get("base")
                    .cloned()
                    .filter(|base| !base.is_null())
                    .map(|base| IecIdentifier::from_hash(&base))
                    .transpose()?
                    .ok_or(Error::MissingAttribute { capture: "base" })?;
                Ok(IecIdentifier::Supplement {
                    supplement: IecSupplement {
                        kind,
                        number: read("number")
                            .ok_or(Error::MissingAttribute { capture: "number" })?,
                        date: read("year"),
                    },
                    base: Box::new(base),
                })
            }
        }
    }

    /// The Vap wrapper off a canonical hash.
    fn vap_from_hash(hash: &Value) -> Result<Self> {
        let base = hash
            .get("base")
            .cloned()
            .filter(|base| !base.is_null())
            .map(|base| IecIdentifier::from_hash(&base))
            .transpose()?
            .ok_or(Error::MissingAttribute { capture: "base" })?;
        Ok(IecIdentifier::Vap {
            codes: hash
                .get("vap")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            edition: optional_hash_str(hash, "edition").map(str::to_string),
            base: Box::new(base),
        })
    }

    /// The consolidated chain off a canonical hash: the base member
    /// first, then the bare supplement members (no `base` key — the
    /// composition is positional).
    fn consolidated_from_hash(hash: &Value) -> Result<Self> {
        let members =
            hash.get("identifiers")
                .and_then(Value::as_array)
                .ok_or(Error::MissingAttribute {
                    capture: "identifiers",
                })?;
        let mut members = members.iter();
        let head = members
            .next()
            .ok_or(Error::MissingAttribute {
                capture: "identifiers",
            })
            .and_then(IecIdentifier::from_hash)?;
        let mut supplements = Vec::new();
        for member in members {
            supplements.push(supplement_from_member_hash(member)?);
        }
        Ok(IecIdentifier::Consolidated {
            head: Box::new(head),
            supplements,
        })
    }

    /// The canonical hash (`_type`-tagged attribute record). The
    /// "IEC" publisher and the empty copublisher list stay off the
    /// wire (the gem's attribute-default omission).
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        match self {
            IecIdentifier::Document(document) => {
                object.insert("_type".into(), document.kind.type_tag().into());
                if document.publisher != "IEC" {
                    object.insert("publisher".into(), document.publisher.clone().into());
                }
                if !document.copublishers.is_empty() {
                    object.insert(
                        "copublishers".into(),
                        Value::Array(
                            document
                                .copublishers
                                .iter()
                                .map(|body| body.clone().into())
                                .collect(),
                        ),
                    );
                }
                object.insert("number".into(), document.number.clone().into());
                for (field, value) in [
                    ("part", &document.part),
                    ("subpart", &document.subpart),
                    ("year", &document.date),
                    ("edition", &document.edition),
                ] {
                    if let Some(value) = value {
                        object.insert(field.into(), value.clone().into());
                    }
                }
            }
            IecIdentifier::Supplement { supplement, base } => {
                object.insert("_type".into(), supplement.kind.type_tag().into());
                object.insert("number".into(), supplement.number.clone().into());
                if let Some(date) = &supplement.date {
                    object.insert("year".into(), date.clone().into());
                }
                object.insert("base".into(), base.to_hash());
            }
            IecIdentifier::Vap {
                codes,
                edition,
                base,
            } => {
                object.insert("_type".into(), "pubid:iec:vap-identifier".into());
                object.insert(
                    "vap".into(),
                    Value::Array(codes.iter().map(|code| code.clone().into()).collect()),
                );
                if let Some(edition) = edition {
                    object.insert("edition".into(), edition.clone().into());
                }
                object.insert("base".into(), base.to_hash());
            }
            IecIdentifier::Consolidated { head, supplements } => {
                object.insert("_type".into(), "pubid:iec:consolidated-identifier".into());
                let mut members = vec![head.to_hash()];
                members.extend(supplements.iter().map(IecSupplement::member_hash));
                object.insert("identifiers".into(), Value::Array(members));
            }
        }
        Value::Object(object)
    }

    /// The human render (the gem's `:human` format over the scoped
    /// surface). Byte-exact on every attested chain face.
    pub fn to_human(&self) -> String {
        self.to_human_opts(true)
    }

    /// The human render under the `with_edition` flag: a Vap hides the
    /// wrapped edition (the deliverable face prints its own after the
    /// codes).
    fn to_human_opts(&self, with_edition: bool) -> String {
        match self {
            IecIdentifier::Document(document) => {
                let mut result = format!(
                    "{}{}",
                    document.publisher_portion(),
                    document.number_portion()
                );
                if with_edition {
                    if let Some(edition) = &document.edition {
                        result.push_str(" ED");
                        result.push_str(edition);
                    }
                }
                result
            }
            IecIdentifier::Consolidated { head, supplements } => {
                let mut result = head.to_human_opts(with_edition);
                for supplement in supplements {
                    result.push_str(&supplement.chain_face());
                }
                result
            }
            IecIdentifier::Vap {
                codes,
                edition,
                base,
            } => {
                let mut result = base.to_human_opts(false);
                result.push(' ');
                result.push_str(&codes.join("-"));
                if let Some(edition) = edition {
                    result.push_str(" ED");
                    result.push_str(edition);
                }
                result
            }
            IecIdentifier::Supplement { supplement, base } => {
                let mut result = base.to_human_opts(with_edition);
                result.push('/');
                result.push_str(supplement.kind.supplement_abbr());
                result.push_str(&supplement.number);
                if let Some(date) = &supplement.date {
                    result.push(':');
                    result.push_str(date);
                }
                result
            }
        }
    }

    /// The official `urn:iec:std:` URN: the positional scheme the
    /// relaton-data-iec corpus pins, byte-for-byte over the attested
    /// chain rows.
    pub fn to_urn(&self) -> String {
        let document = self.root_document();
        let slots = [
            "urn".to_string(),
            "iec".to_string(),
            "std".to_string(),
            document.authority(),
            document.number_segment(),
            document.date.clone().unwrap_or_default(),
            document.kind.urn_type_slot().to_string(),
            self.deliverable_slot(),
            // The language slot: the scoped surface binds no language
            // face, so the slot stays (the positional order is the
            // published corpus's contract).
            String::new(),
        ];
        let mut segments: Vec<String> = slots.to_vec();
        segments.extend(self.adjunct_fields());
        segments.join(":").to_lowercase()
    }

    /// The MR slug (the gem's `to_mr_string` over the wrapper
    /// compositions): the base member's own slug, each chain member
    /// behind a `plus-` marker, the Vap codes appended, and an
    /// attached supplement's suffix composed on the base's own slug —
    /// the part-drop collision fix.
    pub fn to_mr_string(&self) -> String {
        match self {
            IecIdentifier::Document(document) => document.mr_segments().join("."),
            IecIdentifier::Consolidated { head, supplements } => {
                let mut segments = vec![head.to_mr_string()];
                for supplement in supplements {
                    segments.push(format!("plus-{}", supplement.mr_suffix()));
                }
                segments.join("_")
            }
            IecIdentifier::Vap { codes, base, .. } => {
                let head = base.to_mr_string();
                let joined = codes.join("-").to_lowercase();
                if head.is_empty() {
                    joined
                } else {
                    format!("{head}_{joined}")
                }
            }
            IecIdentifier::Supplement { supplement, base } => {
                let head = base.to_mr_string();
                let suffix = supplement.mr_suffix();
                if head.is_empty() {
                    suffix
                } else {
                    format!("{head}_{suffix}")
                }
            }
        }
    }

    /// The identifier this one supplements, with its own supplement
    /// layer dropped: a chain peels to its base member, an attached
    /// supplement to its base; a document and a Vap supplement
    /// nothing.
    pub fn drop_supplements(&self) -> Self {
        match self {
            IecIdentifier::Document(_) | IecIdentifier::Vap { .. } => self.clone(),
            IecIdentifier::Supplement { base, .. } => (**base).clone(),
            IecIdentifier::Consolidated { head, .. } => (**head).clone(),
        }
    }

    /// A copy without the ignored aspects (the gem's `exclude` over the
    /// scoped surface): the structural keys peel through
    /// [`IecIdentifier::drop_supplements`]; `Date` / `Edition` nil the
    /// facet at every level, nested identifiers included.
    pub fn exclude(&self, ignore: &[Ignore]) -> Self {
        let structural: Vec<Ignore> = ignore
            .iter()
            .copied()
            .filter(|key| matches!(key, Ignore::Amendment | Ignore::Supplement))
            .collect();
        if !structural.is_empty() {
            let rest: Vec<Ignore> = ignore
                .iter()
                .copied()
                .filter(|key| !structural.contains(key))
                .collect();
            return self.drop_supplements().exclude(&rest);
        }
        let drop_date = ignore.contains(&Ignore::Date);
        let drop_edition = ignore.contains(&Ignore::Edition);
        if !drop_date && !drop_edition {
            return self.clone();
        }
        match self {
            IecIdentifier::Document(document) => {
                let mut document = document.clone();
                if drop_date {
                    document.date = None;
                }
                if drop_edition {
                    document.edition = None;
                }
                IecIdentifier::Document(document)
            }
            IecIdentifier::Supplement { supplement, base } => {
                let mut supplement = supplement.clone();
                if drop_date {
                    supplement.date = None;
                }
                IecIdentifier::Supplement {
                    supplement,
                    base: Box::new(base.exclude(ignore)),
                }
            }
            IecIdentifier::Vap {
                codes,
                edition,
                base,
            } => IecIdentifier::Vap {
                codes: codes.clone(),
                edition: if drop_edition { None } else { edition.clone() },
                base: Box::new(base.exclude(ignore)),
            },
            IecIdentifier::Consolidated { head, supplements } => IecIdentifier::Consolidated {
                head: Box::new(head.exclude(ignore)),
                supplements: supplements
                    .iter()
                    .map(|supplement| {
                        let mut supplement = supplement.clone();
                        if drop_date {
                            supplement.date = None;
                        }
                        supplement
                    })
                    .collect(),
            },
        }
    }

    /// Whether two identifiers match after excluding `ignore` (the
    /// gem's `matches?(other, ignore:)` — the primitive a catalogue
    /// lookup uses at varying strictness).
    pub fn matches(&self, other: &IecIdentifier, ignore: &[Ignore]) -> bool {
        self.exclude(ignore) == other.exclude(ignore)
    }

    /// The `[publisher, number, part]` tuple naming the document this
    /// identifier belongs to, resolved through wrappers.
    pub fn document_identity(&self) -> (String, String, Option<String>) {
        match self {
            IecIdentifier::Document(document) => (
                document.publisher.clone(),
                document.number.clone(),
                document.part.clone(),
            ),
            IecIdentifier::Supplement { base, .. } | IecIdentifier::Vap { base, .. } => {
                base.document_identity()
            }
            IecIdentifier::Consolidated { head, .. } => head.document_identity(),
        }
    }

    /// Whether self and other are editions of the same document — two
    /// consolidated states of one base resolve through their wrapped
    /// documents (`IEC 60601-1:2005+AMD1:2015` is an edition of
    /// `…+AMD1:2012+COR1:2012`).
    pub fn edition_of(&self, other: &IecIdentifier) -> bool {
        if self == other {
            return false;
        }
        self.document_identity() == other.document_identity()
    }

    /// The document a wrapper stands in front of, peeled recursively:
    /// an attached supplement's base, a Vap's base, a chain's base
    /// member (itself a supplement layer, so the peel continues — the
    /// corrigendum-over-CSV peel reaches the bare standard).
    pub fn base_document(&self) -> Self {
        match self {
            IecIdentifier::Supplement { base, .. } => base.base_document(),
            IecIdentifier::Vap { base, .. } => base.base_document(),
            IecIdentifier::Consolidated { head, .. } => head.base_document(),
            IecIdentifier::Document(_) => self.clone(),
        }
    }

    /// The origin document, with every supplement, VAP and chain layer
    /// peeled (the gem's `root` — what the official URN names).
    fn root_document(&self) -> IecDocument {
        match self {
            IecIdentifier::Document(document) => document.clone(),
            IecIdentifier::Supplement { base, .. } | IecIdentifier::Vap { base, .. } => {
                base.root_document()
            }
            IecIdentifier::Consolidated { head, .. } => head.root_document(),
        }
    }

    /// The official-URN deliverable slot: the Vap codes when a Vap
    /// layer exists (looked for down the chain, the gem's `find_down`),
    /// else the outermost layer's own edition as `ed-{n}`, else empty.
    fn deliverable_slot(&self) -> String {
        let codes = self.vap_codes();
        if !codes.is_empty() {
            return codes;
        }
        match self.outermost_edition() {
            Some(edition) => format!("ed-{edition}"),
            None => String::new(),
        }
    }

    /// The Vap codes found down the wrapper chain, filtered to the
    /// deliverable markers and joined `-`.
    fn vap_codes(&self) -> String {
        match self {
            IecIdentifier::Vap { codes, base, .. } => {
                let deliverables: Vec<String> = codes
                    .iter()
                    .map(|code| code.to_lowercase())
                    .filter(|code| DELIVERABLES.contains(&code.as_str()))
                    .collect();
                if deliverables.is_empty() {
                    base.vap_codes()
                } else {
                    deliverables.join("-")
                }
            }
            IecIdentifier::Supplement { base, .. } => base.vap_codes(),
            IecIdentifier::Consolidated { head, .. } => head.vap_codes(),
            IecIdentifier::Document(_) => String::new(),
        }
    }

    /// The outermost layer's own edition facet (the gem reads
    /// `identifier.edition`, not a walk — the deliverable fallback).
    fn outermost_edition(&self) -> Option<String> {
        match self {
            IecIdentifier::Document(document) => document.edition.clone(),
            IecIdentifier::Vap { edition, .. } => edition.clone(),
            _ => None,
        }
    }

    /// The official-URN adjunct fields, innermost first: a chain's
    /// base-member adjuncts (none) then one `plus`-marked field group
    /// per member; an attached supplement composes on its base's
    /// fields with the bare token form.
    fn adjunct_fields(&self) -> Vec<String> {
        match self {
            IecIdentifier::Document(_) => Vec::new(),
            IecIdentifier::Consolidated { head, supplements } => {
                let mut fields = head.adjunct_fields();
                for supplement in supplements {
                    fields.extend(supplement.adjunct_fields(true));
                }
                fields
            }
            IecIdentifier::Supplement { supplement, base } => {
                let mut fields = base.adjunct_fields();
                fields.extend(supplement.adjunct_fields(false));
                fields
            }
            IecIdentifier::Vap { base, .. } => base.adjunct_fields(),
        }
    }
}

impl IecSupplement {
    /// The official-URN adjunct field group: `plus` rides when the
    /// member is a chain composition (the printed `+` relation), the
    /// attached form leaves the marker slot empty.
    fn adjunct_fields(&self, plus: bool) -> [String; 4] {
        [
            if plus {
                "plus".to_string()
            } else {
                String::new()
            },
            self.kind.urn_adjunct_token().to_string(),
            self.number.clone(),
            self.date.clone().unwrap_or_default(),
        ]
    }
}

/// The bare supplement member off a consolidated hash (no `base` key
/// — the composition is positional).
fn supplement_from_member_hash(hash: &Value) -> Result<IecSupplement> {
    let kind = hash
        .get("_type")
        .and_then(Value::as_str)
        .and_then(IecKind::from_type_tag)
        .filter(|kind| kind.is_supplement())
        .ok_or_else(|| Error::InvalidHash {
            detail: format!(
                "a consolidated member is not a supplement: {:?}",
                hash.get("_type")
            ),
        })?;
    if hash.get("base").is_some_and(|base| !base.is_null()) {
        return Err(Error::InvalidHash {
            detail: "a consolidated member carries no base of its own".to_string(),
        });
    }
    Ok(IecSupplement {
        kind,
        number: optional_hash_str(hash, "number")
            .map(str::to_string)
            .ok_or(Error::MissingAttribute { capture: "number" })?,
        date: optional_hash_str(hash, "year").map(str::to_string),
    })
}

/// The bound-hash reads: shapes outside the consolidated-chain scope
/// are rejected by name, never silently dropped.
fn reject_out_of_scope(bound: &Value) -> Result<()> {
    const OUT_OF_SCOPE: [&str; 16] = [
        "languages",
        "all_parts",
        "database",
        "stage_iteration",
        "supp_stage_iteration",
        "supp_edition",
        "sheet_number",
        "sheet_year",
        "fragment_number",
        "fragment_type",
        "wd_number",
        "wd_stage",
        "wd_language",
        "wp_stage",
        "wp_type",
        "trf_org",
    ];
    for capture in OUT_OF_SCOPE {
        if bound.get(capture).is_some_and(|value| !value.is_null()) {
            return Err(Error::InvalidHash {
                detail: format!("the {capture:?} facet is outside the consolidated-chain surface"),
            });
        }
    }
    Ok(())
}

/// The base document off the bound captures.
fn build_document(bound: &Value) -> Result<IecDocument> {
    // A published head token maps to its class; a token the scoped
    // surface does not carry (a draft stage, a Guide form) is a loud
    // rejection, never a silent default to the IS head.
    let kind = match optional_at(bound, "type_with_stage") {
        Some(token) => IecKind::from_head_token(token).ok_or_else(|| Error::InvalidHash {
            detail: format!("unsupported head type token {token:?}"),
        })?,
        None => InternationalStandard,
    };
    let (number, part, subpart) = IecDocument::segments(bound_str(bound, "number_with_part")?);
    Ok(IecDocument {
        kind,
        publisher: optional_at(bound, "publisher")
            .map(str::to_string)
            .unwrap_or_else(|| "IEC".to_string()),
        copublishers: bound
            .get("copublishers")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| capture_in(item, "copublisher").map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        number,
        part,
        subpart,
        date: optional_at(bound, "date").map(str::to_string),
        edition: None,
    })
}

/// The chain members off the `consolidated_supplements` captures.
fn chain_members(bound: &Value) -> Result<Vec<IecSupplement>> {
    let mut members = Vec::new();
    let Some(items) = bound
        .get("consolidated_supplements")
        .and_then(Value::as_array)
    else {
        return Ok(members);
    };
    for item in items {
        let kind = optional_at(item, "supplement_type")
            .and_then(IecKind::from_supplement_token)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!(
                    "unsupported consolidated supplement token {:?}",
                    optional_at(item, "supplement_type").unwrap_or("")
                ),
            })?;
        members.push(IecSupplement {
            kind,
            number: optional_at(item, "supplement_number")
                .map(str::to_string)
                .ok_or(Error::MissingAttribute {
                    capture: "supplement_number",
                })?,
            date: optional_at(item, "supplement_year").map(str::to_string),
        });
    }
    Ok(members)
}

/// The attached supplement off the `supp_*` captures (the `/AMD1:2012`
/// and `CSV/COR1:2014` faces).
fn attached_supplement(bound: &Value) -> Result<Option<IecSupplement>> {
    let Some(token) = optional_at(bound, "supp_type_with_stage") else {
        return Ok(None);
    };
    let kind = IecKind::from_supplement_token(token).ok_or_else(|| Error::InvalidHash {
        detail: format!("unsupported attached supplement token {token:?}"),
    })?;
    Ok(Some(IecSupplement {
        kind,
        number: optional_at(bound, "supp_number_with_part")
            .map(str::to_string)
            .ok_or(Error::MissingAttribute {
                capture: "supp_number_with_part",
            })?,
        date: optional_at(bound, "supp_date").map(str::to_string),
    }))
}

/// The deliverable codes off the `vap_suffix` capture (`CSV`; a
/// multi-code face is `EXV-CMV`).
fn deliverable_codes(suffix: &str) -> Vec<String> {
    suffix.split('-').map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::{IecDocument, IecIdentifier, IecKind, IecSupplement};
    use crate::iec::Ignore;

    /// One real row per attested shape family (the gem's harvest off
    /// the relaton-data-iec records — the strings are copied from the
    /// data, not invented): the input, its PUBLISHED official URN, and
    /// the MR slug where the gem spec pins one.
    const ATTESTED_ROWS: [(&str, &str); 14] = [
        (
            "IEC 60034-18-41:2014+AMD1:2019 CSV",
            "urn:iec:std:iec:60034-18-41:2014::csv::plus:amd:1:2019",
        ),
        (
            "IEC 60038:2009+AMD1:2021 CSV",
            "urn:iec:std:iec:60038:2009::csv::plus:amd:1:2021",
        ),
        (
            "IEC 60068-2-6:1982+AMD1:1983+AMD2:1985 CSV",
            "urn:iec:std:iec:60068-2-6:1982::csv::plus:amd:1:1983:plus:amd:2:1985",
        ),
        (
            "IEC 60034-1:1969+AMD1:1977+AMD2:1979+AMD3:1980 CSV",
            "urn:iec:std:iec:60034-1:1969::csv::plus:amd:1:1977:plus:amd:2:1979:plus:amd:3:1980",
        ),
        (
            "IEC 60335-1:2010+AMD1:2013 CSV/COR1:2014",
            "urn:iec:std:iec:60335-1:2010::csv::plus:amd:1:2013::cor:1:2014",
        ),
        (
            "IEC 60287-1-1:1994+AMD1:1995+AMD2:2001 CSV/COR1:2006",
            "urn:iec:std:iec:60287-1-1:1994::csv::plus:amd:1:1995:plus:amd:2:2001::cor:1:2006",
        ),
        (
            "CISPR 11:1975+AMD1:1976 CSV",
            "urn:iec:std:cispr:11:1975::csv::plus:amd:1:1976",
        ),
        (
            "CISPR 16-1-1:2006+AMD1:2006+AMD2:2007 CSV",
            "urn:iec:std:cispr:16-1-1:2006::csv::plus:amd:1:2006:plus:amd:2:2007",
        ),
        (
            "IEC TR 60721-4-1:2001+AMD1:2003 CSV",
            "urn:iec:std:iec:60721-4-1:2001:tr:csv::plus:amd:1:2003",
        ),
        (
            "IEC TS 60479-1:2005+AMD1:2016 CSV",
            "urn:iec:std:iec:60479-1:2005:ts:csv::plus:amd:1:2016",
        ),
        (
            "ISO/IEC 11801-1:2017+AMD1:2025 CSV",
            "urn:iec:std:iso-iec:11801-1:2017::csv::plus:amd:1:2025",
        ),
        (
            "IEC/IEEE 62209-1528:2020+AMD1:2026 CSV",
            "urn:iec:std:iec-ieee:62209-1528:2020::csv::plus:amd:1:2026",
        ),
        (
            "IEC/ASTM 62885-7:2020+AMD1:2022 CSV",
            "urn:iec:std:iec-astm:62885-7:2020::csv::plus:amd:1:2022",
        ),
        (
            "ISO/IEC TR 29106:2007+AMD1:2012+AMD2:2019 CSV",
            "urn:iec:std:iso-iec:29106:2007:tr:csv::plus:amd:1:2012:plus:amd:2:2019",
        ),
    ];

    #[test]
    fn the_attested_catalogue_rows_round_trip() {
        for (input, urn) in ATTESTED_ROWS {
            let id = IecIdentifier::parse(input).unwrap_or_else(|error| panic!("{input}: {error}"));
            assert_eq!(
                id.to_human(),
                input,
                "{input}: the render is not byte-exact"
            );
            // The render is a fixed point: it re-parses to the same
            // identifier.
            let reparsed = IecIdentifier::parse(&id.to_human())
                .unwrap_or_else(|error| panic!("{input}: the render does not re-parse: {error}"));
            assert_eq!(reparsed, id, "{input}: the render does not re-parse");
            // The canonical hash round-trips through from_hash.
            let rebuilt = IecIdentifier::from_hash(&id.to_hash())
                .unwrap_or_else(|error| panic!("{input}: from_hash failed: {error}"));
            assert_eq!(rebuilt, id, "{input}: from_hash(to_hash) != parse");
            // The published official URN, byte-for-byte.
            assert_eq!(id.to_urn(), urn, "{input}: the official URN moved");
        }
    }

    #[test]
    fn the_chain_composes_typed_members_onto_the_base() {
        let id = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        let IecIdentifier::Consolidated { head, supplements } = &id else {
            panic!("the chain face builds a Consolidated");
        };
        let IecIdentifier::Document(head_document) = &**head else {
            panic!("the chain head is a document");
        };
        assert_eq!(head_document.kind, IecKind::InternationalStandard);
        assert_eq!(head.to_human(), "IEC 60601-1:2005");
        assert_eq!(supplements.len(), 2);
        assert_eq!(supplements[0].kind, IecKind::Amendment);
        assert_eq!(supplements[0].number, "1");
        assert_eq!(supplements[0].date.as_deref(), Some("2012"));
        assert_eq!(supplements[1].kind, IecKind::Corrigendum);
        assert_eq!(supplements[1].number, "1");
        assert_eq!(supplements[1].date.as_deref(), Some("2012"));
    }

    #[test]
    fn the_bare_chain_face_emits_the_published_urn() {
        let id = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        assert_eq!(
            id.to_urn(),
            "urn:iec:std:iec:60601-1:2005::::plus:amd:1:2012:plus:cor:1:2012"
        );
    }

    #[test]
    fn a_hand_built_chain_equals_the_parsed_one() {
        let built = IecIdentifier::Consolidated {
            head: Box::new(IecIdentifier::Document(IecDocument {
                kind: IecKind::InternationalStandard,
                publisher: "IEC".to_string(),
                copublishers: Vec::new(),
                number: "60601".to_string(),
                part: Some("1".to_string()),
                subpart: None,
                date: Some("2005".to_string()),
                edition: None,
            })),
            supplements: vec![
                IecSupplement {
                    kind: IecKind::Amendment,
                    number: "1".to_string(),
                    date: Some("2012".to_string()),
                },
                IecSupplement {
                    kind: IecKind::Corrigendum,
                    number: "1".to_string(),
                    date: Some("2012".to_string()),
                },
            ],
        };
        assert_eq!(built.to_human(), "IEC 60601-1:2005+AMD1:2012+COR1:2012");
        assert_eq!(
            built,
            IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse")
        );
    }

    #[test]
    fn the_chain_members_carry_their_own_years_and_ordinals() {
        let id = IecIdentifier::parse("IEC 60068-2-6:1982+AMD1:1983+AMD2:1985 CSV").expect("parse");
        let IecIdentifier::Vap { base, .. } = &id else {
            panic!("the CSV face builds a Vap");
        };
        let IecIdentifier::Consolidated { supplements, .. } = &**base else {
            panic!("the Vap wraps the chain");
        };
        assert_eq!(supplements[0].number, "1");
        assert_eq!(supplements[0].date.as_deref(), Some("1983"));
        assert_eq!(supplements[1].number, "2");
        assert_eq!(supplements[1].date.as_deref(), Some("1985"));
    }

    // -- the structural algebra (drop_supplements) ---------------------

    #[test]
    fn the_chain_drops_to_the_base_standard_alone() {
        let base = IecIdentifier::parse("IEC 60601-1:2005").expect("parse");
        let cons = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        assert_eq!(cons.drop_supplements().to_human(), "IEC 60601-1:2005");
        assert_eq!(cons.drop_supplements(), base);
    }

    #[test]
    fn the_structural_exclusions_peel_the_chain() {
        let base = IecIdentifier::parse("IEC 60601-1:2005").expect("parse");
        let cons = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        assert_eq!(cons.exclude(&[Ignore::Amendment]), base);
        assert_eq!(cons.exclude(&[Ignore::Supplement]), base);
    }

    #[test]
    fn the_base_record_matches_under_the_structural_ignores() {
        let base = IecIdentifier::parse("IEC 60601-1:2005").expect("parse");
        let cons = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        assert!(base.matches(&cons, &[Ignore::Amendment]));
        assert!(base.matches(&cons, &[Ignore::Supplement]));
        assert!(cons.matches(&base, &[Ignore::Amendment]));
    }

    #[test]
    fn the_chain_stays_distinct_without_the_ignores() {
        let base = IecIdentifier::parse("IEC 60601-1:2005").expect("parse");
        let cons = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        assert!(!base.matches(&cons, &[]));
        assert_ne!(base, cons);
    }

    #[test]
    fn two_consolidated_states_of_one_base_are_editions() {
        let cons = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2012+COR1:2012").expect("parse");
        let later = IecIdentifier::parse("IEC 60601-1:2005+AMD1:2015").expect("parse");
        assert!(later.edition_of(&cons));
    }

    #[test]
    fn the_corrigendum_over_csv_peels_at_the_wrapped_document() {
        // The supplements keep themselves in base_document; the peel
        // reaches through the chain behind the Vap.
        let csv_cor =
            IecIdentifier::parse("IEC 60335-1:2010+AMD1:2013 CSV/COR1:2014").expect("parse");
        let IecIdentifier::Supplement { base, .. } = &csv_cor else {
            panic!("the CSV/COR face builds an attached supplement");
        };
        assert_eq!(base.base_document().to_human(), "IEC 60335-1:2010");
    }

    // -- the MR slug (the wrapper slugs and the part-drop fix) ---------

    #[test]
    fn the_mr_slugs_compose() {
        let cases = [
            ("IEC 60601-1:2005", "iec.60601-1.2005"),
            (
                "IEC 60601-1:2005+AMD1:2012+COR1:2012",
                "iec.60601-1.2005_plus-amd.1.2012_plus-cor.1.2012",
            ),
            (
                "IEC 60601-1:2005+AMD1:2012 CSV",
                "iec.60601-1.2005_plus-amd.1.2012_csv",
            ),
            (
                "IEC 60601-1:2005+AMD1:2013 CSV/COR1:2014",
                "iec.60601-1.2005_plus-amd.1.2013_csv_cor.1.2014",
            ),
            ("IEC 60038:2009 CSV", "iec.60038.2009_csv"),
            (
                "IEC 60034-1:1969+AMD1:1977+AMD2:1979 CSV",
                "iec.60034-1.1969_plus-amd.1.1977_plus-amd.2.1979_csv",
            ),
            // The standalone chain keeps its own, un-`plus`-ed slug.
            ("IEC 60601-1:2005/Amd 1:2012", "iec.60601-1.2005_amd.1.2012"),
        ];
        for (input, mr) in cases {
            let id = IecIdentifier::parse(input).unwrap_or_else(|error| panic!("{input}: {error}"));
            assert_eq!(id.to_mr_string(), mr, "{input}: the slug moved");
        }
    }

    #[test]
    fn distinct_identifiers_key_distinct_slugs() {
        let parse = |input| IecIdentifier::parse(input).expect("parse");
        // The part-drop collision: two DIFFERENT documents never share
        // one slug.
        assert_ne!(
            parse("IEC 60601-1:2005+AMD1:2012 CSV").to_mr_string(),
            parse("IEC 60601-2:2005+AMD1:2012 CSV").to_mr_string()
        );
        // The bare base and its consolidated edition never share one.
        assert_ne!(
            parse("IEC 60601-1:2005").to_mr_string(),
            parse("IEC 60601-1:2005+AMD1:2012 CSV").to_mr_string()
        );
        // The consolidated chain and the standalone chain never share
        // one.
        assert_ne!(
            parse("IEC 60601-1:2005+AMD1:2012 CSV").to_mr_string(),
            parse("IEC 60601-1:2005/AMD1:2012").to_mr_string()
        );
    }

    // -- the un-renderable trailing-edition chain (the #727 ruling) ----

    #[test]
    fn the_bare_chain_with_a_trailing_edition_is_rejected() {
        let error = IecIdentifier::parse("IEC 60068:2005+AMD1:2012 ED2").expect_err("rejected");
        assert!(error.to_string().contains("trailing edition"), "{error}");
    }

    #[test]
    fn the_reject_covers_a_longer_chain_too() {
        let error =
            IecIdentifier::parse("IEC 60068:2005+AMD1:2012+COR1:2012 ED2").expect_err("rejected");
        assert!(error.to_string().contains("trailing edition"), "{error}");
    }

    #[test]
    fn the_edition_bearing_csv_face_keeps_parsing() {
        let id = IecIdentifier::parse("IEC 60068:2005+AMD1:2012 CSV ED1").expect("parse");
        assert_eq!(id.to_human(), "IEC 60068:2005+AMD1:2012 CSV ED1");
        assert_eq!(IecIdentifier::parse(&id.to_human()).expect("re-parse"), id);
        assert_eq!(
            id.to_urn(),
            "urn:iec:std:iec:60068:2005::csv::plus:amd:1:2012"
        );
    }

    #[test]
    fn a_member_edition_less_chain_parses() {
        let id = IecIdentifier::parse("IEC 60068:2005+AMD1:2012").expect("parse");
        assert!(matches!(id, IecIdentifier::Consolidated { .. }));
    }

    #[test]
    fn a_stored_member_edition_chain_still_deserializes() {
        // The reject is parse-only (the #727 stored-data compatibility):
        // parse can no longer produce the shape, from_hash stays open.
        let hash = serde_json::json!({
            "_type": "pubid:iec:consolidated-identifier",
            "identifiers": [
                {
                    "_type": "pubid:iec:international-standard",
                    "number": "60068",
                    "year": "2005",
                    "edition": "2"
                },
                { "_type": "pubid:iec:amendment", "number": "1", "year": "2012" },
            ],
        });
        let id = IecIdentifier::from_hash(&hash).expect("from_hash stays open");
        let IecIdentifier::Consolidated { head, .. } = &id else {
            panic!("the stored chain deserializes to a Consolidated");
        };
        let IecIdentifier::Document(document) = &**head else {
            panic!("the stored head is a document");
        };
        assert_eq!(document.edition.as_deref(), Some("2"));
    }

    // -- the malformed chains (the grammar admits no weaker face) ------

    #[test]
    fn the_malformed_chains_are_rejected() {
        for input in [
            "IEC 60068:2005+AMD",
            "IEC 60068:2005+COR",
            "IEC 60068:2005+",
            "IEC 60068:2005+AMD1:20",
            "IEC 60068:2005+AMD1:2012 CSV/",
            "IEC 60068:2005+AMD1:2012 CSV CSV",
        ] {
            assert!(
                IecIdentifier::parse(input).is_err(),
                "{input}: every attested chain supplement carries an ordinal and a year"
            );
        }
    }

    // -- the scoped surface: out-of-scope faces are loud rejections -----

    #[test]
    fn out_of_scope_faces_are_rejected_by_name() {
        // The draft head token is not a chain-surface face.
        let error = IecIdentifier::parse("IEC CD 60038").expect_err("rejected");
        assert!(error.to_string().contains("token"), "{error}");
    }

    #[test]
    fn the_attached_amendment_face_round_trips() {
        let id = IecIdentifier::parse("IEC 60601-1:2005/AMD1:2012").expect("parse");
        assert_eq!(id.to_human(), "IEC 60601-1:2005/AMD1:2012");
        // The long spelling collapses onto the uniform glued face (the
        // gem's rendering rule).
        assert_eq!(
            IecIdentifier::parse("IEC 60601-1:2005/Amd 1:2012")
                .expect("parse")
                .to_human(),
            "IEC 60601-1:2005/AMD1:2012"
        );
        assert_eq!(id.to_urn(), "urn:iec:std:iec:60601-1:2005:::::amd:1:2012");
    }

    #[test]
    fn a_single_document_edition_takes_the_deliverable_slot() {
        let id = IecIdentifier::parse("IEC 60068:2005 ED2").expect("parse");
        assert_eq!(id.to_human(), "IEC 60068:2005 ED2");
        assert_eq!(id.to_urn(), "urn:iec:std:iec:60068:2005::ed-2:");
    }
}
