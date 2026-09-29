//! RS1 (TODO 12): schema-driven value materialization over PARG artifacts.
//!
//! The artifact is the contract: schemas, corpora and gates all run
//! against checksum-verified envelopes through `parsanol::parg`. v1 is a
//! schema-driven serde `Value` model; typed structs can replace it
//! behind the same materialize boundary later.

use serde_json::Value;
use std::error::Error;

/// Errors raised while loading or evaluating PARG artifacts.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Load and checksum-verify an artifact from disk.
pub fn load_artifact(path: impl AsRef<std::path::Path>) -> Result<parsanol::PargArtifact> {
    Ok(parsanol::PargArtifact::from_path(path)?)
}

/// Schema-driven materialization (mirrors pubid-ts `materializeFromSchema`):
/// the schema's fields define the attribute set; `*`/`1*` cards become
/// arrays, `0..1` becomes nullable.
pub fn materialize_from_schema(schema: &Value, bound: &Value) -> Result<(String, Value)> {
    let entry = schema["root"]
        .as_str()
        .ok_or("schema entry has no root")?
        .to_string();
    let mut attributes = serde_json::Map::new();
    for (path, requirements) in schema["fields"]
        .as_object()
        .ok_or("schema entry has no fields")?
    {
        let leaf = path.rsplit("].").next().unwrap_or(path);
        let card = requirements
            .get("card")
            .and_then(Value::as_str)
            .unwrap_or("1");
        let value = bound.get(leaf);
        let materialized = if card.ends_with('*') {
            Value::Array(match value {
                Some(Value::Array(items)) => items.clone(),
                Some(v) => vec![v.clone()],
                None => Vec::new(),
            })
        } else if card == "0..1" {
            value.cloned().unwrap_or(Value::Null)
        } else if let Some(v) = value {
            v.clone()
        } else {
            continue; // absent scalar: not part of the attribute set
        };
        attributes.insert(leaf.to_string(), materialized);
    }
    Ok((entry, Value::Object(attributes)))
}

/// A loaded artifact plus its schema, ready to parse and materialize.
pub struct Pubid {
    artifact: parsanol::PargArtifact,
    entry: String,
}

impl Pubid {
    /// Load an artifact; the entry defaults to the compiler-baked
    /// `default_entry`.
    pub fn load(path: impl AsRef<std::path::Path>, entry: Option<&str>) -> Result<Self> {
        let text = std::fs::read_to_string(path.as_ref())?;
        let envelope: Value = serde_json::from_str(&text)?;
        let entry = entry
            .map(str::to_string)
            .or_else(|| {
                envelope
                    .get("default_entry")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .ok_or("artifact has no default_entry; pass an entry")?;
        Ok(Self {
            artifact: load_artifact(path)?,
            entry,
        })
    }

    /// Parse and bind; `None` when the input does not parse.
    pub fn parse_and_bind(&self, input: &str) -> Option<Value> {
        self.artifact.parse_and_bind(&self.entry, input).ok()
    }

    /// Parse, bind, and materialize through the schema.
    pub fn materialize(&self, input: &str) -> Option<(String, Value)> {
        let bound = self.parse_and_bind(input)?;
        let schema = parsanol::parg::schema::from_artifact(&self.artifact).ok()?;
        materialize_from_schema(schema.get(&self.entry)?, &bound).ok()
    }

    /// Embedded-suite runner (RS2); empty means green.
    pub fn run_tests(&self) -> Vec<String> {
        self.artifact.run_tests()
    }
}
