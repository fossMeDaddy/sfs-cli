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

pub fn bytes2str(filesize: u64) -> String {
    let mut i = 0;
    let mut size = filesize as f32;
    while i < 4 {
        let s = size / 10_f32.powi(3);
        if s < 1.0 {
            break;
        }

        size = s;
        i += 1;
    }

    match i {
        0 => format!("{:.3}b", size),
        1 => format!("{:.3}kb", size),
        2 => format!("{:.3}mb", size),
        3 => format!("{:.3}gb", size),
        _ => format!("{:.3}tb", size),
    }
}
