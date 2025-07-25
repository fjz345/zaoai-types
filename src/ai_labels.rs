use anyhow::Result;
use std::io::Write;

use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::mkv::process_mkv_file;
use crate::{
    chapters::{Chapters, VideoMetadata},
    file::get_top_level_dir,
    utils::ListDirSplit,
};

pub const ZAOAI_LABEL_VERSION: u8 = 1;
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ZaoaiLabel {
    pub path: PathBuf,
    pub metadata: VideoMetadata,
    pub version: u8,

    #[serde(with = "humantime_serde")]
    pub opening_start_time: Option<Duration>,
    #[serde(with = "humantime_serde")]
    pub opening_end_time: Option<Duration>,
    pub opening_start_frame: Option<u32>,
    pub opening_end_frame: Option<u32>,
    pub opening_start_normalized: Option<f64>,
    pub opening_end_normalized: Option<f64>,
}

impl ZaoaiLabel {
    pub fn has_opening(&self) -> bool {
        self.opening_start_frame.is_some() && self.opening_end_frame.is_some()
    }
}

pub fn collect_zaoai_labels(
    list_dir_split: &ListDirSplit,
    out_path: impl AsRef<Path>,
) -> Result<()> {
    for entry_with_chapters in &list_dir_split.with_chapters {
        let path_buf = entry_with_chapters.as_ref();
        let zaoai_label = if path_buf.is_file() {
            if path_buf.is_file() {
                let b = process_mkv_file(&entry_with_chapters);
                match b {
                    Ok(mkv_metadata) => {
                        let (op_start, op_end) = mkv_metadata.extract_opening_times();

                        if op_start.is_some() && op_end.is_some() {
                            let opening_start_time = op_start.unwrap();
                            let opening_end_time = op_end.unwrap();

                            let video_metadata: VideoMetadata = mkv_metadata.into();
                            let total_secs = video_metadata.duration.as_secs_f64();

                            let ai_label = ZaoaiLabel {
                                path: path_buf.clone(),
                                metadata: video_metadata,
                                version: ZAOAI_LABEL_VERSION,
                                opening_start_time: Some(opening_start_time),
                                opening_end_time: Some(opening_end_time),
                                opening_start_frame: None,
                                opening_end_frame: None,
                                // Not sure if it should be div_eclid or div_ceil
                                opening_start_normalized: Some(
                                    opening_start_time.as_secs_f64() / total_secs,
                                ),
                                opening_end_normalized: Some(
                                    opening_end_time.as_secs_f64() / total_secs,
                                ),
                                ..Default::default()
                            };
                            Some(ai_label)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(label) = zaoai_label {
            let base_dir = path_buf.as_ref();

            let top_level_dir = get_top_level_dir(&path_buf, base_dir)?.ok_or_else(|| {
                anyhow::anyhow!("File is directly in base_dir without subdirectory")
            })?;

            let output_path = out_path
                .as_ref()
                .join(&top_level_dir)
                .join(path_buf.file_name().unwrap())
                .with_extension("chapters.txt");

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if output_path.exists() {
                eprintln!(
                    "Warning: Output file already exists and will be overwritten: {}",
                    output_path.display()
                );
            }

            let mut file = File::create(&output_path)?;
            let json = serde_json::to_string_pretty(&label)?;

            writeln!(file, "{}", json)?;
            println!("Wrote: {}", output_path.display());
        }
    }

    Ok(())
}
