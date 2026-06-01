use crate::{Peak, PeakReader, TimsResult, spectrum_reader::TimsCentroidError};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::sync::atomic;
use timsrust_core::FrameInfo;
use timsrust_core::utils::reader::ParIterableReader;
use timsrust_core::utils::reader::Reader;
use timsrust_core::{
    Charge, Converter, FrameIndex, Mz, Precursor, Rt, ScanIndex, Spectrum,
    TofIndex,
};
use timsrust_core::{Im, InvertibleConverter};

pub struct NarrowSpectrumReader<
    IonReader,
    InfoReader,
    ImConverter: InvertibleConverter<ScanIndex, Im>,
    MzConverter: Converter<TofIndex, Mz>,
> {
    peak_reader: PeakReader<IonReader, InfoReader>,
    im_converter: Arc<ImConverter>,
    mz_converter: Arc<MzConverter>,
    spectrum_id: atomic::AtomicUsize,
    precursor_id: atomic::AtomicUsize,
    min_spectrum_size: usize,
    monoisotopic_only: bool,
    highest_charge_state_only: bool,
    charges: Vec<u8>,
}

impl<
    IonReader: timsrust_core::utils::reader::Reader<timsrust_core::FrameIons>
        + Sync
        + Send,
    InfoReader: timsrust_core::utils::reader::Reader<timsrust_core::FrameInfo>
        + timsrust_core::utils::reader::IndexedReader<timsrust_core::FrameInfo>
        + Sync
        + Send,
    ImConverter: InvertibleConverter<ScanIndex, Im> + Sync + Send,
    MzConverter: Converter<TofIndex, Mz> + Sync + Send,
