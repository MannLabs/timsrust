use std::sync::Arc;

use indicatif::{ParallelProgressIterator, ProgressStyle};
use rayon::prelude::*;
use timsrust::{ImConverter, MzConverter, RtConverter};
use timsrust_centroid::{
    PeakReader, TimsError, TimsResult,
    spectrum_reader::{
        NarrowSpectrumReader, SpectrumReader,
        narrow_spectrum_reader::{QuadInfo, SpectralChunk, split_peaks},
    },
};
use timsrust_tdf::{FrameInfoReader, TdfFrameReader, TdfIonReader};

use timsrust_core::{Converter, utils::thread::Synced};
use timsrust_core::{
    ConvertibleTo, FrameIndex, Im, Mz, Rt, ScanIndex, TofIndex,
    io::formats::parquet::ParquetWriter,
};
use timsrust_mgf::MGFWriter;
// use timsrust_tdf::{Metadata, Scan2ImConverter, Tof2MzConverter};

use crate::{CoordinatePeak, FullPeak, Precursor};

type TdfPeakReader = PeakReader<TdfIonReader, FrameInfoReader>;
type TdfNarrowReader = NarrowSpectrumReader<
    TdfIonReader,
    FrameInfoReader,
    ImConverter,
    MzConverter,
>;
type TdfSpectrumReader =
    SpectrumReader<TdfIonReader, FrameInfoReader, ImConverter, MzConverter>;

fn make_tdf_frame_reader(
    path: impl AsRef<str>,
) -> TimsResult<timsrust_core::FrameReader<TdfIonReader, FrameInfoReader>> {
    TdfFrameReader::new(path.as_ref())
        .map(|r| r.into_inner())
        .map_err(|e| TimsError::new(e.to_string()))
}

/// Runs the 2D centroiding process on the input file and writes the results to the output file.
///
/// # Arguments
/// * `in_path` - Path to the input file containing a .d folder.
/// * `out_path` - Path to the output (WARNING: will be overwritten). Needs to be .parquet or .mgf.
/// * `min_ion_count_ms1` - Minimum number of ions required for a peak to be considered in MS1.
/// * `min_ion_count_ms2` - Minimum number of ions required for a peak to be considered in MS2.
/// * `min_spectrum_size` - Minimum number of peaks required for a spectrum to be written (only for .mgf output).
///
/// # Returns
/// * `TimsResult<()>` - Returns `Ok(())` if successful, or an error otherwise.
///
/// # Example
/// ```no_run
/// use timsrust_centroid_cli::run;
/// let result = run("raw_data.d", "output.parquet", 10.0, 2.0, 5, false);
/// ```
pub fn run(
    in_path: impl AsRef<str>,
    out_path: impl AsRef<str>,
    min_ion_count_ms1: f64,
    min_ion_count_ms2: f64,
    min_spectrum_size: usize,
    use_precursors: bool,
) -> TimsResult<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    let mz_converter = timsrust::MzConverter::new(&in_path).unwrap();
    let im_converter = timsrust::ImConverter::new(&in_path).unwrap();
    let rt_converter = timsrust::RtConverter::new(&in_path).unwrap();
    match out_path.as_ref() {
        out_path if out_path.ends_with(".spec.parquet") => run_parquet_mgf(
            in_path,
            out_path,
            min_ion_count_ms1,
            min_ion_count_ms2,
            min_spectrum_size,
            use_precursors,
            mz_converter,
            im_converter,
            rt_converter,
        ),
        out_path if out_path.ends_with(".parquet") => run_parquet(
            in_path,
            out_path,
            min_ion_count_ms1,
            min_ion_count_ms2,
            &mz_converter,
            &im_converter,
            &rt_converter,
        ),
        out_path if out_path.ends_with(".mgf") => run_mgf(
            in_path,
            out_path,
            min_ion_count_ms1,
            min_ion_count_ms2,
            min_spectrum_size,
            use_precursors,
            mz_converter,
            im_converter,
        ),
        _ => Err(TimsError::new(
            "Output file must end with .parquet or .mgf".to_string(),
        )),
    }
}

