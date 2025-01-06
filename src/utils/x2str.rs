use chrono::Duration;

const DIVS: [(u32, &str); 4] = [(24 * 60 * 60, "d"), (60 * 60, "h"), (60, "m"), (1, "s")];

pub fn duration2str(duration: Duration) -> String {
    let mut duration_str = String::new();

    let mut secs = duration.num_seconds().abs() as u64;
    if secs == 0 {
        return String::from("0s");
    }
    for (div, suffix) in DIVS {
        let n = (secs as f32 / div as f32).floor() as u64;
        if n > 0 {
            duration_str += &format!("{}{}", n, suffix);
        }
        secs -= n * div as u64;
    }

    duration_str
}
