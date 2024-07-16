use std::{collections::HashMap, fmt::Debug, path::Path, str::FromStr};

use crate::{
    domain_converters::{Frame2RtConverter, Scan2ImConverter, Tof2MzConverter},
    ms_data::Metadata,
};

use super::file_readers::sql_reader::{
    metadata::SqlMetadata, ReadableSqlHashMap, SqlError, SqlReader,
};

const OTOF_CONTROL: &str = "Bruker otofControl";

pub struct MetadataReader;

impl MetadataReader {
    pub fn new(
        path: impl AsRef<Path>,
    ) -> Result<Metadata, MetadataReaderError> {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(&sql_path)?;
        let sql_metadata: HashMap<String, String> =
            SqlMetadata::from_sql_reader(&tdf_sql_reader)?;
        let compression_type =
            parse_value(&sql_metadata, "TimsCompressionType")?;
        let metadata = Metadata {
            path: path.as_ref().to_path_buf(),
            rt_converter: get_rt_converter(&tdf_sql_reader)?,
            im_converter: get_im_converter(&sql_metadata, &tdf_sql_reader)?,
            mz_converter: get_mz_converter(&sql_metadata)?,
            compression_type,
        };
        Ok(metadata)
    }
}

fn get_rt_converter(
    tdf_sql_reader: &SqlReader,
) -> Result<Frame2RtConverter, MetadataReaderError> {
    let rt_values: Vec<f64> =
        tdf_sql_reader.read_column_from_table("Time", "Frames")?;
    Ok(Frame2RtConverter::from_values(rt_values))
}

fn get_mz_converter(
    sql_metadata: &HashMap<String, String>,
) -> Result<Tof2MzConverter, MetadataReaderError> {
    let software = sql_metadata.get("AcquisitionSoftware").ok_or(
        MetadataReaderError::KeyNotFound("AcquisitionSoftware".to_string()),
    )?;
    let tof_max_index: u32 = parse_value(sql_metadata, "DigitizerNumSamples")?;
    let mut mz_min: f64 = parse_value(sql_metadata, "MzAcqRangeLower")?;
    let mut mz_max: f64 = parse_value(sql_metadata, "MzAcqRangeUpper")?;
    if software == OTOF_CONTROL {
        mz_min -= 5.0;
        mz_max += 5.0;
    }
    Ok(Tof2MzConverter::from_boundaries(
        mz_min,
        mz_max,
        tof_max_index,
    ))
}

fn get_im_converter(
    sql_metadata: &HashMap<String, String>,
    tdf_sql_reader: &SqlReader,
) -> Result<Scan2ImConverter, MetadataReaderError> {
    let scan_counts: Vec<u32> =
        tdf_sql_reader.read_column_from_table("NumScans", "Frames")?;
    let scan_max_index = *scan_counts.iter().max().unwrap(); // SqlReader cannot return empty vecs, so always succeeds
    let im_min: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeLower")?;
    let im_max: f64 = parse_value(sql_metadata, "OneOverK0AcqRangeUpper")?;
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
    // #[error("{0}")]
    // TdfBlobReaderError(#[from] TdfBlobReaderError),
    // #[error("{0}")]
    // FileNotFound(String),
    #[error("{0}")]
    SqlError(#[from] SqlError),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Key not parsable: {0}")]
    ParseError(String),
}
