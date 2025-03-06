use colored::Colorize;
use indicatif::ProgressStyle;

pub fn get_sized_throughput_progress_style(label: Option<&str>) -> ProgressStyle {
    ProgressStyle::with_template(&format!(
        "{}{{binary_bytes}} ({{bytes_per_sec}}) {{wide_bar}} {{binary_total_bytes}} [{{eta}} remaining]",
        match label {
            Some(msg) => format!("{} ", msg.bold().cyan()),
            None => "".to_string()
        }
    ))
    .unwrap()
}
