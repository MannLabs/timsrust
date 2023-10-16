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
    pub fn resolve_mz_values(&self, mz_reader: &Tof2MzConverter) -> Vec<f64> {
        self.tof_indices.iter().map(|&x| mz_reader.convert(x)).collect()
    }
}