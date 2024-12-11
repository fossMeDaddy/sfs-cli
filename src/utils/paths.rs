use regex::Regex;
use std::env::current_dir;
use std::io;
use std::path::{absolute, Component, Path};
use std::path::{PathBuf, MAIN_SEPARATOR_STR};
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

    let re = Regex::new(r"\{([^}]*)\}").unwrap();
    let patt = re
        .replace_all(patt.as_str(), |caps: &regex::Captures| {
            let cap = &caps[1];

            format!("({})", cap.replace(",", "|"))
        })
        .to_string();

    let patt = format!("^{}$", patt);

    println!("REGEX: {}", patt);
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
