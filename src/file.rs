use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub enum EntryKind {
    File(PathBuf),
    Directory(PathBuf),
    #[allow(dead_code)]
    Other(PathBuf), // symlink, device, etc.
}

pub fn list_dir<P: AsRef<Path>>(path: P, cull_empty_folders: bool) -> Result<Vec<EntryKind>> {
    let entries = fs::read_dir(path.as_ref()).with_context(|| {
        format!(
            "entries failed: Failed to read directory: {}",
            path.as_ref().display()
        )
    })?;

    let mut results = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        let kind = if file_type.is_file() {
            EntryKind::File(path)
        } else if file_type.is_dir() {
            if cull_empty_folders {
                let mut dir_iter = fs::read_dir(&path).with_context(|| {
                    format!("kind failed: Failed to read directory: {}", path.display())
                })?;

                if dir_iter.next().is_none() {
                    continue;
                }
            }
            EntryKind::Directory(path)
        } else {
            EntryKind::Other(path)
        };

        results.push(kind);
    }

    Ok(results)
}

pub fn get_top_level_dir<'a>(
    file_path: &'a Path,
    base_dir: &'a Path,
) -> anyhow::Result<Option<&'a std::ffi::OsStr>> {
    // Strip base_dir prefix from file_path, get the relative path
    let relative = file_path.strip_prefix(base_dir).with_context(|| {
        format!(
            "Base directory '{}' is not a prefix of file path '{}'",
            base_dir.display(),
            file_path.display()
        )
    })?;
    Ok(relative.components().next().map(|comp| comp.as_os_str()))
}

pub fn relative_after(path: &Path, base: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).ok().map(|p| p.to_path_buf())
}

pub fn relative_before(path: &Path, base: &Path) -> Option<PathBuf> {
    let stripped = path.strip_prefix(base).ok()?;

    let mut components = stripped.components();
    let first_component = components.next()?;

    let mut new_path = PathBuf::from(base);
    new_path.push(first_component.as_os_str());

    Some(new_path)
}

pub fn clear_folder_contents(folder: &Path) -> std::io::Result<()> {
    // If is a directory
    if folder.is_dir() {
        for entry_result in fs::read_dir(folder)? {
            let entry = entry_result?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?; // recursively remove subfolder
            } else {
                fs::remove_file(&path)?; // remove file
            }
        }
    }
    Ok(())
}
