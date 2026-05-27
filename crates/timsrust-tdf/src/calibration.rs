use std::{collections::HashMap, str::FromStr};

use timsrust_core::{Converter, FrameIndex, Im, Mz, Rt, ScanIndex, TofIndex};

use crate::{MetadataReaderError, TdfFrameReader};

#[derive(Clone, Debug, PartialEq)]
pub struct UncalibratedTof2MzConverter {
    tof_intercept: f64,
    tof_slope: f64,
}

impl UncalibratedTof2MzConverter {
    fn from_boundaries(mz_min: f64, mz_max: f64, tof_max_index: u32) -> Self {
        let tof_intercept: f64 = mz_min.sqrt();
        let tof_slope: f64 =
            (mz_max.sqrt() - tof_intercept) / tof_max_index as f64;
        Self {
            tof_intercept,
            tof_slope,
        }
    }

    fn new(path: &str) -> Self {
        use crate::file_readers::sql_reader::{
            ReadableSqlHashMap, SqlReader, metadata::SqlMetadata,
        };

        let tdf_sql_reader = SqlReader::open(path).unwrap();
        let sql_metadata: HashMap<String, String> =
            SqlMetadata::from_sql_reader(&tdf_sql_reader).unwrap();
        let (mz_min, mz_max) = get_mz_bounds(&sql_metadata).unwrap();
        let tof_max_index: u32 =
            parse_value(&sql_metadata, "DigitizerNumSamples").unwrap();
        UncalibratedTof2MzConverter::from_boundaries(
            mz_min,
            mz_max,
            tof_max_index,
        )
    }
}

impl Converter<TofIndex, Mz> for UncalibratedTof2MzConverter {
    fn convert(&self, value: TofIndex) -> Mz {
        let value = u32::from(value) as f64;
        let mz = self.tof_intercept + self.tof_slope * value;
        let result = mz * mz;
        Mz::from(result)
    }
}

impl Converter<Mz, TofIndex> for UncalibratedTof2MzConverter {
    fn convert(&self, value: Mz) -> TofIndex {
        let value = f64::from(value);
        let result = (value.sqrt() - self.tof_intercept) / self.tof_slope;
        TofIndex::try_from(result as u32)
            .expect("TofIndex conversion out of bounds")
    }
}

#[derive(Clone, Debug)]
pub enum Tof2MzConverter {
    Uncalibrated(UncalibratedTof2MzConverter),
}

const OTOF_CONTROL: &str = "Bruker otofControl";

fn get_mz_bounds(
    sql_metadata: &HashMap<String, String>,
) -> Result<(f64, f64), MetadataReaderError> {
    let software = sql_metadata.get("AcquisitionSoftware").ok_or(
        MetadataReaderError::KeyNotFound("AcquisitionSoftware".to_string()),
    )?;
    let mut mz_min: f64 = parse_value(sql_metadata, "MzAcqRangeLower")?;
    let mut mz_max: f64 = parse_value(sql_metadata, "MzAcqRangeUpper")?;
    if software == OTOF_CONTROL {
        mz_min -= 5.0;
        mz_max += 5.0;
    }
    Ok((mz_min, mz_max))
}

impl Tof2MzConverter {
    pub fn new(path: &str) -> Self {
        Self::Uncalibrated(UncalibratedTof2MzConverter::new(path))
    }
}

impl Converter<TofIndex, Mz> for Tof2MzConverter {
    fn convert(&self, value: TofIndex) -> Mz {
        match self {
            Tof2MzConverter::Uncalibrated(converter) => {
                converter.convert(value)
            },
        }
    }
}

