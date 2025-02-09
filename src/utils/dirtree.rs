use std::collections::HashMap;

pub struct PrintDirTreeOpts<'a> {
    pub file_counts: Option<&'a HashMap<String, u32>>,
    pub indent: usize,
    pub level: i16,
    pub print_note: bool,
    pub cwd_dir_path: &'a str,
}

impl PrintDirTreeOpts<'_> {
    pub fn get_default_opts() -> Self {
        Self {
            indent: 2,
            file_counts: None,
            level: i16::MAX,
            print_note: false,
            cwd_dir_path: "",
        }
    }
}

pub fn get_absolute_path(path: &str, wd: &str) -> String {
    if path.starts_with('/') {
        return path.to_string();
    }
    let path = path.trim_matches('/');

    let mut abs_path = wd
        .split('/')
        .filter_map(|seg| if seg == "" { None } else { Some(seg) })
        .collect::<Vec<&str>>();

    for segment in path.split('/') {
        if segment == ".." {
            abs_path.pop();
        } else if segment == "." {
            continue;
        } else {
            abs_path.push(segment);
        }
    }

    return String::from("/") + &abs_path.join("/");
}

/// split path into (dirpath, filename)
pub fn split_path(path: &str) -> (&str, &str) {
    let split_i = path
        .chars()
        .rev()
        .enumerate()
        .find_map(|(i, c)| if c == '/' { Some(i) } else { None });

    match split_i {
        Some(i) => path.split_at(path.len() - i),
        None => (path, ""),
    }
}

pub fn join_paths(paths: &[&str]) -> String {
    let mut segs_iter = paths.iter();

    let mut path_str = String::from(segs_iter.next().unwrap_or(&"/").trim_end_matches("/"));
    for seg in segs_iter {
        let seg_str = seg.trim().trim_matches('/');
        if seg_str.len() > 0 {
            path_str += "/";
            path_str += seg_str;
        }
    }

    path_str
}