fn run_parquet(
    in_path: impl AsRef<str>,
    out_path: impl AsRef<str>,
    min_ion_count_ms1: f64,
    min_ion_count_ms2: f64,
    mz_converter: &MzConverter,
    im_converter: &ImConverter,
    rt_converter: &RtConverter,
) -> TimsResult<()> {
    let time = std::time::Instant::now();
    log::info!("Running 2D centroiding on {}", in_path.as_ref());
    let peak_reader = {
        let fr = make_tdf_frame_reader(&in_path)?;
        PeakReader::new(fr, min_ion_count_ms1, min_ion_count_ms2)?
    };
    let synced_writer =
        Synced::from(ParquetWriter::new(out_path.as_ref()).unwrap());
    log::info!("Found {} frames", peak_reader.frame_count());
    log::info!("Calculated TOF FWHM: {}", peak_reader.tof_fwhm());
    log::info!("Calculated scan FWHM: {}", peak_reader.scan_fwhm());
    log::info!("Using min_ion_count_ms1: {}", min_ion_count_ms1);
    log::info!("Using min_ion_count_ms2: {}", min_ion_count_ms2);
    let peak_count = centroid_all_frames(
        peak_reader,
        synced_writer,
        mz_converter,
        im_converter,
        rt_converter,
    )?;
    log::info!(
        "Wrote {} peaks to {} in {:?}",
        peak_count,
        out_path.as_ref(),
        time.elapsed()
    );
    Ok(())
}

fn centroid_all_frames(
    peak_reader: TdfPeakReader,
    synced_writer: Synced<ParquetWriter<FullPeak>>,
    mz_converter: &MzConverter,
    im_converter: &ImConverter,
    rt_converter: &RtConverter,
) -> TimsResult<usize> {
    let peak_count = (0..peak_reader.frame_count())
        .into_par_iter()
        .progress_with_style(
            ProgressStyle::default_bar()
                .template(" [{elapsed_precise}] {bar} {pos:>7}/{len:7} ({eta}, {per_sec} frames/s)")
                .expect("Failed to set progress style")
        )
        .map(|index| {
            if let Ok(peaks) = peak_reader.get_peaks_from_frame(index) {
                let frame = peak_reader.frame_reader().get_frame(index).unwrap();
                let quad_info = frame.info().quadrupole_settings().clone();
                let peaks = peaks.into_iter().map(|p| {
                    let isolation_window = quad_info.get_isolation_window(p.scan as usize);
                    let isolation_window_lower = isolation_window.as_ref().map(|window| f64::from(window.lower()));
                    let isolation_window_upper = isolation_window.as_ref().map(|window| f64::from(window.upper()));
                    FullPeak {
                            frame: p.frame,
                            scan: p.scan,
                            tof: p.tof,
                            apex_intensity: p.apex_intensity,
                            rt: f64::from(
                                FrameIndex::try_from(p.frame)
                                    .unwrap()
                                    .convert(&rt_converter),
                            ),
                            im: f64::from(
                                ScanIndex::try_from(p.scan)
                                    .unwrap()
                                    .convert(&im_converter),
                            ),
                            mz: f64::from(
                                TofIndex::try_from(p.tof)
                                    .unwrap()
                                    .convert(&mz_converter),
                            ),
                            isolation_window_lower,
                            isolation_window_upper,
                        }}
                ).collect::<Vec<_>>();
                let len = peaks.len();
                let _ = synced_writer
                    .with_lock(|w| w.write_batch(
                        peaks))
                    .expect("Failed to write batch");
                len
            } else {
                0
            }
        })
        .sum::<usize>();
    Ok(peak_count)
}

