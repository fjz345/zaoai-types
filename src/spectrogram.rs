use anyhow::{Context, Result, anyhow};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use sonogram::{ColourGradient, SonogramError, SpecCompute, SpecOptionsBuilder, Spectrogram};

use crate::sound::decode_samples_from_file;

pub const SPECTOGRAM_WIDTH: usize = 512;
pub const SPECTOGRAM_HEIGHT: usize = 512;
pub fn generate_spectogram(
    path: &PathBuf,
    num_spectogram_bins: usize,
) -> Result<Spectrogram, SonogramError> {
    let (samples, sample_rate) = decode_samples_from_file(&path.as_path());

    let mut spectrobuilder = SpecOptionsBuilder::new(num_spectogram_bins)
        .load_data_from_memory_f32(samples, sample_rate)
        .build()?;

    let spectogram = spectrobuilder.compute();

    Ok(spectogram)
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();
pub fn save_spectrogram(
    spectogram: &Spectrogram,
    width: usize,
    height: usize,
    path: impl AsRef<Path>,
) -> Result<()> {
    let spectrogram_buffer = spectogram.to_buffer(sonogram::FrequencyScale::Linear, width, height);
    let data = (width, height, spectrogram_buffer);
    let bytes = bincode::encode_to_vec(data, BINCODE_CONFIG)?;

    if path.as_ref().exists() {
        println!("{}, already exists", path.as_ref().to_string_lossy());
    }
    let mut file = File::create(path.as_ref())
        .with_context(|| format!("Failed to create file at {}", path.as_ref().display()))?;
    file.write_all(&bytes)?;

    Ok(())
}

pub fn load_spectrogram(
    path: impl AsRef<Path>,
    out_width: &mut usize,
    out_height: &mut usize,
) -> Result<Spectrogram> {
    let bytes = std::fs::read(&path)?;
    let (width, height, buffer): (usize, usize, Vec<f32>) =
        bincode::decode_from_slice(&bytes, BINCODE_CONFIG).map(|(v, _)| v)?;

    let mut spectrogram = create_spectrogram_unsafe(buffer, width, height);

    let mut test_path = path.as_ref().to_path_buf();
    test_path.set_extension("png");
    spectrogram.to_png(
        &test_path,
        sonogram::FrequencyScale::Log,
        &mut ColourGradient::black_white_theme(),
        width,
        height,
    )?;

    *out_width = width;
    *out_height = height;
    Ok(spectrogram)
}

pub fn create_spectrogram_unsafe(spec: Vec<f32>, width: usize, height: usize) -> Spectrogram {
    struct SpectrogramRepr {
        spec: Vec<f32>,
        width: usize,
        height: usize,
    }

    let repr = SpectrogramRepr {
        spec,
        width,
        height,
    };

    unsafe { std::mem::transmute::<SpectrogramRepr, Spectrogram>(repr) }
}
