use crate::converters::{ConvertableIndex, Tof2MzConverter};

#[derive(Debug, PartialEq, Default)]
pub struct Frame {
    pub scan_offsets: Vec<u64>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub index: usize,
    pub rt: f64,
    pub frame_type: FrameType,
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
pub struct FrameMSMSWindow {
    pub scan_offsets: Vec<u64>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub frame_index: usize,
    pub rt: f64,
    pub window_group: usize,
    pub scan_start: usize,
}

#[derive(Debug, PartialEq)]
pub enum FrameType {
    MS1,
    MS2DDA,
    MS2DIA,
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
        self.tof_indices.iter().map(|&x| mz_reader.convert(x)).collect()
    }
}