impl Converter<Mz, TofIndex> for Tof2MzConverter {
    fn convert(&self, value: Mz) -> TofIndex {
        match self {
            Tof2MzConverter::Uncalibrated(converter) => {
                converter.convert(value)
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UncalibratedScan2ImConverter {
    scan_intercept: f64,
    scan_slope: f64,
}

impl UncalibratedScan2ImConverter {
    fn from_boundaries(im_min: f64, im_max: f64, scan_max_index: u32) -> Self {
        let scan_intercept: f64 = im_max.sqrt();
        let scan_slope: f64 =
            (im_min.sqrt() - scan_intercept) / scan_max_index as f64;
        Self {
            scan_intercept,
            scan_slope,
        }
    }

    fn new(path: &str) -> Self {
        use crate::file_readers::sql_reader::{
            ReadableSqlHashMap, ReadableSqlTable, SqlReader, frames::SqlFrame,
            metadata::SqlMetadata,
        };
        let tdf_sql_reader = SqlReader::open(path).unwrap();
        let sql_metadata: HashMap<String, String> =
            SqlMetadata::from_sql_reader(&tdf_sql_reader).unwrap();
        let sql_frames = SqlFrame::from_sql_reader(&tdf_sql_reader).unwrap();
        let scan_max_index = sql_frames
            .iter()
            .map(|f| f.scan_count as u32)
            .max()
            .expect("SqlReader cannot return empty vecs, so there is always a max scan index");
        let (im_min, im_max) = get_im_bounds(&sql_metadata).unwrap();
        Self::from_boundaries(im_min, im_max, scan_max_index)
    }
}

impl Converter<ScanIndex, Im> for UncalibratedScan2ImConverter {
    fn convert(&self, value: ScanIndex) -> Im {
        let value = f64::from(value);
        let im = self.scan_intercept + self.scan_slope * value;
        let result = im * im;
        Im::from(result)
    }
}

impl Converter<Im, ScanIndex> for UncalibratedScan2ImConverter {
    fn convert(&self, value: Im) -> ScanIndex {
        let value = f64::from(value);
        let result = (value.sqrt() - self.scan_intercept) / self.scan_slope;
        ScanIndex::try_from(result as u32)
            .expect("ScanIndex conversion out of bounds")
    }
}

#[derive(Clone, Debug)]
pub enum Scan2ImConverter {
    Uncalibrated(UncalibratedScan2ImConverter),
}

fn parse_value<T: FromStr>(
    hash_map: &HashMap<String, String>,
    key: &str,
) -> Result<T, MetadataReaderError> {
    let value: T = hash_map
        .get(key)
        .ok_or(MetadataReaderError::KeyNotFound(key.to_string()))?
        .parse()
        .map_err(|_| MetadataReaderError::ParseError(key.to_string()))?;
    Ok(value)
}

fn get_im_bounds(
    sql_metadata: &HashMap<String, String>,
) -> Result<(f64, f64), MetadataReaderError> {
    let im_min: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeLower")?;
    let im_max: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeUpper")?;
    Ok((im_min, im_max))
}

impl Scan2ImConverter {
    pub fn new(path: &str) -> Self {
        Self::Uncalibrated(UncalibratedScan2ImConverter::new(path))
    }
}

impl Converter<ScanIndex, Im> for Scan2ImConverter {
    fn convert(&self, value: ScanIndex) -> Im {
        match self {
            Scan2ImConverter::Uncalibrated(converter) => {
                converter.convert(value)
            },
        }
    }
}

impl Converter<Im, ScanIndex> for Scan2ImConverter {
    fn convert(&self, value: Im) -> ScanIndex {
        match self {
            Scan2ImConverter::Uncalibrated(converter) => {
                converter.convert(value)
            },
        }
    }
}

/// A converter from Frame -> retention time.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Frame2RtConverter {
    forward: HashMap<FrameIndex, Rt>,
    reverse: HashMap<Rt, FrameIndex>,
}

impl Frame2RtConverter {
    pub fn new(path: &str) -> Self {
        let frame_reader = TdfFrameReader::new(path).unwrap();
        let rt_values = frame_reader
            .iter_indices()
            .map(|index| {
                let frame =
                    frame_reader.get_partial_frame_without_ions(index).unwrap();
                (
                    FrameIndex::try_from(frame.info().index() as u32)
                        .expect("FrameIndex conversion out of bounds"),
                    Rt::from(frame.info().rt_in_seconds()),
                )
            })
            .collect::<HashMap<FrameIndex, Rt>>();
        Self::from_values(rt_values)
    }

    pub fn from_values(forward: HashMap<FrameIndex, Rt>) -> Self {
        let reverse = forward.iter().map(|(k, v)| (*v, *k)).collect();
        Self { forward, reverse }
    }
}

impl Converter<FrameIndex, Rt> for Frame2RtConverter {
    fn convert(&self, value: FrameIndex) -> Rt {
        *self
            .forward
            .get(&value)
            .expect("FrameIndex not found in converter")
    }
}

impl Converter<Rt, FrameIndex> for Frame2RtConverter {
    fn convert(&self, value: Rt) -> FrameIndex {
        *self.reverse.get(&value).expect("Rt not found in converter")
    }
}
