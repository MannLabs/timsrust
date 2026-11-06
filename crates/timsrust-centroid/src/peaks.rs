use crate::{TimsError, TimsResult, centroider::FrameCentroider};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use timsrust_core::utils::reader::{IndexedReader, Reader};
use timsrust_core::utils::thread::Synced;
use timsrust_core::utils::vec::extract_kernel;
use timsrust_core::utils::{ndarray::NDArray, vec::arg_max};
use timsrust_core::{Frame, FrameInfo, FrameIons, FrameReader, MSLevel};

/// Full width at half maximum (FWHM) threshold for kernel extraction.
const FWHM: f32 = 0.5;
/// Maximum TOF (time-of-flight) width for peak extraction.
const TOF_WIDTH: usize = 32;
/// Maximum scan width for peak extraction.
const SCAN_WIDTH: usize = 256;

type Grid2D = NDArray<u64, 2>;
type TOFMap = Vec<FxHashMap<u32, u64>>;
type FootprintVerifier<F> =
    fn(
        &Frame,
        &mut [Peak],
        &FrameCentroider,
    ) -> TimsResult<std::collections::BTreeMap<(u32, u32), F>>;

/// Represents a centroided peak in a frame.
///
/// # Fields
/// - `frame`: Frame index.
/// - `scan`: Scan index within the frame.
/// - `tof`: Time-of-flight index.
/// - `apex_intensity`: Intensity at the apex of the peak (unitless).
///
/// # Example
/// ```
/// use timsrust_centroid::Peak;
/// let peak = Peak {
///     frame: 1,
///     scan: 10,
///     tof: 100,
///     apex_intensity: 5000,
/// };
/// assert_eq!(peak.frame, 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Peak {
    pub frame: u32,
    pub scan: u32,
    pub tof: u32,
    pub apex_intensity: u64,
}

/// Reads and extracts centroided peaks from frames.
///
/// Generic over the ion reader (`IonReader`) and frame-info reader (`InfoReader`)
/// so that `timsrust-centroid` has no direct dependency on any file format crate.
#[derive(Debug)]
pub struct PeakReader<IonReader, InfoReader> {
    frame_reader: FrameReader<IonReader, InfoReader>,
    centroider_ms1: FrameCentroider,
    centroider_ms2: FrameCentroider,
    average_peak: Grid2D,
}

