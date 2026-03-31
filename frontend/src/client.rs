use shared::{ClipSearchParams, SchemaResponse};

const BASE_URL: &str = "http://localhost:3000";

#[derive(Debug)]
pub enum ClientError {
    Http(String),
    Parse(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Http(e) => write!(f, "HTTP error: {}", e),
            ClientError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

pub fn fetch_schema() -> Result<SchemaResponse, ClientError> {
    reqwest::blocking::get(format!("{}/schema", BASE_URL))
        .map_err(|e| ClientError::Http(e.to_string()))?
        .json::<SchemaResponse>()
        .map_err(|e| ClientError::Parse(e.to_string()))
}

pub fn trigger_replay(params: &ClipSearchParams) -> Result<(), ClientError> {
    let mut query = vec![];
    if let Some(v) = params.min_speed {
        query.push(format!("min_speed={}", v));
    }
    if let Some(v) = params.min_decel {
        query.push(format!("min_decel={}", v));
    }

    let url = if query.is_empty() {
        format!("{}/clips/replay", BASE_URL)
    } else {
        format!("{}/clips/replay?{}", BASE_URL, query.join("&"))
    };
    let client = reqwest::blocking::Client::new();
    client
        .post(&url)
        .json(params)
        .send()
        .map_err(|e| ClientError::Http(e.to_string()))?;

    Ok(())
}
