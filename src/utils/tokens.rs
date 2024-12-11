use core::time;

use chrono::Duration;
use regex::Regex;

use crate::shared_types;

pub fn get_acpl(permission: shared_types::AccessTokenPermission, path_pattern: &str) -> String {
    format!("{}:{}", permission, path_pattern)
}

pub fn parse_validate_ttl(exp: &str) -> anyhow::Result<time::Duration> {
    let input_regex = Regex::new(r"^[0-9dhms]+$").unwrap();
    if !input_regex.is_match(exp) {
        return Err(anyhow::anyhow!("Invalid TTL format!"));
    }

    let mut ttl = Duration::seconds(0);

    let day_regex = Regex::new(r"([0-9]+)d").unwrap();
    if let Some(caps) = day_regex.captures(exp) {
        if caps.len() > 2 {
            return Err(anyhow::anyhow!(
                "Invalid TTL format! more than 1 day value found"
            ));
        }

        let days = match caps.get(1) {
            Some(m) => m.as_str().parse::<i64>()?,
            None => 0,
        };

        ttl += Duration::days(days);
    }

    let hour_regex = Regex::new(r"([0-9]+)h").unwrap();
    if let Some(caps) = hour_regex.captures(exp) {
        if caps.len() > 2 {
            return Err(anyhow::anyhow!(
                "Invalid TTL format! more than 1 hour value found"
            ));
        }

        let hours = match caps.get(1) {
            Some(m) => m.as_str().parse::<i64>()?,
            None => 0,
        };

        ttl += Duration::hours(hours);
    }

    let min_regex = Regex::new(r"([0-9]+)m").unwrap();
    if let Some(caps) = min_regex.captures(exp) {
        if caps.len() > 2 {
            return Err(anyhow::anyhow!(
                "Invalid TTL format! more than 1 minute value found"
            ));
        }

        let mins = match caps.get(1) {
            Some(m) => m.as_str().parse::<i64>()?,
            None => 0,
        };

        ttl += Duration::minutes(mins);
    }

    let sec_regex = Regex::new(r"([0-9]+)s").unwrap();
    if let Some(caps) = sec_regex.captures(exp) {
        if caps.len() > 2 {
            return Err(anyhow::anyhow!(
                "Invalid TTL format! more than 1 second value found"
            ));
        }

        let secs = match caps.get(1) {
            Some(m) => m.as_str().parse::<i64>()?,
            None => 0,
        };

        ttl += Duration::seconds(secs);
    };

    Ok(ttl.to_std()?)
}
