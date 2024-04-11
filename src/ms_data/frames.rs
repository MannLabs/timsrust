use crate::AcquisitionType;

/// A frame with all unprocessed data as it was acquired.
#[derive(Debug, PartialEq, Default)]
pub struct Frame {
    pub scan_offsets: Vec<usize>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub index: usize,
    pub rt: f64,
    pub frame_type: FrameType,
    pub acquisition: AcquisitionType,
}

/// The kind of frame, determined by acquisition.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FrameType {
    MS1,
    MS2,
    Unknown,
}

impl Default for FrameType {
    fn default() -> Self {
        Self::Unknown
    }
}
