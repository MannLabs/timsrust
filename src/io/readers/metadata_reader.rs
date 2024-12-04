use std::{collections::HashMap, fmt::Debug, str::FromStr};

use crate::{
    domain_converters::{Frame2RtConverter, Scan2ImConverter, Tof2MzConverter},
    ms_data::Metadata,
};

use super::{
    file_readers::sql_reader::{
        metadata::SqlMetadata, ReadableSqlHashMap, SqlReader, SqlReaderError,
    },
    TimsTofPathLike,
};

const OTOF_CONTROL: &str = "Bruker otofControl";

pub struct MetadataReader;

impl MetadataReader {
    pub fn new(
        path: impl TimsTofPathLike,
    ) -> Result<Metadata, MetadataReaderError> {
        let tdf_sql_reader = SqlReader::open(path)?;
        let sql_metadata: HashMap<String, String> =
            SqlMetadata::from_sql_reader(&tdf_sql_reader)?;
        let compression_type =
            parse_value(&sql_metadata, "TimsCompressionType")?;
        let (mz_min, mz_max) = get_mz_bounds(&sql_metadata)?;
        let (im_min, im_max) = get_im_bounds(&sql_metadata)?;
        let rt_values: Vec<f64> =
            tdf_sql_reader.read_column_from_table("Time", "Frames")?;
        let rt_min = rt_values
            .iter()
            .filter(|&&v| !v.is_nan()) // Filter out NaN values
            .cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let rt_max = rt_values
            .iter()
            .filter(|&&v| !v.is_nan()) // Filter out NaN values
            .cloned()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let metadata = Metadata {
            rt_converter: Frame2RtConverter::from_values(rt_values),
            im_converter: get_im_converter(&sql_metadata, &tdf_sql_reader)?,
            mz_converter: get_mz_converter(&sql_metadata)?,
            lower_rt: rt_min,
            upper_rt: rt_max,
            lower_im: im_min,
            upper_im: im_max,
            lower_mz: mz_min,
            upper_mz: mz_max,
            compression_type,
        };
        Ok(metadata)
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

fn get_mz_converter(
    sql_metadata: &HashMap<String, String>,
) -> Result<Tof2MzConverter, MetadataReaderError> {
    let (mz_min, mz_max) = get_mz_bounds(sql_metadata)?;
    let tof_max_index: u32 = parse_value(sql_metadata, "DigitizerNumSamples")?;
    Ok(Tof2MzConverter::from_boundaries(
        mz_min,
        mz_max,
        tof_max_index,
    ))
}

fn get_im_bounds(
    sql_metadata: &HashMap<String, String>,
) -> Result<(f64, f64), MetadataReaderError> {
    let im_min: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeLower")?;
    let im_max: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeUpper")?;
    Ok((im_min, im_max))
}

fn get_im_converter(
    sql_metadata: &HashMap<String, String>,
    tdf_sql_reader: &SqlReader,
) -> Result<Scan2ImConverter, MetadataReaderError> {
    let scan_counts: Vec<u32> =
        tdf_sql_reader.read_column_from_table("NumScans", "Frames")?;
    let scan_max_index = *scan_counts
        .iter()
        .max()
        .expect("SqlReader cannot return empty vecs, so there is always a max scan index");
    let (im_min, im_max) = get_im_bounds(sql_metadata)?;
    Ok(Scan2ImConverter::from_boundaries(
        im_min,
        im_max,
        scan_max_index,
    ))
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

#[derive(Debug, thiserror::Error)]
pub enum MetadataReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Key not parsable: {0}")]
    ParseError(String),
}
