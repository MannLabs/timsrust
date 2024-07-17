use std::path::PathBuf;

use crate::domain_converters::{
    Frame2RtConverter, Scan2ImConverter, Tof2MzConverter,
};

/// Metadata from a single run.
#[derive(Clone, Debug, Default, PartialEq)]

pub struct Metadata {
    pub path: PathBuf,
    pub rt_converter: Frame2RtConverter,
    pub im_converter: Scan2ImConverter,
    pub mz_converter: Tof2MzConverter,
    pub compression_type: u8,
}