#[allow(clippy::too_many_arguments)]
fn run_parquet_mgf(
    in_path: impl AsRef<str>,
    out_path: impl AsRef<str>,
    min_ion_count_ms1: f64,
    min_ion_count_ms2: f64,
    min_spectrum_size: usize,
    use_precursors: bool,
    mz_converter: MzConverter,
    im_converter: ImConverter,
    rt_converter: RtConverter,
) -> TimsResult<()> {
    let time = std::time::Instant::now();
    log::info!("Running 2D centroiding on {}", in_path.as_ref());
    let peak_reader = {
        let fr = make_tdf_frame_reader(&in_path)?;
        PeakReader::new(fr, min_ion_count_ms1, min_ion_count_ms2)?
    };
    let spectrum_reader = {
        let fr = make_tdf_frame_reader(&in_path)?;
        let pr = PeakReader::new(fr, min_ion_count_ms1, min_ion_count_ms2)?;
        NarrowSpectrumReader::new(
            pr,
            min_spectrum_size,
            Arc::new(im_converter.clone()),
            Arc::new(mz_converter.clone()),
        )?
    };
    log::info!("Found {} frames", spectrum_reader.frame_count());
    log::info!("Calculated TOF FWHM: {}", spectrum_reader.tof_fwhm());
    log::info!("Calculated scan FWHM: {}", spectrum_reader.scan_fwhm());
    log::info!("Using min_ion_count_ms1: {}", min_ion_count_ms1);
    log::info!("Using min_ion_count_ms2: {}", min_ion_count_ms2);
    log::info!("Using min_spectrum_size: {}", min_spectrum_size);
    log::info!("Using precursors: {}", use_precursors);
    let frag_path = out_path
        .as_ref()
        .replace(".spec.parquet", ".fragments.parquet");
    let prec_path = out_path
        .as_ref()
        .replace(".spec.parquet", ".precursors.parquet");
    let precursor_count = std::sync::atomic::AtomicUsize::new(0);
    let synced_fragment_writer =
        Synced::from(ParquetWriter::new(frag_path).unwrap());
    let synced_precursor_writer =
        Synced::from(ParquetWriter::new(prec_path).unwrap());
    (0..spectrum_reader.frame_count())
        .into_par_iter()
        .progress_with_style(
            ProgressStyle::default_bar()
                .template(
                    " [{elapsed_precise}] {bar} {pos:>7}/{len:7} ({eta}, {per_sec} frames/s)",
                )
                .expect("Failed to set progress style"),
        )
        .for_each(|index| {
            if let Some(chunk) = spectrum_reader.get_spectral_chunk_from_frame(index) {
                let count = write_spectrum_parquet_chunk(
                    &chunk,
                    &peak_reader,
                    &spectrum_reader,
                    &synced_fragment_writer,
                    &synced_precursor_writer,
                    &mz_converter,
                    &im_converter,
                    &rt_converter,
                );
                precursor_count.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
            }
        });
    log::info!(
        "Wrote {} precursors to {} in {:?}",
        precursor_count.load(std::sync::atomic::Ordering::Relaxed),
        out_path.as_ref(),
        time.elapsed()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_spectrum_parquet_chunk(
    chunk: &SpectralChunk,
    peak_reader: &TdfPeakReader,
    spectrum_reader: &TdfNarrowReader,
    synced_fragment_writer: &Synced<ParquetWriter<CoordinatePeak>>,
    synced_precursor_writer: &Synced<ParquetWriter<Precursor>>,
    mz_converter: &MzConverter,
    im_converter: &ImConverter,
    rt_converter: &RtConverter,
) -> usize {
    let precursors = chunk
        .peaks
        .iter()
        .flat_map(|(ms2_frame_index, ms2_peaks)| {
            let frame = peak_reader
                .frame_reader()
                .get_frame(*ms2_frame_index)
                .unwrap();
            let quad_info = frame.info().quadrupole_settings().clone();
            let frags = ms2_peaks
                .iter()
                .map(|p| CoordinatePeak {
                    frame: p.frame,
                    scan: p.scan,
                    tof: p.tof,
                    apex_intensity: p.apex_intensity,
                    rt: f64::from(
                        FrameIndex::try_from(p.frame)
                            .unwrap()
                            .convert(&rt_converter),
                    ),
                    im: f64::from(
                        ScanIndex::try_from(p.scan)
                            .unwrap()
                            .convert(&im_converter),
                    ),
                    mz: f64::from(
                        TofIndex::try_from(p.tof)
                            .unwrap()
                            .convert(&mz_converter),
                    ),
                })
                .collect::<Vec<_>>();
            let current_frag_offset = synced_fragment_writer
                .with_lock(|w| {
                    let offset = w.shape().0;
                    let _ = w.write_batch(frags);
                    offset
                })
                .expect("Failed to write batch");
            split_peaks(
                ms2_peaks,
                spectrum_reader.scan_fwhm(),
                &chunk.precursors,
            )
            .filter_map(|(p_id, start, end, scan)| {
                if (end - start) < spectrum_reader.min_spectrum_size() {
                    return None;
                }
                let precursor = &chunk.precursors[p_id];
                let q = QuadInfo::new(&quad_info, scan);
                if !q.is_valid_for_precursor(precursor) {
                    return None;
                }
                let rt = f64::from(precursor.rt());
                let im =
                    f64::from(im_converter.convert(precursor.scan_index()));
                let mz = f64::from(precursor.mz());
                Some(Precursor {
                    frame: u32::from(rt_converter.convert(Rt::from(rt))),
                    scan: u32::from(im_converter.convert(Im::from(im))),
                    tof: u32::from(mz_converter.convert(Mz::from(mz))),
                    apex_intensity: precursor.intensity().unwrap() as u64,
                    rt,
                    im,
                    mz,
                    start: (current_frag_offset + start) as u64,
                    end: (current_frag_offset + end) as u64,
                    charge: i8::from(precursor.charge().unwrap()) as u8,
                    index: precursor.index() as u32,
                    isolation_mz: q.isolation_mz,
                    isolation_width: q.isolation_width,
                    ce: q.ce,
                })
            })
            .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let result = precursors.len();
    synced_precursor_writer
        .with_lock(|w| {
            let _ = w.write_batch(precursors);
        })
        .expect("Failed to write batch");
    result
}

#[allow(clippy::too_many_arguments)]
fn run_mgf(
    in_path: impl AsRef<str>,
    out_path: impl AsRef<str>,
    min_ion_count_ms1: f64,
    min_ion_count_ms2: f64,
    min_spectrum_size: usize,
    use_precursors: bool,
    mz_converter: MzConverter,
    im_converter: ImConverter,
) -> TimsResult<()> {
    let time = std::time::Instant::now();
    log::info!("Running 2D centroiding on {}", in_path.as_ref());
    let spectrum_reader: TdfSpectrumReader = {
        let fr = make_tdf_frame_reader(in_path.as_ref())?;
        SpectrumReader::new(
            fr,
            min_ion_count_ms1,
            min_ion_count_ms2,
            min_spectrum_size,
            use_precursors,
            im_converter,
            mz_converter.clone(),
        )?
    };
    let mz_converter = Arc::new(mz_converter);
    let mgf_writer = MGFWriter::new(out_path.as_ref());
    log::info!("Found {} frames", spectrum_reader.frame_count());
    log::info!("Calculated TOF FWHM: {}", spectrum_reader.tof_fwhm());
    log::info!("Calculated scan FWHM: {}", spectrum_reader.scan_fwhm());
    log::info!("Using min_ion_count_ms1: {}", min_ion_count_ms1);
    log::info!("Using min_ion_count_ms2: {}", min_ion_count_ms2);
    log::info!("Using min_spectrum_size: {}", min_spectrum_size);
    log::info!("Using precursors: {}", use_precursors);
    let synced_mgf_writer = Synced::from(mgf_writer);
    (0..spectrum_reader.frame_count())
        .into_par_iter()
        .progress_with_style(
            ProgressStyle::default_bar()
                .template(
                    " [{elapsed_precise}] {bar} {pos:>7}/{len:7} ({eta}, {per_sec} frames/s)",
                )
                .expect("Failed to set progress style"),
        )
        .for_each(|index| {
            let spectra = spectrum_reader.get_spectra_from_frame(index);
            _ = synced_mgf_writer.with_lock(|mgf_writer| {
                for spectrum in spectra.into_iter() {
                    let spectrum = spectrum.to_mz_spectrum(mz_converter.as_ref());
                    mgf_writer.write(&spectrum);
                }
            });
        });
    log::info!(
        "Wrote {} spectra to {} in {:?}",
        spectrum_reader.len(),
        out_path.as_ref(),
        time.elapsed()
    );
    Ok(())
}
