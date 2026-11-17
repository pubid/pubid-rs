//! The plateau flavor: the MLIT PLATEAU handbooks and technical
//! reports (`PLATEAU Handbook #00 第1.0版`, `PLATEAU Technical Report
//! #00`, the `-1` annexes).
//!
//! The artifact binds `type` (`Handbook` / `TechnicalReport`),
//! `number`, `annex`, and the `第X.Y版` edition; the volume label the
//! MLIT prints after the edition (`本編` — the main volume — or
//! `本編＋付録` — main volume with appendix) extracts from the input
//! tail before the parse (the artifact carries no volume rule), with
//! the ASCII `+` normalized to the full-width `＋` and the
//! full-width (U+3000) space accepted as the separator. A legacy
//! Latin edition captured on a Technical Report drops (builder
//! parity). Numbers and annexes are INTEGERs on the wire; the URN
//! pads both to two digits (`urn:plateau:handbook:03:01`).

use serde_json::Value;

use crate::artifact::plateau_artifact;
use crate::bound::{optional_at, optional_hash_str};
use crate::error::{Error, Result};

/// What concrete plateau class an identifier belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateauKind {
    Handbook,
    TechnicalReport,
}

use PlateauKind::*;

impl PlateauKind {
    /// The shared `_type` prefix every plateau tag carries (the enum's
    /// dispatch arm in the conformance router matches on it).
    pub const KIND_PREFIX: &'static str = "pubid:plateau:";

    /// The polymorphic type tag carried in the canonical hash.
    pub fn type_tag(self) -> &'static str {
        match self {
            Handbook => "pubid:plateau:handbook",
            TechnicalReport => "pubid:plateau:technical-report",
        }
    }

    fn from_type_tag(tag: &str) -> Option<Self> {
        [Handbook, TechnicalReport]
            .into_iter()
            .find(|kind| kind.type_tag() == tag)
    }
}

/// The plateau identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlateauIdentifier {
    /// The concrete class (drives `_type` and the render shape).
    pub kind: PlateauKind,
    /// The document number (`3` in `#03`).
    pub number: u32,
    /// The annex number (`1` in `#03-1`).
    pub annex: Option<u32>,
    /// The `第X.Y版` edition (handbook only).
    pub edition: Option<String>,
    /// The volume label (`本編`, `本編＋付録`; handbook only).
    pub volume: Option<String>,
}

/// The volume-label tail: a separator (ASCII or full-width U+3000),
/// `本編`, optionally `＋付録` (full-width or ASCII `+`).
fn volume_tail(input: &str) -> Option<(&str, &str)> {
    for separator in [" ", "\u{3000}"] {
        let mut body = input.strip_suffix("本編＋付録").and_then(|rest| {
            rest.strip_suffix(separator)
                .map(|body| (body, "本編＋付録"))
        });
        if body.is_none() {
            body = input.strip_suffix("本編+付録").and_then(|rest| {
                rest.strip_suffix(separator)
                    .map(|body| (body, "本編＋付録"))
            });
        }
        if body.is_none() {
            body = input
                .strip_suffix("本編")
                .and_then(|rest| rest.strip_suffix(separator).map(|body| (body, "本編")));
        }
        if let Some(found) = body {
            return Some(found);
        }
    }
    None
}

/// Zero-pad a number face to two digits (`3` → `03`).
fn pad2(value: u32) -> String {
    format!("{value:02}")
}

impl PlateauIdentifier {
    /// The artifact entry the grammar exposes.
    pub const ENTRY: &'static str = "identifier";

    /// Parse an input through the vendored artifact; the volume tail
    /// extracts before the parse.
    pub fn parse(input: &str) -> Result<Self> {
        let (body, volume) = match volume_tail(input) {
            Some((body, face)) => (body, Some(face.to_string())),
            None => (input, None),
        };
        let artifact = plateau_artifact()?;
        let bound = match artifact.parse_and_bind(Self::ENTRY, body) {
            Ok(bound) => bound,
            // Without a strip the input must parse by itself; a volume
            // tail that masked a real parse failure re-raises here.
            Err(err) if volume.is_some() => {
                return Err(Error::Parse(err));
            }
            Err(err) => return Err(Error::Parse(err)),
        };

        let kind = match optional_at(&bound, "type") {
            Some("Handbook") => Handbook,
            _ => TechnicalReport,
        };
        let number = optional_at(&bound, "number")
            .ok_or(Error::MissingAttribute { capture: "number" })?
            .parse::<u32>()
            .map_err(|_| Error::InvalidHash {
                detail: format!("plateau number is not an integer: {bound}"),
            })?;
        let annex = optional_at(&bound, "annex").and_then(|annex| annex.parse::<u32>().ok());
        // The edition and the volume label are handbook faces; the
        // legacy Latin TR edition drops (builder parity).
        let edition = (kind == Handbook)
            .then(|| optional_at(&bound, "edition").map(str::to_string))
            .flatten();
        Ok(Self {
            kind,
            number,
            annex,
            edition,
            volume: (kind == Handbook).then_some(volume).flatten(),
        })
    }

