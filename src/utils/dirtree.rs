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
