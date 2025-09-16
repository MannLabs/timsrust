use super::{AcquisitionType, QuadrupoleSettings};
use std::sync::Arc;

/// A frame with all unprocessed data as it was acquired.
///
/// For details about the fields see [FrameMeta] and [FramePeaks]
/// check their documentation. (details about the compression are also
/// in the FramePeaks docs)
///
/// The most important method is probably [Frame::iter_corrected_peaks] which
/// iterates over all peaks in the frame and applies the intensity correction.
///
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub peaks: FramePeaks,
    pub meta: FrameMeta,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameCalibration {
    pub calibration_id: u8,
    pub t1: f64,
    pub t2: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowGroupInfo {
    pub window_group: u8,
    pub quadrupole_settings: Arc<QuadrupoleSettings>,
}

/// Metadata about a frame.
///
/// This is meant to contain essentially all the information about
/// a frame except the actual peaks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameMeta {
    pub index: usize,
    pub rt_in_seconds: f64,
    pub acquisition_type: AcquisitionType,
    pub ms_level: MSLevel,
    pub intensity_correction_factor: f64,
    pub window_group: Option<WindowGroupInfo>,
    pub calibration: FrameCalibration,
}

/// The actual peaks in a frame.
///
/// This is essentially a multi-array and a run-length encoded
/// vector of scan offsets to indicate which mobility each peak belongs to.
///
/// The lengths of the vectors should always be:
/// - scan_offsets.len() == number of scans + 1
/// - tof_indices.len() == intensities.len()
/// - intensities.len() == number of peaks
/// - scan_offsets[0] == 0
/// - scan_offsets.last() == number of peaks
///
/// Scan_offsets is sorted in ascending order and windowed-start-end
/// encoded. As an example [0, 0, 0, 1, 4, 4, 4] would mean:
/// - 0-0: no peaks in scan 0
/// - 0-0: no peaks in scan 1
/// - 0-1: 1 peak in scan 2
/// - 1-4: 3 peaks in scan 3
/// - 4-4: no peaks in scan 4 ...
///
/// The simple way of using this is to just call iter_peaks()
/// which will expand the scan offsets to scan indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FramePeaks {
    pub scan_offsets: Vec<u32>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
}

impl FramePeaks {
    pub fn len(&self) -> usize {
        self.intensities.len()
    }
    pub fn is_empty(&self) -> bool {
        self.intensities.is_empty()
    }

    /// This in theory would be used to pre-allocate space
    /// and re-use the vectors when reading frames with
    /// FrameReader.get_buffered(...)
    pub fn with_capacity(scan_capacty: usize, peak_capacity: usize) -> Self {
        Self {
            scan_offsets: Vec::with_capacity(scan_capacty),
            tof_indices: Vec::with_capacity(peak_capacity),
            intensities: Vec::with_capacity(peak_capacity),
        }
    }

    pub fn clear(&mut self) {
        self.scan_offsets.clear();
        self.tof_indices.clear();
        self.intensities.clear();
    }

