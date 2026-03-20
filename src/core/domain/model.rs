pub struct Timespan {
    pub start: u64,
    pub end: u64,
}

impl Timespan {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }
}

#[derive(Debug)]
pub struct DataError {
    error_msg: String,
}

impl DataError {
    pub fn new(error_msg: String) -> Self {
        Self { error_msg }
    }
}
