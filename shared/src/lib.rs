use serde::{Deserialize, Serialize};
pub mod error;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClipSearchParams {
    pub min_speed: Option<f64>,
    pub min_decel: Option<f64>,
    pub label_classes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaResponse {
    pub tables: Vec<TableSchema>,   // one entry per registered table
    pub label_classes: Vec<String>, // from obstacles.label_class
}

impl SchemaResponse {
    pub fn new(tables: Vec<TableSchema>, label_classes: Vec<String>) -> Self {
        Self {
            tables,
            label_classes,
        }
    }
}
