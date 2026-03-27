use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name:      String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClipSearchParams {
    pub min_speed: Option<f64>,
    pub min_decel: Option<f64>,
}
