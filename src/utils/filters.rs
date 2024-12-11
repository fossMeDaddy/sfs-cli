use chrono::{DateTime, Local, NaiveDateTime, TimeZone};

use crate::api::fs_files::{Filter, FilterCol, FilterOp};

fn str2datetime(datetime_str: &str) -> anyhow::Result<DateTime<Local>> {
    match NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        Ok(datetime) => match Local.from_local_datetime(&datetime).single() {
            Some(datetime) => Ok(datetime),
            None => return Err(anyhow::anyhow!("error occured while parsing datetime!")),
        },
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}

fn str2bytes(b_str: &str) -> anyhow::Result<f32> {
    let mut n: f32 =
        String::from_iter(
            b_str
                .chars()
                .filter_map(|c| match c.is_digit(10) || c == '.' {
                    true => Some(c),
                    false => None,
                }),
        )
        .parse()?;

    if b_str.ends_with("kb") {
        n = n * 10_f32.powi(3).powi(1);
    } else if b_str.ends_with("mb") {
        n = n * 10_f32.powi(3).powi(2);
    } else if b_str.ends_with("gb") {
        n = n * 10_f32.powi(3).powi(3);
    } else if b_str.ends_with("tb") {
        n = n * 10_f32.powi(3).powi(4);
    }

    Ok(n)
}

fn str2json(filter_col: FilterCol, filter_str_seg: &str) -> anyhow::Result<serde_json::Value> {
    match filter_col {
        FilterCol::CreatedAt | FilterCol::DeletedAt => {
            let start_datetime = str2datetime(filter_str_seg)?;
            Ok(serde_json::json!(start_datetime.to_rfc3339()))
        }
        FilterCol::FileSize => {
            let start_filesize = str2bytes(filter_str_seg)?;
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
                let val = str2json(filter_col, filter_str_seg)?;
                filters.push(Filter(filter_col.clone(), FilterOp::Gt, val));
            }
        };

        if let Some(filter_str_seg) = filter_str_split.get(1) {
            if filter_str_seg.len() > 0 {
                let val = str2json(filter_col, filter_str_seg)?;
                filters.push(Filter(filter_col, FilterOp::Lt, val));
            }
        };
    }

    if filters.len() == 0 {
        return Err(anyhow::anyhow!("invalid filter string provided!"));
    }

    Ok(filters)
}
