use crate::api::fs_files::{Filter, FilterCol, FilterOp};

use super::str2x;

fn filter_str2json(
    filter_col: FilterCol,
    filter_str_seg: &str,
) -> anyhow::Result<serde_json::Value> {
    match filter_col {
        FilterCol::CreatedAt | FilterCol::DeletedAt => {
            let start_datetime = str2x::str2datetime(filter_str_seg)?;
            Ok(serde_json::json!(start_datetime.to_rfc3339()))
        }
        FilterCol::FileSize => {
            let start_filesize = str2x::str2bytes(filter_str_seg)?;
            Ok(serde_json::json!(start_filesize))
        }
        _ => Err(anyhow::anyhow!(
            "unsupported column given for Gt/Lt filter operations!"
        )),
    }
}

/// possible inputs: `2024-01-01 12:12:12...2024-01-01 12:12:12`, `2024-01-01 12:12:12...`, `...2024-01-01 12:12:12`
pub fn parse_filter_str(filter_col: FilterCol, filter_str: &str) -> anyhow::Result<Vec<Filter>> {
    let filter_str = filter_str.trim();

    let mut filters: Vec<Filter> = vec![];
    if filter_str.contains("...") {
        let filter_str_split = filter_str.split("...").collect::<Vec<&str>>();

        if let Some(filter_str_seg) = filter_str_split.get(0) {
            if filter_str_seg.len() > 0 {
                let val = filter_str2json(filter_col, filter_str_seg)?;
                filters.push(Filter(filter_col.clone(), FilterOp::Gt, val));
            }
        };

        if let Some(filter_str_seg) = filter_str_split.get(1) {
            if filter_str_seg.len() > 0 {
                let val = filter_str2json(filter_col, filter_str_seg)?;
                filters.push(Filter(filter_col, FilterOp::Lt, val));
            }
        };
    }

    if filters.len() == 0 {
        return Err(anyhow::anyhow!("invalid filter string provided!"));
    }

    Ok(filters)
}