impl<IonReader, InfoReader> PeakReader<IonReader, InfoReader>
where
    IonReader: Reader<FrameIons> + Sync + Send,
    InfoReader: Reader<FrameInfo> + IndexedReader<FrameInfo> + Sync + Send,
{
    /// Constructs a new [`PeakReader`] from the given frame reader and minimum ion counts.
    ///
    /// # Arguments
    /// * `frame_reader` - A [`FrameReader`] providing frame data.
    /// * `min_ion_count_ms1` - Minimum ion count for centroiding MS1 frames.
    /// * `min_ion_count_ms2` - Minimum ion count for centroiding MS2 frames.
    ///
    /// # Errors
    /// Returns an error if kernel extraction fails.
    pub fn new(
        frame_reader: FrameReader<IonReader, InfoReader>,
        min_ion_count_ms1: f64,
        min_ion_count_ms2: f64,
    ) -> TimsResult<Self> {
        let average_peak = get_average_ms1_peak(&frame_reader)?;
        let tof_kernel = average_peak.project_axis(0);
        let tof_kernel = extract_kernel(&tof_kernel, FWHM)
            .ok_or(TimsError::new("Failed to extract kernel"))?;
        let scan_kernel = average_peak.project_axis(1);
        let scan_kernel = extract_kernel(&scan_kernel, FWHM)
            .ok_or(TimsError::new("Failed to extract kernel"))?;
        let min_ion_count_ms1 = if min_ion_count_ms1 <= 0.0 {
            usize::MAX
        } else if min_ion_count_ms1 < 1.0 {
            (min_ion_count_ms1 * scan_kernel.len() as f64).ceil() as usize
        } else {
            min_ion_count_ms1 as usize
        };
        let min_ion_count_ms2 = if min_ion_count_ms2 < 1.0 {
            (min_ion_count_ms2 * scan_kernel.len() as f64).ceil() as usize
        } else {
            min_ion_count_ms2 as usize
        };
        let centroider_ms1 =
            FrameCentroider::new(&scan_kernel, &tof_kernel, min_ion_count_ms1);
        let centroider_ms2 =
            FrameCentroider::new(&scan_kernel, &tof_kernel, min_ion_count_ms2);
        let result = Self {
            frame_reader,
            centroider_ms1,
            centroider_ms2,
            average_peak,
        };
        Ok(result)
    }

    pub fn frame_reader(&self) -> &FrameReader<IonReader, InfoReader> {
        &self.frame_reader
    }

    /// Returns the number of frames in the dataset.
    pub fn frame_count(&self) -> usize {
        self.frame_reader.len()
    }

    /// Returns the length of the TOF kernel (FWHM).
    pub fn tof_fwhm(&self) -> usize {
        self.centroider_ms1.tof_kernel().len()
    }

    /// Returns the length of the scan kernel (FWHM).
    pub fn scan_fwhm(&self) -> usize {
        self.centroider_ms1.scan_smoother().kernel().len()
    }

    /// Returns the minimum MS1 ion count used for centroiding.
    pub fn min_count_ms1(&self) -> usize {
        self.centroider_ms1.scan_smoother().min_count()
    }

    /// Returns the minimum MS2 ion count used for centroiding.
    pub fn min_count_ms2(&self) -> usize {
        self.centroider_ms2.scan_smoother().min_count()
    }

    /// Extracts centroided peaks from the specified frame index.
    ///
    /// Equal-response candidates are resolved per connected plateau in the
    /// scan/TOF grid, including diagonal adjacency. The representative is an
    /// actual candidate nearest the plateau's integer mean position.
    ///
    /// # Arguments
    /// * `index` - Frame index.
    ///
    /// # Errors
    /// Returns an error if the frame cannot be read or centroiding fails.
    ///
    /// # Example
    /// ```ignore
    /// use timsrust_centroid::PeakReader;
    /// let frame_reader = /* e.g. TdfFrameReader::new("example.d").unwrap() */;
    /// let reader = PeakReader::new(frame_reader, 10.0, 5.0).unwrap();
    /// let peaks = reader.get_peaks_from_frame(0).unwrap();
    /// ```
    pub fn get_peaks_from_frame(&self, index: usize) -> TimsResult<Vec<Peak>> {
        let frame = self
            .frame_reader()
            .get_frame(index)
            .map_err(|e| TimsError::new(e.to_string()))?;
        self.centroid_frame(&frame)
    }

    /// Centroids an already loaded frame with the same native kernels.
    pub fn centroid_frame(&self, frame: &Frame) -> TimsResult<Vec<Peak>> {
        let tofs = transpose_tofs(frame);
        let peaks = if frame.info().ms_level() == MSLevel::MS1 {
            self.centroider_ms1.centroid(tofs, frame.index()).collect()
        } else if frame.info().ms_level() == MSLevel::MS2 {
            self.centroider_ms2.centroid(tofs, frame.index()).collect()
        } else {
            return Err(TimsError::new(format!(
                "Unsupported MS level for frame {}",
                frame.index()
            )));
        };
        Ok(peaks)
    }

    /// Measures isolated weak candidates in this frame, without pooling RT intensities.
    pub fn rt_consistency_frame(
        &self,
        frame: &Frame,
    ) -> TimsResult<crate::neighborhood::RtCandidateFrame> {
        Ok(self
            .detect_candidates(
                frame,
                true,
                false,
                crate::neighborhood::raw_support::verified_footprints,
            )?
            .candidates)
    }

    /// Raw-verified transmission footprints; native measurements are unchanged.
    pub fn transmission_candidates(
        &self,
        frame: &Frame,
        include_weak: bool,
    ) -> TimsResult<crate::neighborhood::TransmissionFrame> {
        self.detect_candidates(
            frame,
            include_weak,
            true,
            crate::neighborhood::raw_support::verified_footprints,
        )
    }

    /// Experimental mass-only footprints, with exact native response verification.
    pub fn trajectory_candidates(
        &self,
        frame: &Frame,
        include_weak: bool,
    ) -> TimsResult<
        crate::neighborhood::TransmissionFrame<
            crate::neighborhood::MassTransmissionFootprint,
        >,
    > {
        self.detect_candidates(
            frame,
            include_weak,
            true,
            crate::neighborhood::raw_support::verified_mass_footprints,
        )
    }

    fn detect_candidates<F>(
        &self,
        frame: &Frame,
        include_weak: bool,
        shared: bool,
        verify: FootprintVerifier<F>,
    ) -> TimsResult<crate::neighborhood::TransmissionFrame<F>> {
        use crate::neighborhood::raw_support::{
            validate_frame, verified_support,
        };
        validate_frame(frame)?;
        let mut native = self.centroid_frame(frame)?;
        let mut footprints = if shared {
            verify(frame, &mut native, &self.centroider_ms2)?
        } else {
            Default::default()
        };
        let mut eligible = if shared {
            footprints.keys().copied().collect()
        } else {
            verified_support(frame, &mut native, &self.centroider_ms2)?
        };
        let low = self.centroider_ms2.with_min_count(1);
        let mut expanded: Vec<_> = if include_weak {
            low.centroid(transpose_tofs(frame), frame.index()).collect()
        } else {
            Vec::new()
        };
        let expanded_eligible = if shared {
            let extra = verify(frame, &mut expanded, &low)?;
            let ids = extra.keys().copied().collect();
            footprints.extend(extra);
            ids
        } else {
            verified_support(frame, &mut expanded, &low)?
        };
        let near = |peaks: &[Peak], peak: &Peak, exclude_self: bool| {
            let first = peaks.partition_point(|p| {
                u64::from(p.tof) + self.tof_fwhm() as u64 <= u64::from(peak.tof)
            });
            peaks[first..]
                .iter()
                .take_while(|p| {
                    u64::from(p.tof)
                        < u64::from(peak.tof) + self.tof_fwhm() as u64
                })
                .any(|p| {
                    (!exclude_self || p.scan != peak.scan || p.tof != peak.tof)
                        && peak.tof.abs_diff(p.tof) < self.tof_fwhm() as u32
                        && peak.scan.abs_diff(p.scan) < self.scan_fwhm() as u32
                })
        };
        let weak: Vec<_> = expanded
            .iter()
            .filter(|p| {
                expanded_eligible.contains(&(p.scan, p.tof))
                    && !near(&native, p, false)
                    && !near(&expanded, p, true)
            })
            .cloned()
            .collect();
        eligible.extend(weak.iter().map(|p| (p.scan, p.tof)));
        footprints.retain(|id, _| eligible.contains(id));
        Ok(crate::neighborhood::TransmissionFrame {
            candidates: crate::neighborhood::RtCandidateFrame {
                native,
                weak,
                eligible,
                scan_count: frame.ions().scan_count(),
            },
            footprints,
        })
    }

    // pub fn get_frame(&self, index: usize) -> TimsResult<Frame> {
    //     let result = self
    //         .frame_reader
    //         .get_partial_frame_without_ions(index)
    //         .map_err(|e| TimsError::new(e.to_string()))?;
    //     Ok(result)
    // }

    /// Returns the transposed TOF map for the specified frame index.
    ///
    /// # Arguments
    /// * `index` - Frame index.
    ///
    /// # Errors
    /// Returns an error if the frame cannot be read.
    ///
    /// # Example
    /// ```ignore
    /// use timsrust_centroid::PeakReader;
    /// let frame_reader = /* e.g. TdfFrameReader::new("example.d").unwrap() */;
    /// let reader = PeakReader::new(frame_reader, 10.0, 5.0).unwrap();
    /// let tofs = reader.get_transposed_tofs(0).unwrap();
    /// ```
    pub fn get_transposed_tofs(&self, index: usize) -> TimsResult<TOFMap> {
        if let Ok(frame) = self.frame_reader.get_frame(index) {
            Ok(transpose_tofs(&frame))
        } else {
            Err(TimsError::new(format!(
                "Failed to get transposed TOFs for frame {}",
                index
            )))
        }
    }

    /// Returns a reference to the average MS1 peak grid.
    ///
    /// # Example
    /// ```ignore
    /// use timsrust_centroid::PeakReader;
    /// let frame_reader = /* e.g. TdfFrameReader::new("example.d").unwrap() */;
    /// let reader = PeakReader::new(frame_reader, 10.0, 5.0).unwrap();
    /// let avg_peak = reader.get_average_ms1_peak();
    /// ```
    pub fn get_average_ms1_peak(&self) -> &Grid2D {
        &self.average_peak
    }
}

