use std::env;

use datafusion::{
    arrow::array::RecordBatch,
    error::DataFusionError,
    prelude::{ParquetReadOptions, SessionContext},
};

pub struct Timespan {
    start: u64,
    end: u64,
}

impl Timespan {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }
}

pub trait Querier {
    async fn query_selected_time(
        &self,
        timespan: Timespan,
    ) -> Result<Vec<RecordBatch>, DataFusionError>;
}

impl Querier for SessionContext {
    async fn query_selected_time(
        &self,
        timespan: Timespan,
    ) -> Result<Vec<RecordBatch>, DataFusionError> {
        let home = env::var("HOME").expect("HOME not set");
        let base = format!("{}/Developer/Rust/cairn/data/synthetic_data", home);

        self.register_parquet(
            "ego_motion",
            &format!("{}/ego_motion/", base),
            ParquetReadOptions::default(),
        )
        .await?;
        let _ = self
            .register_parquet(
                "metadata",
                "~/Developer/Rust/cairn/data/synthetic_data/metadata/",
                ParquetReadOptions::default(),
            )
            .await;
        let df = self
            .sql(format!("SELECT * FROM ego_motion WHERE timestamp >= {} and timestamp <= {} ORDER BY timestamp", timespan.start, timespan.end).as_str())
            .await?;
        df.clone().collect().await
    }
}
