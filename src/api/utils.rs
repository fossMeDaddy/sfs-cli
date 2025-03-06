use rand::distributions::{Alphanumeric, DistString};

use crate::constants;

pub fn get_random_filename() -> String {
    format!(
        "{}.{}",
        Alphanumeric.sample_string(&mut rand::thread_rng(), constants::RANDOM_FILENAME_LEN),
        constants::UNKNOWN_FILE_EXT
    )
}
