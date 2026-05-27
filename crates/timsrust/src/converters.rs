use timsrust_core::{Converter, FrameIndex, Im, Mz, Rt, ScanIndex, TofIndex};

use crate::{TimsTofPath, timstof::TimsTofFileType};

// pub type Spectrum = timsrust_core::Spectrum<MzConverter>;

#[derive(Debug, Clone)]
pub enum MzConverter {
    #[cfg(feature = "sdk")]
    Sdk(timsrust_sdk::WrappedTof2MzConverterSDK),
    #[cfg(feature = "bps")]
    Bps(timsrust_bruker::bps::Tof2MzConverter),
    Bit(timsrust_core::BitConverter),
    Tdf(timsrust_tdf::Tof2MzConverter),
    MiniTdf,
}

impl MzConverter {
    pub fn new(path: impl AsRef<str>) -> Option<Self> {
        let timstof = TimsTofPath::new(path.as_ref()).ok()?;
        #[allow(unreachable_code)]
        match timstof.file_type() {
            TimsTofFileType::Tdf(tdf_path) => {
                #[cfg(feature = "sdk")]
                return Some(MzConverter::Sdk(
                    timsrust_sdk::WrappedTof2MzConverterSDK::new(
                        tdf_path.tdf().as_ref(),
                    )?,
                ));
                #[cfg(feature = "bps")]
                return Some(MzConverter::Bps(
                    timsrust_bruker::bps::Tof2MzConverter::from_tdf(
                        tdf_path.tdf().as_ref(),
                    )?,
                ));
                Some(MzConverter::Tdf(timsrust_tdf::Tof2MzConverter::new(
                    tdf_path.tdf().as_ref(),
                )))
            },
            TimsTofFileType::MiniTdf(_) => Some(MzConverter::MiniTdf),
            #[cfg(feature = "bps")]
            TimsTofFileType::Parquet(_) => {
                Some(MzConverter::Bit(timsrust_core::BitConverter()))
            },
        }
    }
}

impl Converter<TofIndex, Mz> for MzConverter {
    fn convert(&self, tof_index: TofIndex) -> Mz {
        match self {
            #[cfg(feature = "bps")]
            Self::Bps(converter) => converter.convert(tof_index),
            #[cfg(feature = "sdk")]
            Self::Sdk(converter) => converter.convert(tof_index),
            Self::Bit(converter) => converter.convert(tof_index),
            Self::Tdf(converter) => converter.convert(tof_index),
            Self::MiniTdf => timsrust_core::BitConverter().convert(tof_index),
        }
    }
}

impl Converter<Mz, TofIndex> for MzConverter {
    fn convert(&self, mz: Mz) -> TofIndex {
        match self {
            #[cfg(feature = "bps")]
            Self::Bps(converter) => converter.convert(mz),
            #[cfg(feature = "sdk")]
            Self::Sdk(converter) => converter.convert(mz),
            Self::Bit(converter) => converter.convert(mz),
            Self::Tdf(converter) => converter.convert(mz),
            Self::MiniTdf => timsrust_core::BitConverter().convert(mz),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ImConverter {
    #[cfg(feature = "sdk")]
    Sdk(timsrust_sdk::WrappedScan2ImConverterSDK),
    #[cfg(feature = "bps")]
    Bps(timsrust_bruker::bps::Scan2ImConverter),
    Bit(timsrust_core::BitConverter),
    Tdf(timsrust_tdf::Scan2ImConverter),
    MiniTdf,
}

impl ImConverter {
    pub fn new(path: impl AsRef<str>) -> Option<Self> {
        let timstof = TimsTofPath::new(path.as_ref()).ok()?;
        #[allow(unreachable_code)]
        match timstof.file_type() {
            TimsTofFileType::Tdf(tdf_path) => {
                #[cfg(feature = "sdk")]
                return Some(ImConverter::Sdk(
                    timsrust_sdk::WrappedScan2ImConverterSDK::new(
                        tdf_path.tdf().as_ref(),
                    )?,
                ));
                #[cfg(feature = "bps")]
                return Some(ImConverter::Bps(
                    timsrust_bruker::bps::Scan2ImConverter::from_tdf(
                        tdf_path.tdf().as_ref(),
                    )?,
                ));
                Some(ImConverter::Tdf(timsrust_tdf::Scan2ImConverter::new(
                    tdf_path.tdf().as_ref(),
                )))
            },
            TimsTofFileType::MiniTdf(_) => Some(ImConverter::MiniTdf),
            #[cfg(feature = "bps")]
            TimsTofFileType::Parquet(_) => {
                Some(ImConverter::Bit(timsrust_core::BitConverter()))
            },
        }
    }
}

impl Converter<ScanIndex, Im> for ImConverter {
    fn convert(&self, scan_index: ScanIndex) -> Im {
        match self {
            #[cfg(feature = "bps")]
            Self::Bps(converter) => converter.convert(scan_index),
            #[cfg(feature = "sdk")]
            Self::Sdk(converter) => converter.convert(scan_index),
            Self::Bit(converter) => converter.convert(scan_index),
            Self::Tdf(converter) => converter.convert(scan_index),
            Self::MiniTdf => timsrust_core::BitConverter().convert(scan_index),
        }
    }
}

impl Converter<Im, ScanIndex> for ImConverter {
    fn convert(&self, im: Im) -> ScanIndex {
        match self {
            #[cfg(feature = "bps")]
            Self::Bps(converter) => converter.convert(im),
            #[cfg(feature = "sdk")]
            Self::Sdk(converter) => converter.convert(im),
            Self::Bit(converter) => converter.convert(im),
            Self::Tdf(converter) => converter.convert(im),
            Self::MiniTdf => timsrust_core::BitConverter().convert(im),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RtConverter {
    Bit(timsrust_core::BitConverter),
    Tdf(timsrust_tdf::Frame2RtConverter),
    MiniTdf,
}

impl RtConverter {
    pub fn new(path: impl AsRef<str>) -> Option<Self> {
        let timstof = TimsTofPath::new(path.as_ref()).ok()?;
        match timstof.file_type() {
            TimsTofFileType::Tdf(tdf_path) => Some(RtConverter::Tdf(
                timsrust_tdf::Frame2RtConverter::new(tdf_path.tdf().as_ref()),
            )),
            TimsTofFileType::MiniTdf(_) => Some(RtConverter::MiniTdf),
            #[cfg(feature = "bps")]
            TimsTofFileType::Parquet(_) => {
                Some(RtConverter::Bit(timsrust_core::BitConverter()))
            },
        }
    }
}

impl Converter<FrameIndex, Rt> for RtConverter {
    fn convert(&self, frame_index: FrameIndex) -> Rt {
        match self {
            Self::Bit(converter) => converter.convert(frame_index),
            Self::Tdf(converter) => converter.convert(frame_index),
            Self::MiniTdf => timsrust_core::BitConverter().convert(frame_index),
        }
    }
}

impl Converter<Rt, FrameIndex> for RtConverter {
    fn convert(&self, rt: Rt) -> FrameIndex {
        match self {
            Self::Bit(converter) => converter.convert(rt),
            Self::Tdf(converter) => converter.convert(rt),
            Self::MiniTdf => timsrust_core::BitConverter().convert(rt),
        }
    }
}
