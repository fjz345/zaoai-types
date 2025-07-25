use crate::{
    chapters::{Chapters, extract_chapters},
    file::{self, EntryKind, clear_folder_contents, list_dir},
    temp::copy_to_temp,
};
use anyhow::{Context, Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use std::io::Write;

pub(crate) fn get_third_party_binary(name: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR will be the zaohelper/ path even when used from zaoai
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    base.join("third_party\\bin").join(name)
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ListDirSplit {
    pub with_chapters: Vec<EntryKind>,
    pub without_chapters: Vec<EntryKind>,
    pub skipped: Vec<EntryKind>,
    pub path_source: PathBuf,
}

impl ListDirSplit {
    pub fn from_file_json(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)?;
        let new = serde_json::from_str::<Self>(&json_str)?;
        Ok(new)
    }
}

pub fn list_dir_with_kind_has_chapters_split(
    list: &[EntryKind],
    cull_empty_folders: bool,
) -> Result<ListDirSplit> {
    let mut list_dir_split = ListDirSplit::default();

    for item in list {
        match item {
            EntryKind::File(path_buf) => {
                let mut has_chapters = false;
                if let Some(ext) = path_buf.extension() {
                    if ext == "mkv" {
                        // Copy only this file to temp
                        let (_temp_dir, temp_file_path) = copy_to_temp(path_buf)?;
                        let mkv_file_str = temp_file_path
                            .to_str()
                            .ok_or_else(|| anyhow::anyhow!("Invalid temp path string"))?;

                        // Read chapters from local copy
                        match extract_chapters(&mkv_file_str) {
                            Ok(chapters) => has_chapters = !chapters.iter().next().is_none(),
                            Err(e) => println!("{e}"),
                        }
                    }
                }

                if has_chapters {
                    list_dir_split.with_chapters.push(item.clone());
                } else {
                    list_dir_split.without_chapters.push(item.clone());
                }
            }
            EntryKind::Directory(path_buf) => {
                let path_buf_entry_list = list_dir(path_buf, cull_empty_folders)
                    .with_context(|| format!("Failed to copy file: {}", path_buf.display()))?;
                let dir_res =
                    list_dir_with_kind_has_chapters_split(&path_buf_entry_list, cull_empty_folders);
                match dir_res {
                    Ok(dir_split) => {
                        let ListDirSplit {
                            with_chapters: directory_with_chapters,
                            without_chapters: directory_without_chapters,
                            skipped,
                            path_source,
                        } = dir_split;
                        list_dir_split.with_chapters.extend(directory_with_chapters);
                        list_dir_split
                            .without_chapters
                            .extend(directory_without_chapters);
                    }
                    Err(e) => {
                        println!("{e}")
                    }
                }
            }
            EntryKind::Other(_) => {
                // Skip or handle Other if needed
            }
        }
    }

    Ok(list_dir_split)
}

fn all_files_have_chapters(dir_path: &Path, cull_empty_folders: bool) -> Result<bool> {
    let entries = list_dir(dir_path, cull_empty_folders)?;

    for entry in entries {
        match entry {
            EntryKind::File(ref path) => {
                // ignore .txt files
                if path.extension().unwrap_or_default() == "txt" {
                    continue;
                }
                if !has_chapters(path)? {
                    return Ok(false);
                }
            }
            EntryKind::Directory(ref path) => {
                if !all_files_have_chapters(path, cull_empty_folders)? {
                    return Ok(false);
                }
            }
            EntryKind::Other(_) => {
                // skip or treat as no chapters (your choice)
            }
        }
    }

    Ok(true)
}

fn has_chapters(path: &Path) -> Result<bool> {
    if let Some(ext) = path.extension() {
        if ext == "mkv" {
            // Copy only this file to temp
            let (_temp_dir, temp_file_path) = copy_to_temp(path)?;
            let mkv_file_str = temp_file_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid temp path string"))?;

            // Read chapters from local copy

            let chapters = extract_chapters(&mkv_file_str)?;
            return Ok(!chapters.iter().next().is_none());
        }
    }
    Ok(false)
}