> NarrowSpectrumReader<IonReader, InfoReader, ImConverter, MzConverter>
{
    pub fn new(
        peak_reader: PeakReader<IonReader, InfoReader>,
        min_spectrum_size: usize,
        im_converter: Arc<ImConverter>,
        mz_converter: Arc<MzConverter>,
    ) -> TimsResult<Self> {
        let spectrum_id = atomic::AtomicUsize::new(0);
        let precursor_id = atomic::AtomicUsize::new(0);
        let result = Self {
            peak_reader,
            mz_converter,
            im_converter,
            spectrum_id,
            precursor_id,
            min_spectrum_size,
            monoisotopic_only: true,
            highest_charge_state_only: true,
            charges: (1..6).collect(),
        };
        Ok(result)
    }

    pub fn set_charges(&mut self, charges: Vec<u8>) {
        self.charges = charges;
    }

    pub fn set_monoisotopic_only(&mut self, value: bool) {
        self.monoisotopic_only = value;
    }

    pub fn set_highest_charge_state_only(&mut self, value: bool) {
        self.highest_charge_state_only = value;
    }

    pub fn charges(&self) -> &[u8] {
        &self.charges
    }

    pub fn monoisotopic_only(&self) -> bool {
        self.monoisotopic_only
    }

    pub fn highest_charge_state_only(&self) -> bool {
        self.highest_charge_state_only
    }

    pub fn min_spectrum_size(&self) -> usize {
        self.min_spectrum_size
    }

    pub fn get_spectral_chunk_from_frame(
        &self,
        index: usize,
    ) -> Option<SpectralChunk> {
        if let Ok(frame) = self
            .peak_reader
            .frame_reader()
            .get_partial_frame_without_ions(index)
        {
            if frame.info().ms_level() != timsrust_core::MSLevel::MS1 {
                return None;
            }
            if let Ok(ms1_peaks) = self.peak_reader.get_peaks_from_frame(index)
            {
                let precursors = deisotope(
                    ms1_peaks,
                    &frame,
                    self.im_converter.as_ref(),
                    self.mz_converter.as_ref(),
                    &self.precursor_id,
                    self.peak_reader.scan_fwhm(),
                    self.monoisotopic_only,
                    self.highest_charge_state_only,
                    &self.charges,
                );
                if precursors.is_empty() {
                    return None;
                }
                let mut spectral_chunk = SpectralChunk {
                    peaks: FxHashMap::default(),
                    precursors,
                };
                for ms2_frame_index in find_ms2_frames(
                    index,
                    self.peak_reader.frame_reader().info_reader(),
                ) {
                    if let Ok(mut ms2_peaks) =
                        self.peak_reader.get_peaks_from_frame(ms2_frame_index)
                    {
                        if ms2_peaks.is_empty() {
                            continue;
                        }
                        ms2_peaks.sort_by(|a, b| a.scan.cmp(&b.scan));
                        spectral_chunk.peaks.insert(ms2_frame_index, ms2_peaks);
                    }
                }
                return Some(spectral_chunk);
            }
        }
        None
    }

    pub fn get_spectra_from_frame(
        &self,
        index: usize,
    ) -> Vec<timsrust_core::Spectrum> {
        match self.get_spectral_chunk_from_frame(index) {
            Some(chunk) => chunk
                .peaks
                .iter()
                .flat_map(|(ms2_frame_index, ms2_peaks)| {
                    create_spectra_from_ms2_peaks(
                        ms2_peaks,
                        &chunk.precursors,
                        self.peak_reader.scan_fwhm(),
                        &self.spectrum_id,
                        self.min_spectrum_size,
                        self.peak_reader
                            .frame_reader()
                            .get_partial_frame_without_ions(*ms2_frame_index)
                            .expect("Known to exist")
                            .info()
                            .quadrupole_settings(),
                    )
                })
                .collect(),
            None => vec![],
        }
    }

    pub fn _par_iter(
        &self,
    ) -> impl ParallelIterator<Item = timsrust_core::Spectrum> + '_ {
        (0..self.frame_count())
            .into_par_iter()
            .flat_map(|index| self.get_spectra_from_frame(index))
    }

    pub(crate) fn len(&self) -> usize {
        self.spectrum_id.load(atomic::Ordering::Relaxed)
    }

    pub fn scan_fwhm(&self) -> usize {
        self.peak_reader.scan_fwhm()
    }

    pub fn tof_fwhm(&self) -> usize {
        self.peak_reader.tof_fwhm()
    }

    pub fn frame_count(&self) -> usize {
        self.peak_reader.frame_count()
    }

    pub fn spectrum_id(&self) -> &atomic::AtomicUsize {
        &self.spectrum_id
    }
}

#[derive(Debug)]
pub struct SpectralChunk {
    pub peaks: FxHashMap<usize, Vec<Peak>>,
    pub precursors: Vec<timsrust_core::Precursor>,
}

impl<
    'a,
    IonReader: timsrust_core::utils::reader::Reader<timsrust_core::FrameIons>
        + Sync
        + Send,
    InfoReader: timsrust_core::utils::reader::Reader<timsrust_core::FrameInfo>
        + timsrust_core::utils::reader::IndexedReader<timsrust_core::FrameInfo>
        + Sync
        + Send,
    ImConverter: InvertibleConverter<ScanIndex, Im> + Sync + Send,
    MzConverter: Converter<TofIndex, Mz> + Sync + Send,
> ParIterableReader<'a, Spectrum>
    for NarrowSpectrumReader<IonReader, InfoReader, ImConverter, MzConverter>
{
    type Error = TimsCentroidError;

    fn par_iter(
        &'a self,
    ) -> impl ParallelIterator<Item = Result<Spectrum, Self::Error>> {
        self._par_iter().map(Ok)
    }
}

