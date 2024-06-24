use std::path::PathBuf;

use crate::domain_converters::{
    Frame2RtConverter, Scan2ImConverter, Tof2MzConverter,
};

use super::AcquisitionType;

/// Metadata from a single run
#[derive(Debug, Clone)]
pub struct Metadata {
    pub path: PathBuf,
    pub acquisition_type: AcquisitionType,
    pub rt_converter: Frame2RtConverter,
    pub im_converter: Scan2ImConverter,
    pub mz_converter: Tof2MzConverter,
}