fn transpose_tofs(frame: &Frame) -> TOFMap {
    let Some(&max_tof) = frame.ions().tof_indices().iter().max() else {
        return Vec::new();
    };
    let max_tof = usize::from(max_tof) + 1;
    let mut tofs = vec![FxHashMap::default(); max_tof];
    frame.ions().scan_offsets().windows(2).enumerate().for_each(
        |(scan_index, s)| {
            let start = s[0];
            let end = s[1];
            for index in start..end {
                let tof = usize::from(frame.ions().tof_indices()[index]);
                let value = u64::from(frame.ions().intensities()[index]);
                tofs[tof].insert(scan_index as u32, value);
            }
        },
    );
    tofs
}

pub fn get_average_ms1_peak<IR, InfoR>(
    frame_reader: &FrameReader<IR, InfoR>,
) -> TimsResult<Grid2D>
where
    IR: Reader<FrameIons> + Sync + Send,
    InfoR: Reader<FrameInfo> + IndexedReader<FrameInfo> + Sync + Send,
{
    let synced_summed_grid =
        Synced::from(Grid2D::empty([TOF_WIDTH * 2 + 1, SCAN_WIDTH * 2 + 1]));
    frame_reader
        .parallel_filter(|f| f.info().ms_level() == MSLevel::MS1)
        .for_each(|frame| {
            if let Ok(frame) = frame {
                if frame.ions().intensities().is_empty() {
                    return;
                }
                let grid = get_best_peak_for_frame(frame);
                _ = synced_summed_grid.with_lock(|g| *g += grid);
            }
        });
    synced_summed_grid
        .try_finalize()
        .ok_or(TimsError::new("Failed to finalize synced grid"))
}

