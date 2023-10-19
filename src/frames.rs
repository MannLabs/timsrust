use crate::acquisition::AcquisitionType;

/// A frame with all unprocessed data as it was acquired.

use std::fmt::Display;

use crate::converters::{ConvertableIndex, Tof2MzConverter};

/// A Frame is a group of scans (subset of frame along IMS).
/// That are associated to the same collection of ions in the
/// tims funnel.
///
/// * `scan_offsets` - The scan offsets for the frame.
///     This is a vec of length N, where N is the number
///     of scans in the frame.
/// * `tof_indices` - The TOF indices for the frame.
///     These values are related to the m/z values of the peaks.
///     This is a vec of length P, where P is the number of peaks
///     in the frame.
/// * `intensities` - The intensities for the frame.
///     This is a vec of the same size as the tof_indices, therefore
///     each peak is defined by a combination of the tof_index and
///     the intensity.
/// * `index` - The index of the frame. This is the index of the
///     frame in the "parent" tdf file.
/// * `rt` - The retention time of the frame, in seconds.
/// * `frame_type` - The type of frame. This is an enum with the
///     following possible values:
///     * `MS1` - The frame is an MS1 frame.
///     * `MS2DDA` - The frame is an MS2 DDA frame.
///     * `MS2DIA` - The frame is an MS2 DIA frame.
///     * `Unknown` - The frame type is unknown.
///
#[derive(Debug, PartialEq, Default)]
pub struct Frame {
    pub scan_offsets: Vec<u64>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub index: usize,
    pub rt: f64,
    pub frame_type: FrameType,
}


impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(Frame: {}, RT: {}, n_peaks={})",
            self.index,
            self.rt,
            self.tof_indices.len(),
        )
    }
}

/// A Frame window is a group of scans (subset of frame along IMS)
/// that are associated with a single isolation window.
///
/// * `scan_offsets` - The scan offsets for the frame window.
/// * `tof_indices` - The TOF indices for the frame window.
///     These values are correlated with the m/z values of the peaks
///     but are kept as u32 to save space (converting them makes them f64).
/// * `intensities` - The intensities for the frame window.
/// * `frame_index` - The index of the frame.
/// * `rt` - The retention time of the frame.
/// * `window_group` - The window group of the frame window.
///     This is an integer value that originally comes from the "WindowGroup"
///     in the `dia_frame_msms_table` of the TDF file.
/// * `scan_start` - The index of the first scan in the frame window.
///     This is used to know what offset to use when converting ims
///     values from the scan offsets (this number should converted
///     to mobility and all other mobility values within this frame window
///     should have that added).
#[derive(Debug, PartialEq)]
pub struct FrameMSMSWindow {
    pub scan_offsets: Vec<u64>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub frame_index: usize,
    pub rt: f64,
    pub window_group: usize,
    pub scan_start: usize,
}

impl Display for FrameMSMSWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(Frame: {}, Window: {}, RT:{}, npeaks={})",
            self.frame_index,
            self.window_group,
            self.rt,
            self.tof_indices.len(),
        )
    }
}

/// The kind of frame, determined by acquisition.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FrameType {
    MS1,
    MS2(AcquisitionType),
    Unknown,
}

impl Default for FrameType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Frame {
    /// Resolves the m/z values for the frame.
    ///
    /// * `mz_reader` - The TOF to m/z converter.
    ///
    /// # Returns
    /// A vec with the m/z values for the frame (as f64).
    pub fn resolve_mz_values(&self, mz_reader: &Tof2MzConverter) -> Vec<f64> {
        self.tof_indices
            .iter()
            .map(|&x| mz_reader.convert(x))
            .collect()
    }
}
