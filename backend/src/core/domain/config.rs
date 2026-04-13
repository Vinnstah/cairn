use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dataset {
    pub name: String,
    pub path: PathBuf,
    pub file_ext: String,
    pub description: Option<String>,
    pub schema: Option<SchemaDefinition>,
    pub characteristics: Characteristics,
}

/// Domain representation of a column field — mirrors Arrow's Field
/// without importing datafusion::arrow::datatypes::Field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub data_type: String, // Arrow type as string e.g. "Float64", "Int64", "Utf8"
    pub nullable: bool,
    pub metadata: HashMap<String, String>,
}

/// Domain representation of a schema.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SchemaDefinition {
    pub fields: Vec<FieldDefinition>,
    pub metadata: HashMap<String, String>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct Characteristics {
    pub contains_classes: Option<bool>,
    pub semantics: Semantics,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Semantics {
    pub timestamp: Option<String>,
    pub clip_id: Option<String>,
    pub label_class: Option<String>,
}
