use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub path: PathBuf,
    pub description: Option<String>,
    pub schema: Option<Schema>,
    pub semantics: Semantics,
}

#[derive(Debug, Deserialize)]
pub struct Schema {
    /// A sequence of fields that describe the schema.
    pub fields: Vec<HashMap<String, String>>,
    /// A map of key-value pairs containing additional metadata.
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Semantics {
    pub timestamp: Option<String>,
}
