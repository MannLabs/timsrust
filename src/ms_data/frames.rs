use super::{AcquisitionType, QuadrupoleSettings};
use std::sync::Arc;

/// A frame with all unprocessed data as it was acquired.
#[derive(Debug, PartialEq, Default, Clone)]
pub struct Frame {
    pub scan_offsets: Vec<usize>,
    pub tof_indices: Vec<u32>,
    pub intensities: Vec<u32>,
    pub index: usize,
    pub rt: f64,
    pub acquisition_type: AcquisitionType,
    pub ms_level: MSLevel,
    pub quadrupole_settings: Arc<QuadrupoleSettings>,
    pub intensity_correction_factor: f64,
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
