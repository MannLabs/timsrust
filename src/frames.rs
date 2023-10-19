use crate::acquisition::AcquisitionType;

#[derive(Debug, PartialEq, Default)]
pub struct Frame {
    pub scan_offsets: Vec<u64>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub index: usize,
    pub rt: f64,
    pub frame_type: FrameType,
}

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
