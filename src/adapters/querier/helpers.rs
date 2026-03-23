use std::path::Path;

use datafusion::prelude::{ParquetReadOptions, SessionContext};

use crate::core::domain::model::ClipSearchParams;

pub async fn register_with_clip_id(
    ctx: &SessionContext,
    dir: &Path,
    file_suffix: &str, // e.g. ".egomotion.parquet"
    table_name: &str,
) -> anyhow::Result<()> {
    let mut views = vec![];

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
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
        anyhow::bail!(
            "No files found in {} with suffix {}",
            dir.display(),
            file_suffix
        );
    }

    let sql = format!(
        "CREATE VIEW {} AS {}",
        table_name,
        views.join(" UNION ALL ")
    );
    ctx.sql(&sql).await?.collect().await?;

    println!("[{}] registered {} clips", table_name, views.len());
    Ok(())
}

pub fn build_search_query(params: ClipSearchParams) -> String {
    let mut wheres: Vec<String> = vec![];

    if let Some(min_speed) = params.min_speed {
        wheres.push(format!(
            "AVG(SQRT(e.vx * e.vx + e.vy * e.vy)) > {min_speed}"
        ));
    }

    if let Some(min_decel) = params.min_decel {
        wheres.push(format!(
            "AVG(SQRT(e.ax * e.ax + e.ay * e.ay)) > {min_decel}"
        ));
    }

    let where_clause = if wheres.is_empty() {
        "1=1".to_owned()
    } else {
        wheres.join(" AND ")
    };

    format!(
        r#"
        SELECT e.clip_id
        FROM ego_motion e
        GROUP BY e.clip_id
        HAVING {where_clause}
        "#
    )
}
