use colored::Colorize;
use regex::Regex;
use std::env::current_dir;
use std::io;
use std::path::{absolute, Component, Path, MAIN_SEPARATOR};
use std::path::{PathBuf, MAIN_SEPARATOR_STR};
use terminal_size::terminal_size;
use walkdir::WalkDir;

pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    if let Some(stripped) = p.strip_prefix("~").ok() {
        if let Some(home_dir) = dirs::home_dir() {
            return home_dir.join(stripped);
        }
    }
    p.to_path_buf()
}

pub fn canonicalize<P: AsRef<Path>>(relative_path: P) -> io::Result<PathBuf> {
    let mut abs_path = current_dir()?;
    let relative_path = relative_path.as_ref();

    if relative_path.has_root() {
        return Ok(relative_path.to_path_buf());
    }

    for component in relative_path.components() {
        match component {
            Component::ParentDir => {
                abs_path.pop();
            }
            Component::RootDir => unreachable!(),
            Component::CurDir => continue,
            Component::Normal(c) => {
                abs_path.push(c.to_str().unwrap());
            }
            Component::Prefix(prefix) => {
                abs_path.push(prefix.as_os_str().to_str().unwrap());
            }
        };
    }

    Ok(abs_path)
}

pub fn get_absolute_path<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    Ok(absolute(canonicalize(expand_tilde(path))?)?)
}

pub fn get_paths_from_pattern(patt: &str) -> anyhow::Result<Vec<PathBuf>> {
    let patt = get_absolute_path(patt)?;

    let mut paths: Vec<PathBuf> = vec![];
    let mut ref_wd = PathBuf::new();

    for path_str in patt.iter() {
        let path_str = path_str.to_str();
        let path_str = match path_str {
            None => continue,
            Some(path_str) => path_str,
        };

        if path_str.contains("*") || path_str.contains("{") || path_str.contains("}") {
            break;
        }

        ref_wd.push(path_str);
    }

    if ref_wd.to_str().unwrap_or("") == patt.to_str().unwrap_or("") {
        if ref_wd.is_file() {
            paths.push(ref_wd);
        } else if ref_wd.is_dir() {
            for entry in ref_wd.read_dir()? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    paths.push(path);
                }
            }
        }

        return Ok(paths);
    }

    let patt = match patt.to_str() {
        None => return Ok(paths),
        Some(patt) => patt,
    };
    let patt = patt
        .replace('\\', r"\\")
        .replace('(', r"\(")
        .replace(')', r"\)")
        .replace('[', r"\[")
        .replace(']', r"\]")
        .replace('-', r"\-")
        .replace('.', r"\.")
        .replace('|', r"\|")
        .replace("**", r".+")
        .replace("*", format!(r"[^\{}]*", MAIN_SEPARATOR_STR).as_str());

    let re = Regex::new(r"\{([\w\.\|]+)\}").unwrap();
    let patt = re
        .replace_all(patt.as_str(), |caps: &regex::Captures| {
            let cap = &caps[1];

            format!("({cap})")
        })
        .to_string();

    let patt = format!("^{}$", patt);

    println!("{}", format!("REGEX: {}", patt).dimmed());
    let patt_regex = Regex::new(patt.as_str())?;

    for entry in WalkDir::new(&ref_wd) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && patt_regex.is_match(path.to_str().unwrap_or("")) {
            paths.push(path.to_path_buf());
        }
    }

    println!();
    Ok(paths)
}

/// returns ref path and pretty printed paths
pub fn get_pretty_paths(paths: &Vec<PathBuf>) -> (String, String) {
    let mut ref_wd: Option<&str> = None;

    let mut max_col_size = 0;
    for path in paths {
        let path_str = match path.to_str() {
            Some(path_str) => path_str,
            None => continue,
        };

        max_col_size = max_col_size.max(path_str.len());

        ref_wd = match ref_wd {
            Some(ref_wd) => {
                let mut common_i = 0;
                for (ref_seg, path_seg) in ref_wd
                    .split(MAIN_SEPARATOR)
                    .zip(path_str.split(MAIN_SEPARATOR))
                {
                    if ref_seg != path_seg {
                        break;
                    }
                    if ref_seg == "" {
                        continue;
                    };

                    common_i += ref_seg.len() + 1;
                }

                Some(&ref_wd[..=common_i.min(ref_wd.len() - 1)])
            }
            None => Some(path_str),
        };
    }
    let ref_wd = {
        let ref_wd = ref_wd.unwrap_or("");
        let last_i = ref_wd.len()
            - ref_wd
                .chars()
                .rev()
                .enumerate()
                .find_map(|(i, c)| if c == MAIN_SEPARATOR { Some(i) } else { None })
                .unwrap_or(0);

        &ref_wd[..last_i]
    };

    let n_cols = match terminal_size() {
        Some((w, _)) => {
            ((w.0 as f32 / (max_col_size - ref_wd.len() + 2).max(2) as f32) as usize).max(1)
        }
        None => 1,
    };

    let str_iter = paths.iter().map(|path| {
        path.to_string_lossy()
            .trim_start_matches(ref_wd)
            .to_string()
    });
    let output_str = super::term::get_formatted_cols(str_iter.into_iter(), n_cols);

    (ref_wd.to_string(), output_str)
}