pub fn get_best_peak_for_frame(frame: Frame) -> Grid2D {
    let mut grid = Grid2D::empty([TOF_WIDTH * 2 + 1, SCAN_WIDTH * 2 + 1]);
    let max_index = arg_max(frame.ions().intensities()).unwrap_or(0);
    let max_tof = i64::from(frame.ions().tof_indices()[max_index]);
    let max_scan = frame
        .ions()
        .scan_offsets()
        .partition_point(|&offset| offset <= max_index)
        .saturating_sub(1);
    frame.ions().scan_offsets().windows(2).enumerate().for_each(
        |(scan_index, s)| {
            let start = s[0];
            let end = s[1];
            for index in start..end {
                let tof = i64::from(frame.ions().tof_indices()[index]);
                let intensity = u64::from(frame.ions().intensities()[index]);
                let tof_diff = tof - max_tof as i64;
                let scan_diff = scan_index as i64 - max_scan as i64;
                if scan_diff.abs() > SCAN_WIDTH as i64
                    || tof_diff.abs() > TOF_WIDTH as i64
                {
                    continue; // Skip if out of bounds
                }
                grid[[
                    (tof_diff + TOF_WIDTH as i64) as usize,
                    (scan_diff + SCAN_WIDTH as i64) as usize,
                ]] += intensity;
            }
        },
    );
    grid
}

