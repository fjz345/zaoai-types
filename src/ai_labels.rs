/*

{
  "file": "episode_03.mkv",
  "opening_start": {
    "frame": 2212,
    "timestamp": 92.312,
    "normalized": 0.17
  },
  "opening_end": {
    "frame": 3665,
    "timestamp": 152.750,
    "normalized": 0.28
  }
}

*/

use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use crate::chapters::{Chapters, VideoMetadata};

#[derive(Deserialize, Serialize, Debug)]
pub struct ZaoaiLabel {
    pub path: PathBuf,
    pub metadata: VideoMetadata,
    pub version: u8,

    // Labeling for training
    pub opening_start_normalized: Option<f32>, // 0.0 - 1.0
    pub opening_end_normalized: Option<f32>,   // 0.0 - 1.0

    #[serde(with = "humantime_serde")]
    pub opening_start_time: Option<Duration>,
    #[serde(with = "humantime_serde")]
    pub opening_end_time: Option<Duration>,

    pub opening_start_frame: Option<u32>,
    pub opening_end_frame: Option<u32>,
}

impl ZaoaiLabel {
    pub fn has_opening(&self) -> bool {
        self.opening_start_frame.is_some() && self.opening_end_frame.is_some()
    }
}
