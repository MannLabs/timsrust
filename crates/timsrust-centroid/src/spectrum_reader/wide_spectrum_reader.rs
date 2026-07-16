use crate::{Peak, PeakReader, TimsResult};
use rayon::prelude::*;
use std::sync::{Arc, atomic};
use timsrust_core::{
    FrameIndex, Im, InvertibleConverter, Rt, ScanIndex, TofIndex,
};

pub struct WideSpectrumReader<
    IonReader,
    InfoReader,
    ImConverter: InvertibleConverter<ScanIndex, Im>,
> {
    peak_reader: PeakReader<IonReader, InfoReader>,
    im_converter: Arc<ImConverter>,
    spec_id: atomic::AtomicUsize,
    min_spectrum_size: usize,
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
> WideSpectrumReader<IonReader, InfoReader, ImConverter>
{
    pub(crate) fn new(
        peak_reader: PeakReader<IonReader, InfoReader>,
        min_spectrum_size: usize,
        im_converter: Arc<ImConverter>,
    ) -> TimsResult<Self> {
        let spec_id = atomic::AtomicUsize::new(0);
        let result = Self {
            peak_reader,
            im_converter,
            spec_id,
            min_spectrum_size,
        };
        Ok(result)
    }

    pub fn get_spectra_from_frame(
        &self,
        index: usize,
    ) -> Vec<timsrust_core::Spectrum> {
        #[allow(clippy::collapsible_if)]
        if let Ok(frame) = self
            .peak_reader
            .frame_reader()
            .get_partial_frame_without_ions(index)
        {
            if frame.info().ms_level() == timsrust_core::MSLevel::MS2 {
                if let Ok(peaks) = self.peak_reader.get_peaks_from_frame(index)
                {
                    if !peaks.is_empty() {
                        let spectra = peaks_to_spectra(
                            peaks,
                            &frame,
                            self.peak_reader.scan_fwhm(),
                            self.im_converter.as_ref(),
                            &self.spec_id,
                            self.min_spectrum_size,
                        );
                        return spectra;
                    }
                }
            }
        }
        vec![]
    }

    pub fn len(&self) -> usize {
        self.spec_id.load(atomic::Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    pub fn _par_iter(
        &self,
    ) -> impl ParallelIterator<Item = timsrust_core::Spectrum> + '_ {
        (0..self.frame_count())
            .into_par_iter()
            .flat_map(|index| self.get_spectra_from_frame(index))
    }
}

fn peaks_to_spectra(
    mut peaks: Vec<Peak>,
    frame: &timsrust_core::Frame,
    scan_fwhm: usize,
    im_converter: impl InvertibleConverter<ScanIndex, Im>,
    spec_id: &atomic::AtomicUsize,
    min_spectrum_size: usize,
) -> Vec<timsrust_core::Spectrum> {
    let mut id = frame.info().index() << 32;
    peaks.sort_by(|a, b| a.scan.cmp(&b.scan));
    split_peaks(&peaks, scan_fwhm)
        .filter_map(|(subpeaks, scan)| {
            if subpeaks.len() < min_spectrum_size {
                return None;
            }
            let frame_info = frame.info();
            let index = frame_info
                .quadrupole_settings()
                .scan_starts
                .iter()
                .position(|&s| s > scan)
                .unwrap_or(frame_info.quadrupole_settings().len())
                .max(1)
                - 1;
            let isolation_mz =
                frame_info.quadrupole_settings().isolation_windows[index]
                    .center();
            let isolation_width =
                frame_info.quadrupole_settings().isolation_windows[index]
                    .width();
            let ce = frame_info.quadrupole_settings().isolation_windows[index]
                .collision_energy();
            let intensity_values = subpeaks
                .iter()
                .map(|p| p.apex_intensity as f32)
                .collect::<Vec<_>>();
            id += 1;
            _ = spec_id.fetch_add(1, atomic::Ordering::Relaxed);
            let scan = ScanIndex::try_from(scan as u32).unwrap();
            let precursor = timsrust_core::Precursor::new(
                isolation_mz,
                im_converter.convert(scan),
                Rt::from(frame_info.rt_in_seconds()),
                scan,
                None,
                None,
                id,
                FrameIndex::try_from(frame_info.index() as u32).unwrap(),
            );
            let isolation_window =
                timsrust_core::IsolationWindow::new_from_center(
                    isolation_mz,
                    isolation_width,
                    ce,
                );
            let spectrum = timsrust_core::Spectrum::new(
                intensity_values.into_iter().map(|x| x.into()).collect(),
                id,
                Some(precursor),
                // mz_values: mz_values.into_iter().map(|x| x.into()).collect(),
                subpeaks
                    .iter()
                    .map(|p| TofIndex::try_from(p.tof).unwrap())
                    .collect(),
                isolation_window,
            );
            // let spectrum = timsrust_core::Spectrum {
            //     // mz_values: mz_values.into_iter().map(|x| x.into()).collect(),
            //     tof_indices: subpeaks
            //         .iter()
            //         .map(|p| TofIndex::try_from(p.tof).unwrap())
            //         .collect(),
            //     intensities: intensity_values
            //         .into_iter()
            //         .map(|x| x.into())
            //         .collect(),
            //     precursor: Some(precursor),
            //     index: id,
            //     isolation_window,
            //     ..Default::default()
            // };
            Some(spectrum)
        })
        .collect()
}

fn split_peaks(
    peaks: &[Peak],
    scan_fwhm: usize,
) -> impl Iterator<Item = (Vec<Peak>, usize)> + '_ {
    assert!(peaks.is_sorted_by(|a, b| a.scan <= b.scan));
    let step = (scan_fwhm) / 2;
    let width = scan_fwhm;
    let subpeaks = peaks;
    let mut results = Vec::new();
    if !subpeaks.is_empty() {
        let mut start = 0;
        let mut left = 0;
        let n = subpeaks.len();
        loop {
            let end = start + width;
            while left < n && (subpeaks[left].scan as usize) < start {
                left += 1;
            }
            if left >= n {
                break;
            }
            let mut right = left;
            while right < n && (subpeaks[right].scan as usize) < end {
                right += 1;
            }
            if left < right {
                let mut chunk = subpeaks[left..right].to_vec();
                chunk.sort_by(|a, b| a.tof.cmp(&b.tof));
                let avg_scan = start + step;
                results.push((chunk, avg_scan));
            }
            start += step;
        }
    }
    results.into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_peak(scan: u32, tof: u32, intensity: u32) -> Peak {
        Peak {
            scan,
            tof,
            apex_intensity: intensity as u64,
            // Fill with dummy/defaults for any other fields if needed
            ..Default::default()
        }
    }

    #[test]
    fn test_split_peaks_runs() {
        let peaks = vec![
            dummy_peak(100, 100, 1000),
            dummy_peak(120, 110, 900),
            dummy_peak(150, 120, 800),
            dummy_peak(200, 130, 700),
        ];
        let result = split_peaks(&peaks, 2).collect::<Vec<_>>();
        assert!(!result.is_empty());
        let total = result.iter().map(|(v, _)| v.len()).sum::<usize>();
        assert_eq!(total, 2 * peaks.len());
    }

    // #[test]
    // fn test_peaks_to_spectra_empty() {
    //     // Use dummy converters and frame
    //     let peaks = vec![dummy_peak(10, 100, 1000)];
    //     // let frame = unsafe { std::mem::zeroed() }; // Only for minimal test, not for real use!
    //     let frame = Frame::default();
    //     let im_converter = unsafe { std::mem::zeroed() };
    //     let mz_converter = unsafe { std::mem::zeroed() };
    //     let spec_id = AtomicUsize::new(0);
    //     let spectra = peaks_to_spectra(
    //         peaks,
    //         &frame,
    //         4,
    //         im_converter,
    //         mz_converter,
    //         &spec_id,
    //         2,
    //     );
    //     assert!(spectra.is_empty());
    // }
}
