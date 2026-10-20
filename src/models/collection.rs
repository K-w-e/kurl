use super::request::Request;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionItem {
    pub path: PathBuf,
    pub name: String,
    pub group: Option<String>,
    pub request: Request,
}

impl CollectionItem {
    pub fn from_file<P: AsRef<Path>>(path: P, group: Option<String>) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let request = Request::from_file(&path_buf)?;
        let name = if !request.name.is_empty() {
            request.name.clone()
        } else {
            path_buf
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Request".to_string())
        };

        Ok(Self {
            path: path_buf,
            name,
            group,
            request,
        })
    }

    pub fn scan_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<Self>> {
        let mut items = Vec::new();
        let base_path = dir.as_ref();
        if !base_path.exists() {
            return Ok(items);
        }

        Self::scan_recursive(base_path, base_path, &mut items)?;

        items.sort_by(|a, b| match (&a.group, &b.group) {
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(ga), Some(gb)) => ga.cmp(gb).then_with(|| a.name.cmp(&b.name)),
            (None, None) => a.name.cmp(&b.name),
        });

        Ok(items)
    }

    fn scan_recursive(base_dir: &Path, current_dir: &Path, items: &mut Vec<Self>) -> Result<()> {
        let entries = fs::read_dir(current_dir)
            .with_context(|| format!("Failed to read directory: {:?}", current_dir))?;

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let filename = entry_path.file_name().unwrap_or_default().to_string_lossy();
                if filename.ends_with(".posting.yaml")
                    || filename.ends_with(".posting.yml")
                    || filename.ends_with(".yaml")
                {
                    let group = if current_dir != base_dir {
                        current_dir
                            .strip_prefix(base_dir)
                            .ok()
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                    } else {
                        None
                    };

                    if let Ok(item) = Self::from_file(&entry_path, group) {
                        items.push(item);
                    }
                }
            } else if entry_path.is_dir() {
                let _ = Self::scan_recursive(base_dir, &entry_path, items);
            }
        }
        Ok(())
    }

    pub fn create_group<P: AsRef<Path>>(base_dir: P, group_name: &str) -> Result<PathBuf> {
        let sanitized = group_name.trim().replace(['\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let group_path = base_dir.as_ref().join(sanitized);
        fs::create_dir_all(&group_path)
            .with_context(|| format!("Failed to create group directory: {:?}", group_path))?;
        Ok(group_path)
    }

    pub fn list_groups<P: AsRef<Path>>(base_dir: P) -> Result<Vec<String>> {
        let mut groups = Vec::new();
        let path = base_dir.as_ref();
        if !path.exists() {
            return Ok(groups);
        }
        let entries = fs::read_dir(path)?;
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                groups.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        groups.sort();
        Ok(groups)
    }
}
