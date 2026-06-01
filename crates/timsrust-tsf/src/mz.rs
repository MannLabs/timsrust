use std::{collections::HashMap, str::FromStr};

use serde::Deserialize;
use timsrust_core::{
    Converter, Mz, TofIndex,
    io::formats::sql::SqlReader,
    utils::simple_error,
};

use crate::TSFPathLike;

#[derive(Deserialize)]
struct KvRow {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

const OTOF_CONTROL: &str = "Bruker otofControl";

#[derive(Clone, Debug, PartialEq)]
pub struct Tof2MzConverter {
    tof_intercept: f64,
    tof_slope: f64,
}

impl Tof2MzConverter {
    fn from_boundaries(mz_min: f64, mz_max: f64, tof_max_index: u32) -> Self {
        let tof_intercept: f64 = mz_min.sqrt();
        let tof_slope: f64 =
            (mz_max.sqrt() - tof_intercept) / tof_max_index as f64;
        Self {
            tof_intercept,
            tof_slope,
        }
    }

    pub fn new(path: impl TSFPathLike) -> Self {
        let timstof_path = path.to_timstof_path().unwrap();
        let reader = SqlReader::from(timstof_path.tsf().as_str()).unwrap();
        let hash_map: HashMap<String, String> = reader
            .from_table::<KvRow>("GlobalMetadata")
            .unwrap()
            .read_all()
            .unwrap()
            .into_iter()
            .map(|r| (r.key, r.value))
            .collect();
        let (mz_min, mz_max) = get_mz_bounds(&hash_map).unwrap();
        let tof_max_index: u32 =
            parse_value(&hash_map, "DigitizerNumSamples").unwrap();
        Self::from_boundaries(mz_min, mz_max, tof_max_index)
    }
}

impl Converter<TofIndex, Mz> for Tof2MzConverter {
    fn convert(&self, value: TofIndex) -> Mz {
        let value = u32::from(value) as f64;
        let mz = self.tof_intercept + self.tof_slope * value;
        let result = mz * mz;
        Mz::from(result)
    }
}

impl Converter<Mz, TofIndex> for Tof2MzConverter {
    fn convert(&self, value: Mz) -> TofIndex {
        let value = f64::from(value);
        let result = (value.sqrt() - self.tof_intercept) / self.tof_slope;
        TofIndex::try_from(result as u32)
            .expect("TofIndex conversion out of bounds")
    }
}

fn get_mz_bounds(
    sql_metadata: &HashMap<String, String>,
) -> Result<(f64, f64), Tof2MzConverterError> {
    let software = sql_metadata.get("AcquisitionSoftware").unwrap();
    let mut mz_min: f64 = parse_value(sql_metadata, "MzAcqRangeLower")?;
    let mut mz_max: f64 = parse_value(sql_metadata, "MzAcqRangeUpper")?;
    if software == OTOF_CONTROL {
        mz_min -= 5.0;
        mz_max += 5.0;
    }
    Ok((mz_min, mz_max))
}

fn parse_value<T: FromStr>(
    hash_map: &HashMap<String, String>,
    key: &str,
) -> Result<T, Tof2MzConverterError> {
    let value: T = hash_map
        .get(key)
        .unwrap()
        .parse()
        .map_err(|_| Tof2MzConverterError())?;
    Ok(value)
}

simple_error!(Tof2MzConverterError);
