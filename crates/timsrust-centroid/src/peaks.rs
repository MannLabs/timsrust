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
        let tofs = self.get_transposed_tofs(index)?;
        let frame = self.frame_reader().get_frame(index).unwrap();
        let peaks = if frame.info().ms_level() == MSLevel::MS1 {
            self.centroider_ms1.centroid(tofs, frame.index()).collect()
        } else if frame.info().ms_level() == MSLevel::MS2 {
            self.centroider_ms2.centroid(tofs, frame.index()).collect()
        } else {
            return Err(TimsError::new(format!(
                "Unsupported MS level for frame {}",
                index
            )));
        };
        Ok(peaks)
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
    let max_tof = usize::from(
        *frame
            .ions()
            .tof_indices()
            .iter()
            .max()
            .expect("Failed to find max TOF"),
    ) + 1;
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
    // R: FrameReader<IR, InfoR> + Sync + Send,
    // <R as FrameReader<IR, InfoR>>::Error: Sync + Send,
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