#[allow(clippy::too_many_arguments)]
fn deisotope(
    mut peaks: Vec<Peak>,
    frame: &timsrust_core::Frame,
    im_converter: impl Converter<ScanIndex, Im>,
    mz_converter: impl Converter<TofIndex, Mz>,
    precursor_id: &atomic::AtomicUsize,
    scan_fwhm: usize,
    try_monoisotopic_only: bool,
    highest_charge_state_only: bool,
    charges: &[u8],
) -> Vec<timsrust_core::Precursor> {
    peaks.sort_by_key(|p| p.scan);
    // const PROTON_MASS: f64 = 1.007276466812;
    const ISOTOPE_MASS: f64 = 1.0033548378;
    const MAX_DELTA_MZ: f64 = 0.01;
    let mut result = vec![];
    let mzs = peaks
        .iter()
        .map(|p| mz_converter.convert(TofIndex::try_from(p.tof).unwrap()))
        .collect::<Vec<_>>();
    let mut lower = 0;
    let mut upper = 0;
    for (i, peak) in peaks.iter().enumerate() {
        while (lower < peaks.len())
            && (peaks[lower].scan
                < (peak.scan.saturating_sub(scan_fwhm as u32 / 2)))
        {
            lower += 1;
        }
        while (upper < peaks.len())
            && (peaks[upper].scan < (peak.scan + scan_fwhm as u32 / 2))
        {
            upper += 1;
        }
        let mz = f64::from(mzs[i]);
        for charge in charges.iter().rev() {
            if try_monoisotopic_only {
                let previous_isotope_mz = mz - ISOTOPE_MASS / *charge as f64;
                if mzs[lower..upper].iter().any(|&x| {
                    (f64::from(x) - previous_isotope_mz).abs() < MAX_DELTA_MZ
                }) {
                    continue;
                }
            }
            let next_isotope_mz = mz + ISOTOPE_MASS / *charge as f64;
            if mzs[lower..upper]
                .iter()
                .any(|&x| (f64::from(x) - next_isotope_mz).abs() < MAX_DELTA_MZ)
            {
                // found isotope peak
                let id = precursor_id.fetch_add(1, atomic::Ordering::Relaxed);
                let scan = ScanIndex::try_from(peak.scan).unwrap();
                let precursor = timsrust_core::Precursor::new(
                    Mz::from(mz as f32),
                    im_converter.convert(scan),
                    Rt::from(frame.info().rt_in_seconds()),
                    scan,
                    Some(Charge::try_from(*charge as usize).unwrap()),
                    Some(peak.apex_intensity as f64),
                    id,
                    FrameIndex::try_from(frame.index() as u32).unwrap(),
                );
                result.push(precursor);
                if highest_charge_state_only {
                    break;
                }
            }
        }
    }
    result
}

fn find_ms2_frames(
    mut frame_index: usize,
    frame_reader: &impl Reader<FrameInfo>,
) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        frame_index += 1;
        frame_reader.get(frame_index).ok().and_then(|frame_info| {
            if frame_info.ms_level() == timsrust_core::MSLevel::MS2 {
                Some(frame_index)
            } else {
                None
            }
        })
    })
}

fn create_spectra_from_ms2_peaks(
    peaks: &[Peak],
    precursors: &[timsrust_core::Precursor],
    scan_fwhm: usize,
    spectrum_id: &atomic::AtomicUsize,
    min_spectrum_size: usize,
    quadrupole_settings: &timsrust_core::QuadrupoleSettings,
) -> Vec<timsrust_core::Spectrum> {
    assert!(precursors.is_sorted_by(|a, b| a.scan_index() <= b.scan_index()));
    assert!(peaks.is_sorted_by(|a, b| a.scan <= b.scan));
    split_peaks(peaks, scan_fwhm, precursors)
        .map(|(_precursor_id, lower_id, upper_id, scan)| {
            (
                precursors[_precursor_id].clone(),
                &peaks[lower_id..upper_id],
                scan,
            )
        })
        .filter_map(|(precursor, subpeaks, scan)| {
            to_spectrum(
                &precursor,
                subpeaks,
                quadrupole_settings,
                min_spectrum_size,
                scan,
                spectrum_id,
            )
        })
        .collect()
}

