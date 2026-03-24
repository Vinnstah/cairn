#[derive(Debug)]
pub struct DataError {
    pub error_msg: String,
}

impl DataError {
    pub fn new(error_msg: String) -> Self {
        Self { error_msg }
    }
}

#[derive(serde::Deserialize)]
pub struct ClipSearchParams {
    pub min_decel: Option<f64>,
    pub min_speed: Option<f64>,
}
