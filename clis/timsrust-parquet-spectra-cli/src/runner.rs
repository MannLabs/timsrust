// use indicatif::{ParallelProgressIterator, ProgressStyle};
use rayon::prelude::*;
use timsrust_core::utils::thread::Synced;

use crate::Fragment;
use crate::{ParquetWriter, Precursor};

pub fn run(
    in_path: impl AsRef<str>,
    out_path: impl AsRef<str>,
    min_spectrum_size: usize,
) {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    let time = std::time::Instant::now();
    log::info!("Running on file {}", in_path.as_ref());
    let spectrum_reader = timsrust::SpectrumReader::new(in_path).unwrap();
    log::info!("Using min_spectrum_size: {}", min_spectrum_size);
    let frag_path = out_path
        .as_ref()
        .replace(".spec.parquet", ".fragments.parquet");
    let prec_path = out_path
        .as_ref()
        .replace(".spec.parquet", ".precursors.parquet");
    let synced_fragment_writer =
        Synced::from(ParquetWriter::new(frag_path).unwrap());
    let synced_precursor_writer =
        Synced::from(ParquetWriter::new(prec_path).unwrap());
    let precursors = spectrum_reader
        .par_iter()
        .filter_map(|x| {
            if let Ok(spectrum) = x {
                if spectrum.len() < min_spectrum_size {
                    return None;
                }
                if let Some(precursor) = spectrum.precursor() {
                    let fragments = spectrum
                        .intensities()
                        .iter()
                        .zip(spectrum.mz_values())
                        .map(|(intensity, mz)| Fragment {
                            mz: f64::from(*mz),
                            apex_intensity: *intensity as u64,
                        })
                        .collect();
                    let current_frag_offset = synced_fragment_writer
                        .with_lock(|w| {
                            let offset = w.shape().0;
                            let _ = w.write_batch(fragments);
                            offset
                        })
                        .expect("Failed to write batch");
                    let rt = f64::from(precursor.rt());
                    let im = f64::from(precursor.im());
                    let mz = f64::from(precursor.mz());
                    let parquet_precursor = Precursor {
                        frame: u32::from(precursor.frame_index()),
                        scan: u32::from(precursor.scan_index()),
                        tof: 0, //todo
                        // apex_intensity: precursor.intensity().unwrap() as u64,
                        apex_intensity: match precursor.intensity() {
                            Some(i) => *i as u64,
                            _ => 0,
                        },
                        rt,
                        im,
                        mz,
                        start: current_frag_offset as u64,
                        end: (current_frag_offset + spectrum.len()) as u64,
                        charge: match precursor.charge() {
                            Some(c) => i8::from(*c) as u8,
                            _ => 0,
                        },
                        index: precursor.index() as u32,
                        isolation_mz: f64::from(
                            spectrum.isolation_window().center(),
                        ),
                        isolation_width: f64::from(
                            spectrum.isolation_window().width(),
                        ),
                        ce: spectrum.isolation_window().collision_energy(),
                    };
                    return Some(parquet_precursor);
                };
            };
            None
        })
        .collect::<Vec<_>>();
    let len = precursors.len();
    synced_precursor_writer
        .with_lock(|w| {
            let _ = w.write_batch(precursors);
        })
        .expect("Failed to write batch");
    log::info!(
        "Wrote {} precursors to {} in {:?}",
        len,
        out_path.as_ref(),
        time.elapsed()
    );
}