#[cfg(test)]
mod iteration_tests {
    use super::*;
    use crate::spectrum_reader::WideSpectrumReader;
    use std::convert::Infallible;
    use std::sync::{Arc, Mutex};
    use timsrust_core::BitConverter;

    struct MockReader {
        indices: Vec<usize>,
        requested: Arc<Mutex<Vec<usize>>>,
    }

    impl Reader<FrameInfo> for MockReader {
        type Error = Infallible;

        fn get(&self, index: usize) -> Result<FrameInfo, Self::Error> {
            self.requested.lock().unwrap().push(index);
            Ok(FrameInfo::default())
        }
    }

    impl Reader<FrameIons> for MockReader {
        type Error = Infallible;

        fn get(&self, _: usize) -> Result<FrameIons, Self::Error> {
            unreachable!("Non-MS2 frames must not load ions")
        }
    }

    impl IndexedReader<FrameInfo> for MockReader {
        type Iter = std::vec::IntoIter<usize>;

        fn iter(&self) -> Self::Iter {
            self.indices.clone().into_iter()
        }
    }

    #[test]
    fn wide_reader_visits_actual_one_based_and_gapped_frame_ids() {
        for indices in [vec![1, 2, 3], vec![1, 4, 9]] {
            let requested = Arc::new(Mutex::new(Vec::new()));
            let reader = || MockReader {
                indices: indices.clone(),
                requested: requested.clone(),
            };
            let kernel = [0.5, 1.0, 0.5];
            let peak_reader = PeakReader {
                frame_reader: FrameReader::new(reader(), reader()),
                centroider_ms1: FrameCentroider::new(&kernel, &kernel, 1),
                centroider_ms2: FrameCentroider::new(&kernel, &kernel, 1),
                average_peak: Grid2D::new([1, 1], vec![1]).unwrap(),
            };
            let wide = WideSpectrumReader::new(
                peak_reader,
                1,
                Arc::new(BitConverter()),
            )
            .unwrap();
            assert_eq!(wide._par_iter().count(), 0);
            let mut actual = requested.lock().unwrap().clone();
            actual.sort_unstable();
            assert_eq!(actual, indices);
        }
    }
}

#[cfg(test)]
mod rt_raw_tests {
    use super::*;
    use std::sync::Arc;
    use timsrust_core::{AcquisitionType, IsolationWindow, QuadrupoleSettings};

