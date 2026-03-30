use serde::{Deserialize, Serialize};
pub mod error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClipSearchParams {
    pub min_speed: Option<f64>,
    pub min_decel: Option<f64>,
    pub label_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaResponse {
    pub column_info: Vec<ColumnInfo>,
    pub label_classes: Vec<String>,
}

impl SchemaResponse {
    pub fn new(column_info: Vec<ColumnInfo>, label_classes: Vec<String>) -> Self {
        Self {
            column_info,
            label_classes,
        }
    }
}
