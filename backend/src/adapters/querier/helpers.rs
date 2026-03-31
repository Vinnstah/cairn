use std::path::Path;

use datafusion::prelude::{ParquetReadOptions, SessionContext};
use log::info;
use shared::{ClipSearchParams, error::CairnError};

use crate::error::ServerError;

pub async fn register_with_clip_id(
    ctx: &SessionContext,
    dir: &Path,
    file_suffix: &str, // e.g. ".egomotion.parquet"
    table_name: &str,
) -> Result<(), ServerError> {
    let mut views = vec![];

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|err| CairnError::Generic {
            reason: err.to_string(),
        })?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.ends_with(file_suffix))
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let filename = entry.file_name();
        let filename = filename.to_str().unwrap();
        let clip_id = filename.strip_suffix(file_suffix).unwrap();

        let alias = format!("{}_{}", table_name, i);
        ctx.register_parquet(
            &alias,
            path.to_str().unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

        views.push(format!("SELECT '{}' AS clip_id, * FROM {}", clip_id, alias));
    }

    if views.is_empty() {
        return Err(CairnError::Generic {
            reason: format!(
                "No files found in {} with suffix {}",
                dir.display(),
                file_suffix
            ),
        }
        .into());
    }

    let sql = format!(
        "CREATE VIEW {} AS {}",
        table_name,
        views.join(" UNION ALL ")
    );
    ctx.sql(&sql).await?.collect().await?;

    info!("[{}] registered {} clips", table_name, views.len());
    Ok(())
}

pub fn build_search_query(params: ClipSearchParams) -> String {
    info!("build search query");
    let mut having = vec![];

    if let Some(v) = params.min_speed {
        having.push(format!("AVG(SQRT(e.vx * e.vx + e.vy * e.vy)) > {v}"));
    }
    if let Some(v) = params.min_decel {
        having.push(format!("AVG(SQRT(e.ax * e.ax + e.ay * e.ay)) > {v}"));
    }

    let having_clause = if having.is_empty() {
        "1=1".into()
    } else {
        having.join(" AND ")
    };

    if params.label_classes.is_empty() {
        format!(
            "SELECT e.clip_id
         FROM ego_motion e
         WHERE e.clip_id IN (SELECT DISTINCT clip_id FROM lidar)
         GROUP BY e.clip_id
         HAVING {having_clause}
         LIMIT 10"
        )
    } else {
        let class_list = params
            .label_classes
            .iter()
            .map(|c| format!("'{c}'"))
            .collect::<Vec<_>>()
            .join(", ");

        let class_count = params.label_classes.len();

        format!(
            "SELECT e.clip_id
         FROM ego_motion e
         WHERE e.clip_id IN (SELECT DISTINCT clip_id FROM lidar)
           AND e.clip_id IN (SELECT DISTINCT clip_id FROM obstacles
                             WHERE label_class IN ({class_list})
                             GROUP BY clip_id
                             HAVING COUNT(DISTINCT label_class) = {class_count})
         GROUP BY e.clip_id
         HAVING {having_clause}
         LIMIT 10"
        )
    }
}
