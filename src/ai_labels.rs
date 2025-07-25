use anyhow::{Context, Result};
use std::fs;
use std::io::Write;

use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::file::relative_path_from_base;
use crate::mkv::process_mkv_file;
use crate::{
    chapters::{Chapters, VideoMetadata},
    utils::ListDirSplit,
};

pub const ZAOAI_LABEL_VERSION: u8 = 1;
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ZaoaiLabel {
    pub path: PathBuf,
    pub path_source: PathBuf,
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
    return collect_zaoai_labels_multithread(list_dir_split, out_path);

    let path_source = &list_dir_split.path_source.clone();
    for entry_with_chapters in &list_dir_split.with_chapters {
        let path_buf = entry_with_chapters.as_ref();
        let zaoai_label = if path_buf.is_file() {
            if path_buf.is_file() {
                let b = process_mkv_file(&entry_with_chapters);
                match b {
                    Ok(mkv_metadata) => {
                        let (Some(op_start), Some(op_end)) = mkv_metadata.extract_opening_times()
                        else {
                            // Same effect as if zaoai_label was None
                            return Ok(());
                        };

                        let opening_start_time = op_start;
                        let opening_end_time = op_end;

                        let video_metadata: VideoMetadata = mkv_metadata.into();
                        let total_secs = video_metadata.duration.as_secs_f64();

                        let ai_label = ZaoaiLabel {
                            path: path_buf.clone(),
                            path_source: path_source.clone(),
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
            // println!("path_soruce: {}", label.path_source.display());
            // println!("path_buf: {}", path_buf.display());

            let relative_path = relative_path_from_base(path_buf, &label.path_source)?;
            // println!("relative: {}", Path::new(relative_path).display());

            let output_path = out_path.as_ref().join(relative_path).with_extension("zlbl");

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            println!("{}", output_path.display());
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

pub fn collect_zaoai_labels_multithread(
    list_dir_split: &ListDirSplit,
    out_path: impl AsRef<Path>,
) -> Result<()> {
    let out_path = out_path.as_ref().to_path_buf(); // clone for thread move
    let path_source = list_dir_split.path_source.clone();

    std::thread::scope(|scope| {
        let mut handles = vec![];

        for entry in list_dir_split.with_chapters.iter().cloned() {
            let path_source = path_source.clone();
            let out_path = out_path.clone();

            let handle = scope.spawn(move || -> Result<(), anyhow::Error> {
                let path_buf = entry.as_ref();

                if !path_buf.is_file() {
                    return Ok(()); // skip non-files
                }

                let mkv_metadata = match process_mkv_file(&entry) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("process_mkv_file error: {e}");
                        return Ok(());
                    }
                };

                let (Some(op_start), Some(op_end)) = mkv_metadata.extract_opening_times() else {
                    // Same effect as if zaoai_label was None
                    return Ok(());
                };

                let video_metadata: VideoMetadata = mkv_metadata.into();
                let total_secs = video_metadata.duration.as_secs_f64();

                let label = ZaoaiLabel {
                    path: path_buf.to_path_buf(),
                    path_source: path_source.clone(),
                    metadata: video_metadata,
                    version: ZAOAI_LABEL_VERSION,
                    opening_start_time: Some(op_start),
                    opening_end_time: Some(op_end),
                    opening_start_frame: None,
                    opening_end_frame: None,
                    opening_start_normalized: Some(op_start.as_secs_f64() / total_secs),
                    opening_end_normalized: Some(op_end.as_secs_f64() / total_secs),
                    ..Default::default()
                };

                let relative_path = relative_path_from_base(path_buf, &label.path_source)
                    .with_context(|| format!("Failed to compute relative path"))?;
                let output_path = out_path.join(relative_path).with_extension("zlbl");

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                if output_path.exists() {
                    eprintln!(
                        "Warning: Output file already exists: {}",
                        output_path.display()
                    );
                }

                let mut file = File::create(&output_path)?;
                let json = serde_json::to_string_pretty(&label)?;
                writeln!(file, "{}", json)?;

                println!("Wrote: {}", output_path.display());
                Ok(())
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Err(e) = handle.join().expect("thread panicked") {
                eprintln!("Worker failed: {e}");
            }
        }

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