    /// Expand the scan offset slice to mobilities.
    ///
    /// The scan offsets is in essence a run-length
    /// encoded vector of scan numbers that can be converter to the 1/k0
    /// values.
    ///
    /// Essentially ... the slice [0,4,5,5], would expand to
    /// [0,0,0,0,1]; 0 to 4 have index 0, 4 to 5 have index 1, 5 to 5 would
    /// have index 2 but its empty!
    ///
    /// Then this index can be converted using the Scan2ImConverter.convert
    fn expand_mobility_iter(&self) -> impl Iterator<Item = u16> + '_ {
        let ims_iter = self
            .scan_offsets
            .windows(2)
            .enumerate()
            .filter_map(|(i, w)| {
                assert!(w[1] >= w[0], "Scan offsets should be sorted");

                let num = w[1] - w[0];
                if num == 0 {
                    return None;
                }
                let lo = w[0];
                let hi = w[1];

                let scan_index: u16 = i
                    .try_into()
                    .expect("Frames should never have more than 65535 scans");

                Some((scan_index, lo, hi))
            })
            .flat_map(|(im, lo, hi)| (lo..hi).map(move |_| im));
        ims_iter
    }

    /// Iterate over all peaks in the frame.
    ///
    /// Note: this does not apply any intensity correction
    pub fn iter_peaks(&self) -> impl Iterator<Item = FramePeak> + '_ {
        self.expand_mobility_iter().enumerate().map(
            |(peak_index, scan_index)| FramePeak {
                scan_index,
                tof_index: self.tof_indices[peak_index],
                intensity: self.intensities[peak_index],
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FramePeak {
    pub scan_index: u16,
    pub tof_index: u32,
    pub intensity: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrectedFramePeak {
    pub scan_index: u16,
    pub tof_index: u32,
    pub corrected_intensity: f64,
}

impl Frame {
    pub fn get_corrected_intensity(&self, index: usize) -> f64 {
        self.meta.intensity_correction_factor
            * self.peaks.intensities[index] as f64
    }

    /// Iterate over the intensity-corrected peaks.
    ///
    /// The coccected intensity takes into account the
    /// injection time of the frame.
    pub fn iter_corrected_peaks(
        &self,
    ) -> impl Iterator<Item = CorrectedFramePeak> + '_ {
        let factor = self.meta.intensity_correction_factor;
        self.peaks.iter_peaks().map(move |peak| CorrectedFramePeak {
            scan_index: peak.scan_index,
            tof_index: peak.tof_index,
            corrected_intensity: factor * peak.intensity as f64,
        })
    }
}

/// The MS level used.
#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum MSLevel {
    MS1,
    MS2,
    /// Default value.
    #[default]
    Unknown,
}

impl MSLevel {
    pub fn read_from_msms_type(msms_type: u8) -> MSLevel {
        match msms_type {
            0 => MSLevel::MS1,
            8 => MSLevel::MS2,
            9 => MSLevel::MS2,
            _ => MSLevel::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_frame_peaks_iter() {
        let frame_peaks = FramePeaks {
            scan_offsets: vec![0, 5, 11],
            // 0-5 -> scan 0
            // 5-11 -> scan 1
            tof_indices: (10..21).collect(),
            intensities: (10..21).map(|x| (x + 1) * 2).collect(),
        };
        let peaks: Vec<FramePeak> = frame_peaks.iter_peaks().collect();
        assert_eq!(peaks.len(), 11);
        assert_eq!(peaks[0].scan_index, 0);
        assert_eq!(peaks[0].tof_index, 10);
        assert_eq!(peaks[0].intensity, 22);
        assert_eq!(peaks.last().unwrap().scan_index, 1);
        assert_eq!(peaks.last().unwrap().tof_index, 20);
        assert_eq!(peaks.last().unwrap().intensity, 42);
    }

    fn test_frame_peaks_iter2() {
        let frame_peaks = FramePeaks {
            scan_offsets: vec![0, 0, 0, 5, 11, 11, 11, 11],
            // 0-5 -> scan 2
            // 5-11 -> scan 3
            // Trailing 11,11,11 should be ignored, since it points to emty scans.
            tof_indices: (10..21).collect(),
            intensities: (10..21).map(|x| (x + 1) * 2).collect(),
        };

        let peaks: Vec<FramePeak> = frame_peaks.iter_peaks().collect();
        assert_eq!(peaks.len(), 11);
        assert_eq!(peaks[0].scan_index, 2);
        assert_eq!(peaks[0].tof_index, 10);
        assert_eq!(peaks[0].intensity, 22);
        assert_eq!(peaks.last().unwrap().scan_index, 3);
        assert_eq!(peaks.last().unwrap().tof_index, 20);
        assert_eq!(peaks.last().unwrap().intensity, 42);
    }
}