    /// Build from a canonical hash (`_type`-tagged attribute record).
    pub fn from_hash(hash: &Value) -> Result<Self> {
        let kind = hash
            .get("_type")
            .and_then(Value::as_str)
            .and_then(PlateauKind::from_type_tag)
            .ok_or_else(|| Error::InvalidHash {
                detail: format!("unknown plateau _type {:?}", hash.get("_type")),
            })?;
        let read_int = |field: &'static str| -> Result<u32> {
            hash.get(field)
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(Error::MissingAttribute { capture: field })
        };
        Ok(Self {
            kind,
            number: read_int("number")?,
            annex: hash
                .get("annex")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            edition: optional_hash_str(hash, "edition").map(str::to_string),
            volume: optional_hash_str(hash, "volume").map(str::to_string),
        })
    }

    /// The canonical hash: `_type`, the INTEGER number/annex, then the
    /// handbook's edition and volume faces.
    pub fn to_hash(&self) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("_type".into(), self.kind.type_tag().into());
        object.insert("number".into(), Value::Number(self.number.into()));
        if let Some(annex) = self.annex {
            object.insert("annex".into(), Value::Number(annex.into()));
        }
        if let Some(edition) = &self.edition {
            object.insert("edition".into(), edition.clone().into());
        }
        if let Some(volume) = &self.volume {
            object.insert("volume".into(), volume.clone().into());
        }
        Value::Object(object)
    }

    /// The canonical human.
    pub fn to_human(&self) -> String {
        let mut result = match self.kind {
            Handbook => format!("PLATEAU Handbook #{}", pad2(self.number)),
            TechnicalReport => format!("PLATEAU Technical Report #{}", pad2(self.number)),
        };
        if let Some(annex) = self.annex {
            result.push('-');
            result.push_str(&annex.to_string());
        }
        if let Some(edition) = &self.edition {
            result.push_str(&format!(" 第{edition}版"));
        }
        if let Some(volume) = &self.volume {
            result.push(' ');
            result.push_str(volume);
        }
        result
    }

    /// The URN: `urn:plateau:handbook|tr:<nn>[:<aa>]`, two-digit padded.
    pub fn to_urn(&self) -> String {
        let mut parts = vec![
            "urn".to_string(),
            "plateau".to_string(),
            match self.kind {
                Handbook => "handbook".to_string(),
                TechnicalReport => "tr".to_string(),
            },
            pad2(self.number),
        ];
        if let Some(annex) = self.annex {
            parts.push(pad2(annex));
        }
        parts.join(":")
    }
}

#[cfg(test)]
mod tests {
    use super::PlateauIdentifier;
    use serde_json::json;

    #[test]
    fn the_handbook_editions_render_and_urn() {
        let id = PlateauIdentifier::parse("PLATEAU Handbook #00 第1.0版").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:plateau:handbook",
                "number": 0,
                "edition": "1.0",
            })
        );
        assert_eq!(id.to_human(), "PLATEAU Handbook #00 第1.0版");
        assert_eq!(id.to_urn(), "urn:plateau:handbook:00");
    }

    #[test]
    fn the_volume_label_extracts_and_normalizes() {
        let id =
            PlateauIdentifier::parse("PLATEAU Handbook #00 第5.0版 本編＋付録").expect("parse");
        assert_eq!(id.volume.as_deref(), Some("本編＋付録"));
        assert_eq!(id.to_human(), "PLATEAU Handbook #00 第5.0版 本編＋付録");

        // The full-width space alias carries the same hash.
        let alias = PlateauIdentifier::parse("PLATEAU Handbook #00 第5.0版\u{3000}本編＋付録")
            .expect("parse");
        assert_eq!(alias.to_hash(), id.to_hash());
    }

    #[test]
    fn the_annex_pads_in_the_urn_only() {
        let id = PlateauIdentifier::parse("PLATEAU Handbook #03-1 第1.0版").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:plateau:handbook",
                "number": 3,
                "edition": "1.0",
                "annex": 1,
            })
        );
        assert_eq!(id.to_human(), "PLATEAU Handbook #03-1 第1.0版");
        assert_eq!(id.to_urn(), "urn:plateau:handbook:03:01");
    }

    #[test]
    fn the_technical_report_drops_the_edition() {
        let id = PlateauIdentifier::parse("PLATEAU Technical Report #46-1").expect("parse");
        assert_eq!(
            id.to_hash(),
            json!({
                "_type": "pubid:plateau:technical-report",
                "number": 46,
                "annex": 1,
            })
        );
        assert_eq!(id.to_human(), "PLATEAU Technical Report #46-1");
        assert_eq!(id.to_urn(), "urn:plateau:tr:46:01");
    }

    #[test]
    fn round_trips_through_the_hash() {
        for spelling in [
            "PLATEAU Handbook #00 第2.0版",
            "PLATEAU Handbook #05 第1.0版",
            "PLATEAU Technical Report #00",
        ] {
            let id = PlateauIdentifier::parse(spelling).expect(spelling);
            let rebuilt = PlateauIdentifier::from_hash(&id.to_hash()).expect("from_hash");
            assert_eq!(rebuilt.to_hash(), id.to_hash(), "{spelling}");
            assert_eq!(rebuilt.to_human(), id.to_human(), "{spelling}");
        }
    }
}
