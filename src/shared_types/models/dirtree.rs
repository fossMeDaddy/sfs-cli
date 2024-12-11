use colored::*;
use serde::{Deserialize, Serialize};
use std::iter;

use crate::utils::dirtree::PrintDirTreeOpts;

#[derive(Deserialize, Serialize, Debug)]
pub struct DirTree {
    pub id: String,
    pub name: String,
    pub children: Vec<DirTree>,
}

impl DirTree {
    fn _print_dir_tree(&self, opts: &PrintDirTreeOpts, full_path: String) -> String {
        if opts.level < 0 {
            return "".to_string();
        }

        let mut res = format!(
            "{}— {}/ {}\n",
            iter::repeat(" ").take(opts.indent).collect::<String>(),
            if full_path == opts.cwd_dir_path {
                self.name.bold().cyan().to_string()
            } else {
                self.name.clone()
            },
            if let Some(file_counts) = opts.file_counts {
                let count = file_counts.get(&self.id).unwrap_or(&(0 as u32));
                let file_count_str = format!("({})", count).bright_black();

                file_count_str.to_string()
            } else {
                String::new()
            },
        );
        for child in &self.children {
            res.push_str(&Self::_print_dir_tree(
                &child,
                &PrintDirTreeOpts {
                    file_counts: opts.file_counts,
                    level: opts.level - 1,
                    indent: opts.indent + 2,
                    print_note: opts.print_note,
                    cwd_dir_path: opts.cwd_dir_path,
                },
                format!("{}/{}", full_path, child.name),
            ));
        }

        res
    }

    pub fn print_dir_tree(&self, opts: &PrintDirTreeOpts) -> String {
        let mut dirtree_str = self._print_dir_tree(opts, String::new());
        if opts.print_note {
            let note = String::from("\nNOTE: file counts displayed as '(..)' do not represent cumulative file counts in subsequent children directories.");
            dirtree_str.push_str(note.bright_black().to_string().as_str());
        }

        return dirtree_str;
    }

    pub fn get_sub_tree(&self, dirpath: &str) -> Option<&Self> {
        let dirpath = dirpath.trim_matches('/');
        if dirpath == self.name {
            return Some(self);
        }

        let mut currentdir = self;
        for path_segment in dirpath.split("/") {
            currentdir = match currentdir
                .children
                .iter()
                .find(|item| item.name == path_segment)
            {
                Some(dir) => dir,
                None => return None,
            }
        }

        Some(currentdir)
    }

    pub fn split_path<'a>(&self, dirpath: &'a str) -> Option<(&Self, Option<&'a str>)> {
        if let Some(subtree) = self.get_sub_tree(dirpath) {
            Some((subtree, None))
        } else {
            let mut dirpath = dirpath.split('/').collect::<Vec<&str>>();
            let child_path = dirpath.pop();

            if let Some(dir_subtree) = self.get_sub_tree(&dirpath.join("/")) {
                Some((dir_subtree, child_path))
            } else {
                None
            }
        }
    }
}
