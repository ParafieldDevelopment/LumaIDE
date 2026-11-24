use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryTree {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<DirectoryTree>>,
}

pub fn build(path: &Path) -> Result<DirectoryTree> {
    let mut node = DirectoryTree {
        name: file_name(path),
        path: path.to_string_lossy().into_owned(),
        is_dir: true,
        children: Some(Vec::new()),
    };

    let Ok(entries) = std::fs::read_dir(path) else {
        return Ok(node);
    };

    let mut children = Vec::new();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        children.push(if entry_path.is_dir() {
            build(&entry_path)?
        } else {
            DirectoryTree {
                name: file_name(&entry_path),
                path: entry_path.to_string_lossy().into_owned(),
                is_dir: false,
                children: None,
            }
        });
    }
    sort(&mut children);
    node.children = Some(children);
    Ok(node)
}

pub fn sort(entries: &mut [impl Sortable]) {
    entries.sort_by(|left, right| {
        let order = right.is_dir().cmp(&left.is_dir());
        if order != Ordering::Equal {
            return order;
        }
        left.name().to_lowercase().cmp(&right.name().to_lowercase())
    });
}

pub trait Sortable {
    fn name(&self) -> &str;
    fn is_dir(&self) -> bool;
}

impl Sortable for DirectoryTree {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}
