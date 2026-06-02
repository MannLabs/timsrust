use std::{collections::HashMap, fmt::Debug, str::FromStr, sync::Arc};

use timsrust_core::{AcquisitionType, FrameIndex, Im, MSLevel, Mz, Rt};

use crate::{Frame2RtConverter, TdfFrameReader};

use super::{
    TDFPathLike,
    file_readers::sql_reader::{
        ReadableSqlHashMap, SqlReader, SqlReaderError, metadata::SqlMetadata,
    },
};

/// Metadata from a single run.
#[derive(Clone, Debug)]
pub struct Metadata {
    rt_converter: Arc<Frame2RtConverter>,
    compression_type: u8,
    acquisition_type: AcquisitionType,
    lower_rt: Rt,
    upper_rt: Rt,
    lower_im: Im,
    upper_im: Im,
    lower_mz: Mz,
    upper_mz: Mz,
    path: String,
    max_peaks_per_scan: usize,
}

const OTOF_CONTROL: &str = "Bruker otofControl";

impl Metadata {
    pub fn new(path: impl TDFPathLike) -> Result<Self, MetadataReaderError> {
        let tdf_sql_reader = SqlReader::open(&path)?;
        let sql_metadata: HashMap<String, String> =
            SqlMetadata::from_sql_reader(&tdf_sql_reader)?;
        let compression_type =
            parse_value(&sql_metadata, "TimsCompressionType")?;
        let max_peaks_per_scan =
            parse_value(&sql_metadata, "MaxNumPeaksPerScan")?;
        let frame_reader = TdfFrameReader::without_metadata(
            path.as_ref(),
            compression_type,
            max_peaks_per_scan,
        )
        .unwrap();
        let (mz_min, mz_max) = get_mz_bounds(&sql_metadata)?;
        let (im_min, im_max) = get_im_bounds(&sql_metadata)?;
        // let rt_values: Vec<f64> = tdf_sql_reader
        //     .sql_file()
        //     .read_column_from_table("Time", "Frames")?;
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
        let rt_min = rt_values
            .values()
            .cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let rt_max = rt_values
            .values()
            .cloned()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let mut acquisition_type = AcquisitionType::Unknown;
        for index in frame_reader.iter_indices() {
            let frame =
                frame_reader.get_partial_frame_without_ions(index).unwrap();
            let frame_info = frame.info();
            if frame_info.ms_level() != MSLevel::MS1 {
                acquisition_type = frame_info.acquisition_type();
                break;
            }
        }
        let metadata = Metadata {
            rt_converter: Arc::new(Frame2RtConverter::from_values(rt_values)),
            lower_rt: rt_min,
            upper_rt: rt_max,
            lower_im: im_min.into(),
            upper_im: im_max.into(),
            lower_mz: mz_min.into(),
            upper_mz: mz_max.into(),
            compression_type,
            path: path.as_ref().to_string(),
            max_peaks_per_scan,
            acquisition_type,
        };
        Ok(metadata)
    }

    pub fn rt_converter(&self) -> &Arc<Frame2RtConverter> {
        &self.rt_converter
    }

    pub fn max_peaks_per_scan(&self) -> usize {
        self.max_peaks_per_scan
    }

    pub fn compression_type(&self) -> u8 {
        self.compression_type
    }

    pub fn acquisition_type(&self) -> AcquisitionType {
        self.acquisition_type
    }

    pub fn lower_rt(&self) -> Rt {
        self.lower_rt
    }

    pub fn upper_rt(&self) -> Rt {
        self.upper_rt
    }

    pub fn lower_im(&self) -> Im {
        self.lower_im
    }

    pub fn upper_im(&self) -> Im {
        self.upper_im
    }

    pub fn lower_mz(&self) -> Mz {
        self.lower_mz
    }

    pub fn upper_mz(&self) -> Mz {
        self.upper_mz
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

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

fn get_im_bounds(
    sql_metadata: &HashMap<String, String>,
) -> Result<(f64, f64), MetadataReaderError> {
    let im_min: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeLower")?;
    let im_max: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeUpper")?;
    Ok((im_min, im_max))
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

#[allow(private_interfaces)]
#[derive(Debug, thiserror::Error)]
pub enum MetadataReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Key not parsable: {0}")]
    ParseError(String),
}