fn to_spectrum(
    precursor: &Precursor,
    subpeaks: &[Peak],
    quadrupole_settings: &timsrust_core::QuadrupoleSettings,
    min_spectrum_size: usize,
    scan: usize,
    spectrum_id: &atomic::AtomicUsize,
) -> Option<timsrust_core::Spectrum> {
    if subpeaks.len() < min_spectrum_size {
        return None;
    }
    let quad_info = QuadInfo::new(quadrupole_settings, scan);
    if !quad_info.is_valid_for_precursor(precursor) {
        return None;
    }
    let mut subpeaks = subpeaks.to_vec();
    subpeaks.sort_by(|a, b| a.tof.cmp(&b.tof));
    let intensity_values = subpeaks
        .iter()
        .map(|p| p.apex_intensity as f32)
        .collect::<Vec<_>>();
    let id = spectrum_id.fetch_add(1, atomic::Ordering::Relaxed);
    let isolation_window = timsrust_core::IsolationWindow::new_from_center(
        Mz::from(quad_info.isolation_mz),
        Mz::from(quad_info.isolation_width),
        quad_info.ce,
    );
    let spectrum = timsrust_core::Spectrum::new(
        intensity_values.into_iter().map(|x| x.into()).collect(),
        id,
        Some(precursor.clone()),
        subpeaks
            .iter()
            .map(|p| TofIndex::try_from(p.tof).unwrap())
            .collect(),
        isolation_window,
    );
    // let spectrum = timsrust_core::Spectrum {
    //     tof_indices: subpeaks
    //         .iter()
    //         .map(|p| TofIndex::try_from(p.tof).unwrap())
    //         .collect(),
    //     intensities: intensity_values.into_iter().map(|x| x.into()).collect(),
    //     precursor: Some(precursor.clone()),
    //     index: id,
    //     isolation_window,
    //     ..Default::default()
    // };
    Some(spectrum)
}

#[derive(Debug)]
pub struct QuadInfo {
    pub isolation_mz: f64,
    pub isolation_width: f64,
    pub ce: f64,
}

impl QuadInfo {
    pub fn new(
        quadrupole_settings: &timsrust_core::QuadrupoleSettings,
        scan: usize,
    ) -> Self {
        let index = quadrupole_settings
            .scan_starts
            .iter()
            .position(|&s| s > scan)
            .unwrap_or(quadrupole_settings.len())
            .max(1)
            - 1;
        let isolation_mz =
            f64::from(quadrupole_settings.isolation_windows[index].center());
        let isolation_width =
            f64::from(quadrupole_settings.isolation_windows[index].width())
                / 2.0;
        let ce =
            quadrupole_settings.isolation_windows[index].collision_energy();
        Self {
            isolation_mz,
            isolation_width,
            ce,
        }
    }

    pub fn is_valid_for_precursor(&self, precursor: &Precursor) -> bool {
        (f64::from(precursor.mz()) >= self.isolation_mz - self.isolation_width)
            && (f64::from(precursor.mz())
                <= self.isolation_mz + self.isolation_width)
    }
}

pub fn split_peaks<'a>(
    peaks: &'a [Peak],
    scan_fwhm: usize,
    precursors: &'a [timsrust_core::Precursor],
    // ) -> impl Iterator<Item = (timsrust_core::Precursor, &'a [Peak], usize)> + 'a {
) -> impl Iterator<Item = (usize, usize, usize, usize)> + 'a {
    assert!(peaks.is_sorted_by(|a, b| a.scan <= b.scan));
    let mut results = Vec::new();
    let mut lower_id = 0;
    let mut upper_id = 0;
    for (precursor_id, precursor) in precursors.iter().enumerate() {
        let scan = usize::from(precursor.scan_index());
        let start = scan.saturating_sub(scan_fwhm / 2) as u32;
        let end = (scan + scan_fwhm / 2 + 1) as u32;
        while lower_id < peaks.len() && peaks[lower_id].scan < start {
            lower_id += 1;
        }
        while upper_id < peaks.len() && peaks[upper_id].scan < end {
            upper_id += 1;
        }
        if upper_id > lower_id {
            // results.push((precursor.clone(), &peaks[lower_id..upper_id], scan));
            results.push((precursor_id, lower_id, upper_id, scan));
        }
    }
    results.into_iter()
}
