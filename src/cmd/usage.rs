use std::u8;

use clap::Parser;
use colored::{Color, Colorize};

use crate::{api, shared_types::CliSubCmd, utils};

#[derive(Debug)]
struct RGB(u8, u8, u8);

impl Into<Color> for RGB {
    fn into(self) -> Color {
        return Color::TrueColor {
            r: self.0,
            g: self.1,
            b: self.2,
        };
    }
}

/// fraction should be in range [0, 1]
fn get_target_color(start: &RGB, end: &RGB, fraction: f32) -> RGB {
    RGB(
        (start.0 as f32 + (end.0 as f32 - start.0 as f32) * fraction).min(u8::MAX as f32) as u8,
        (start.1 as f32 + (end.1 as f32 - start.1 as f32) * fraction).min(u8::MAX as f32) as u8,
        (start.2 as f32 + (end.2 as f32 - start.2 as f32) * fraction).min(u8::MAX as f32) as u8,
    )
}

#[derive(Parser)]
pub struct UsageCommand;

impl CliSubCmd for UsageCommand {
    async fn run(&self) {
        let usage = api::usage::get_api_usage()
            .await
            .expect("error occured while fetching api key usage!");

        let start = RGB(20, 234, 37);
        let end = RGB(234, 37, 20);

        let reads_color = get_target_color(
            &start,
            &end,
            usage.reads_used as f32 / usage.reads_limit as f32,
        );
        let writes_color = get_target_color(
            &start,
            &end,
            usage.writes_used as f32 / usage.writes_limit as f32,
        );
        let storage_gbh_color = get_target_color(
            &start,
            &end,
            usage.storage_gb_hour_used as f32 / usage.storage_gb_hour_limit as f32,
        );

        println!(
            "using: {}",
            format!(
                "{}",
                utils::x2str::bytes2str((usage.storage_gb_used * 1024_i64.pow(3) as f32) as u64)
            )
            .bold()
        );
        println!();

        println!(
            "{}",
            format!(
                "{} {} / {} {}",
                "-".dimmed(),
                usage.reads_used.to_string().bold().color(reads_color),
                usage.reads_limit.to_string().bold(),
                "reads".dimmed()
            )
        );
        println!(
            "{}",
            format!(
                "{} {} / {} {}",
                "-".dimmed(),
                usage.writes_used.to_string().bold().color(writes_color),
                usage.writes_limit.to_string().bold(),
                "writes".dimmed()
            )
        );
        println!(
            "{}",
            format!(
                "{} {} / {} {}",
                "-".dimmed(),
                usage
                    .storage_gb_hour_used
                    .to_string()
                    .bold()
                    .color(storage_gbh_color),
                usage.storage_gb_hour_limit.to_string().bold(),
                "GB-hr".dimmed()
            )
        );
    }
}
