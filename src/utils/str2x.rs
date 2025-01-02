use anyhow::anyhow;
use base64::prelude::*;
use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone};
use regex::Regex;

use crate::{constants, shared_types::AccessToken};

pub fn str2datetime(datetime_str: &str) -> anyhow::Result<DateTime<Local>> {
    match NaiveDateTime::parse_from_str(datetime_str, constants::LOCAL_DATETIME_FORMAT) {
        Ok(datetime) => match Local.from_local_datetime(&datetime).single() {
            Some(datetime) => Ok(datetime),
            None => return Err(anyhow::anyhow!("error occured while parsing datetime!")),
        },
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}

pub fn str2bytes(b_str: &str) -> anyhow::Result<f32> {
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

pub fn str2duration(exp: &str) -> anyhow::Result<Duration> {
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

    Ok(ttl)
}

pub fn str2at(token_str: &str) -> anyhow::Result<AccessToken> {
    let token_str = super::url::decode_access_token(token_str);

    let segs = token_str.splitn(2, "_").collect::<Vec<&str>>();

    let mut contents: Vec<String> = vec![];
    for content in BASE64_STANDARD_NO_PAD
        .decode(
            segs.first()
                .ok_or(anyhow!("Invalid segments in access token."))?,
        )?
        .splitn(3, |c| *c == '\n' as u8)
        .take(2)
        .map(|s| String::from_utf8(s.to_vec()))
    {
        match content {
            Ok(content) => contents.push(content),
            Err(err) => return Err(anyhow!(err)),
        };
    }

    if contents.len() != 2 {
        return Err(anyhow!(
            "Invalid number of segments in access token. Access token seems corrupted."
        ));
    }

    let expires_at: DateTime<Local> = DateTime::parse_from_rfc3339(
        contents
            .get(0)
            .ok_or(anyhow!("Invalid content segments in access token."))?,
    )?
    .into();

    let acpl: Vec<String> = serde_json::from_str(
        contents
            .get(1)
            .ok_or(anyhow!("Invalid content segments in access token."))?,
    )?;

    Ok(AccessToken {
        api_key: None,
        acpl,
        expires_at,
    })
}
