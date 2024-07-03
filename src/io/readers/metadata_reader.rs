use std::{collections::HashMap, path::Path};

use crate::{
    domain_converters::{Frame2RtConverter, Scan2ImConverter, Tof2MzConverter},
    ms_data::Metadata,
};

use super::file_readers::sql_reader::{
    metadata::SqlMetadata, ReadableSqlHashMap, SqlReader,
};

const OTOF_CONTROL: &str = "Bruker otofControl";

pub struct MetadataReader;

impl MetadataReader {
    pub fn new(path: impl AsRef<Path>) -> Metadata {
        let sql_path = path.as_ref();
        let tdf_sql_reader = SqlReader::open(&sql_path).unwrap();
        let sql_metadata: HashMap<String, String> =
            SqlMetadata::from_sql_reader(&tdf_sql_reader).unwrap();
        Metadata {
            path: path.as_ref().to_path_buf(),
            rt_converter: get_rt_converter(&tdf_sql_reader),
            im_converter: get_im_converter(&sql_metadata, &tdf_sql_reader),
            mz_converter: get_mz_converter(&sql_metadata),
        }
    }
}

fn get_rt_converter(tdf_sql_reader: &SqlReader) -> Frame2RtConverter {
    let rt_values: Vec<f64> = tdf_sql_reader
        .read_column_from_table("Time", "Frames")
        .unwrap();
    Frame2RtConverter::from_values(rt_values)
}

fn get_mz_converter(sql_metadata: &HashMap<String, String>) -> Tof2MzConverter {
    let software = sql_metadata.get("AcquisitionSoftware").unwrap();
    let tof_max_index: u32 = sql_metadata
        .get("DigitizerNumSamples")
        .unwrap()
        .parse()
        .unwrap();
    let mut mz_min: f64 = sql_metadata
        .get("MzAcqRangeLower")
        .unwrap()
        .parse()
        .unwrap();
    let mut mz_max: f64 = sql_metadata
        .get("MzAcqRangeUpper")
        .unwrap()
        .parse()
        .unwrap();
    if software == OTOF_CONTROL {
        mz_min -= 5.0;
        mz_max += 5.0;
    }
    Tof2MzConverter::from_boundaries(mz_min, mz_max, tof_max_index)
}

fn get_im_converter(
    sql_metadata: &HashMap<String, String>,
    tdf_sql_reader: &SqlReader,
) -> Scan2ImConverter {
    let scan_counts: Vec<u32> = tdf_sql_reader
        .read_column_from_table("NumScans", "Frames")
        .unwrap();
    let scan_max_index = *scan_counts.iter().max().unwrap();
    // let scan_max_index = 927;
    let im_min: f64 = sql_metadata
        .get("OneOverK0AcqRangeLower")
        .unwrap()
        .parse()
        .unwrap();
    let im_max: f64 = sql_metadata
        .get("OneOverK0AcqRangeUpper")
        .unwrap()
        .parse()
        .unwrap();
    Scan2ImConverter::from_boundaries(im_min, im_max, scan_max_index)
}