    struct Unused;
    impl<T> Reader<T> for Unused {
        type Error = std::io::Error;
        fn get(&self, _: usize) -> Result<T, Self::Error> {
            Err(std::io::Error::other("unexpected fixture read"))
        }
    }
    impl<T> IndexedReader<T> for Unused {
        type Iter = std::iter::Empty<usize>;
        fn iter(&self) -> Self::Iter {
            std::iter::empty()
        }
    }
    fn reader() -> PeakReader<Unused, Unused> {
        PeakReader {
            frame_reader: FrameReader::new(Unused, Unused),
            centroider_ms1: FrameCentroider::new(
                &[0.5, 1.0, 0.5],
                &[0.2, 1.0, 0.2],
                2,
            ),
            centroider_ms2: FrameCentroider::new(
                &[0.5, 1.0, 0.5],
                &[0.2, 1.0, 0.2],
                2,
            ),
            average_peak: Grid2D::empty([3, 3]),
        }
    }
    fn frame(bins: &[(usize, u32, u32)], split: bool) -> Frame {
        let (mut offsets, mut tofs, mut intensities) =
            (vec![0], vec![], vec![]);
        for scan in 0..40 {
            for &(_, tof, intensity) in bins.iter().filter(|b| b.0 == scan) {
                tofs.push(tof.try_into().unwrap());
                intensities.push(intensity.try_into().unwrap());
            }
            offsets.push(tofs.len());
        }
        let q = QuadrupoleSettings {
            index: 1,
            scan_starts: if split { vec![0, 20] } else { vec![0] },
            scan_ends: if split { vec![20, 40] } else { vec![40] },
            isolation_windows: (0..if split { 2 } else { 1 })
                .map(|i| {
                    IsolationWindow::new_from_bounds(
                        400.0.into(),
                        600.0.into(),
                        30.0 + i as f64,
                    )
                })
                .collect(),
        };
        FrameIons::new(offsets, tofs, intensities).add_info(FrameInfo::new(
            Arc::new(q),
            17,
            12.0,
            1.0,
            AcquisitionType::DIAPASEF,
            MSLevel::MS2,
            1,
            None,
        ))
    }
    #[test]
    fn weak_response_is_measured_in_the_focal_frame() {
        let reader = reader();
        let frame = frame(&[(20, 100, 7)], false);
        assert!(reader.centroid_frame(&frame).unwrap().is_empty());
        let candidates = reader.rt_consistency_frame(&frame).unwrap();
        assert_eq!(candidates.weak.len(), 1);
        assert_eq!(candidates.weak[0].apex_intensity, 7 * 255);
        assert!(candidates.eligible.contains(&(20, 100)));
    }
    #[test]
    fn trajectory_verifies_actual_native_responses_and_empty_interior_scans() {
        let mut reader = reader();
        reader.centroider_ms2 = FrameCentroider::new(
            &[0.5, 0.75, 1., 0.75, 0.5],
            &[0.2, 1., 0.2],
            2,
        );
        let mut raw = frame(&[(18, 100, 7), (20, 100, 9), (22, 100, 7)], true);
        let native = reader.centroid_frame(&raw).unwrap();
        assert!(!native.is_empty());
        assert!(
            reader
                .transmission_candidates(&raw, false)
                .unwrap()
                .footprints
                .is_empty()
        );
        let output = reader.trajectory_candidates(&raw, true).unwrap();
        assert_eq!(output.candidates.native, native);
        assert!(!output.footprints.is_empty());
        let mut tampered = native.clone();
        tampered[0].apex_intensity += 1;
        assert!(
            crate::neighborhood::raw_support::verified_mass_footprints(
                &raw,
                &mut tampered,
                &reader.centroider_ms2,
            )
            .is_err()
        );
        let q = QuadrupoleSettings {
            index: 2,
            scan_starts: vec![0, 19, 20],
            scan_ends: vec![19, 20, 40],
            isolation_windows: [
                (400., 600., 30.),
                (600., 700., 30.),
                (400., 600., 31.),
            ]
            .into_iter()
            .map(|(a, b, ce)| {
                IsolationWindow::new_from_bounds(a.into(), b.into(), ce)
            })
            .collect(),
        };
        raw.set_info(FrameInfo::new(
            Arc::new(q),
            17,
            12.,
            1.,
            AcquisitionType::DIAPASEF,
            MSLevel::MS2,
            1,
            None,
        ));
        let output = reader.trajectory_candidates(&raw, false).unwrap();
        assert_eq!(output.candidates.native, native);
        assert!(output.footprints.is_empty());
    }
    #[test]
    fn native_values_are_preserved_and_cross_interval_support_is_ineligible() {
        let reader = reader();
        let raw = frame(&[(20, 100, 7), (20, 101, 9)], false);
        let candidates = reader.rt_consistency_frame(&raw).unwrap();
        assert_eq!(candidates.native, reader.centroid_frame(&raw).unwrap());
        assert!(candidates.weak.is_empty());
        let crossed = reader
            .rt_consistency_frame(&frame(&[(19, 100, 7), (20, 100, 9)], true))
            .unwrap();
        assert!(!crossed.native.is_empty());
        assert!(crossed.eligible.is_empty());
        assert!(
            reader
                .rt_consistency_frame(&frame(&[], false))
                .unwrap()
                .weak
                .is_empty()
        );
    }
}
