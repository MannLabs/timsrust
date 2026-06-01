use crate::{PressureCompensationStrategy, TimsData};
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use timsrust_core::io::formats::sql::SqlReader;
use timsrust_core::utils::thread::Synced;
use timsrust_core::{Converter, Im, Mz, ScanIndex, TofIndex};

#[derive(Clone, Debug, Default)]
pub struct Tof2MzConverterSDK {
    sdk_reader: Synced<TimsData>,
}

impl Tof2MzConverterSDK {
    pub fn new(path: &str) -> Self {
        let sdk_reader = TimsData::new(
            path.into(),
            false,
            PressureCompensationStrategy::default(),
        );
        Self {
            sdk_reader: Synced::from(sdk_reader),
        }
    }
}

impl Converter<TofIndex, Mz> for Tof2MzConverterSDK {
    fn convert(&self, value: TofIndex) -> Mz {
        let result = self
            .sdk_reader
            .with_lock(|sdk| {
                sdk.index_to_mz(1, vec![u32::from(value) as f64])[0]
            })
            .unwrap();
        Mz::from(result)
    }

    fn batch_convert(&self, values: &[TofIndex]) -> Vec<Mz>
    where
        TofIndex: Copy,
    {
        self.sdk_reader
            .with_lock(|sdk| {
                let mz_values = sdk.index_to_mz(
                    1,
                    values.iter().map(|&v| f64::from(v)).collect(),
                );
                mz_values.into_iter().map(Mz::from).collect()
            })
            .unwrap()
    }
}

impl Converter<Mz, TofIndex> for Tof2MzConverterSDK {
    fn convert(&self, value: Mz) -> TofIndex {
        let result = self
            .sdk_reader
            .with_lock(|sdk| sdk.mz_to_index(1, vec![f64::from(value)])[0])
            .unwrap();
        TofIndex::try_from(result as u32).unwrap()
    }
}

#[derive(Clone, Debug, Default)]
pub struct WrappedTof2MzConverterSDK {
    // sdk_reader: Tof2MzConverterSDK,
    // forward: HashMap<TofIndex, Mz>,
    forward: Vec<Mz>,
    reverse: HashMap<Mz, TofIndex>,
}

impl WrappedTof2MzConverterSDK {
    pub fn new(path: &str) -> Option<Self> {
        dbg!("Using SDK mz calibration");
        let sdk_reader = Tof2MzConverterSDK::new(path);
        let sql_metadata = get_metadata(path);
        let tof_max_index = parse_value(&sql_metadata, "DigitizerNumSamples")?;
        let mz_vals = sdk_reader.batch_convert(
            &(0..=tof_max_index)
                .map(|s| TofIndex::try_from(s).unwrap())
                .collect::<Vec<_>>(),
        );
        let tof_vals = (0..=tof_max_index)
            .map(|s| TofIndex::try_from(s).unwrap())
            .collect::<Vec<_>>();
        let forward: HashMap<TofIndex, Mz> =
            tof_vals.into_iter().zip(mz_vals.iter().cloned()).collect();
        let reverse = forward.into_iter().map(|(k, v)| (v, k)).collect();
        let result = Self {
            // sdk_reader,
            forward: mz_vals,
            reverse,
        };
        Some(result)
    }
}

impl Converter<TofIndex, Mz> for WrappedTof2MzConverterSDK {
    fn convert(&self, value: TofIndex) -> Mz {
        // *self
        //     .forward
        //     .get(&value)
        //     .expect("TofIndex not found in converter")
        self.forward[usize::from(value)]
    }
}

impl Converter<Mz, TofIndex> for WrappedTof2MzConverterSDK {
    fn convert(&self, value: Mz) -> TofIndex {
        *self.reverse.get(&value).expect("Mz not found in converter")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Scan2ImConverterSDK {
    sdk_reader: Synced<TimsData>,
}

impl Scan2ImConverterSDK {
    pub fn new(path: &str) -> Self {
        let sdk_reader = TimsData::new(
            path.into(),
            false,
            PressureCompensationStrategy::default(),
        );
        Self {
            sdk_reader: Synced::from(sdk_reader),
        }
    }
}

impl Converter<ScanIndex, Im> for Scan2ImConverterSDK {
    fn convert(&self, value: ScanIndex) -> Im {
        let result = self
            .sdk_reader
            .with_lock(|sdk| {
                sdk.scan_num_to_one_over_k0(1, vec![u32::from(value) as f64])[0]
            })
            .unwrap();
        Im::from(result)
    }

    fn batch_convert(&self, values: &[ScanIndex]) -> Vec<Im>
    where
        TofIndex: Copy,
    {
        self.sdk_reader
            .with_lock(|sdk| {
                let im_values = sdk.scan_num_to_one_over_k0(
                    1,
                    values.iter().map(|&v| f64::from(v)).collect(),
                );
                im_values.into_iter().map(Im::from).collect()
            })
            .unwrap()
    }
}

impl Converter<Im, ScanIndex> for Scan2ImConverterSDK {
    fn convert(&self, value: Im) -> ScanIndex {
        let result = self
            .sdk_reader
            .with_lock(|sdk| {
                sdk.one_over_k0_to_scan_number(1, vec![f64::from(value)])[0]
            })
            .unwrap();
        ScanIndex::try_from(result as u32).unwrap()
    }
}

#[derive(Clone, Debug, Default)]
pub struct WrappedScan2ImConverterSDK {
    // sdk_reader: Scan2ImConverterSDK,
    // forward: HashMap<ScanIndex, Im>,
    forward: Vec<Im>,
    reverse: HashMap<Im, ScanIndex>,
}

impl WrappedScan2ImConverterSDK {
    pub fn new(path: &str) -> Option<Self> {
        dbg!("Using SDK im calibration");
        let sdk_reader = Scan2ImConverterSDK::new(path);
        let tdf_path = std::path::Path::new(path);
        let scan_counts: Vec<u32> = SqlReader::from(tdf_path.to_str().unwrap())
            .ok()?
            .from_table::<FrameRow>("Frames")
            .ok()?
            .read_all()
            .ok()?
            .into_iter()
            .map(|f| f.num_scans)
            .collect();
        // let sql_metadata = get_metadata(path);
        let scan_max_index = *scan_counts
                .iter()
                .max()
                .expect("SqlReader cannot return empty vecs, so there is always a max scan index");
        let im_vals = sdk_reader.batch_convert(
            &(0..=scan_max_index)
                .map(|s| ScanIndex::try_from(s).unwrap())
                .collect::<Vec<_>>(),
        );
        let scan_vals = (0..=scan_max_index)
            .map(|s| ScanIndex::try_from(s).unwrap())
            .collect::<Vec<_>>();
        let forward: HashMap<ScanIndex, Im> =
            scan_vals.into_iter().zip(im_vals.iter().cloned()).collect();
        let reverse = forward.into_iter().map(|(k, v)| (v, k)).collect();
        let result = Self {
            // sdk_reader,
            // forward,
            forward: im_vals,
            reverse,
        };
        Some(result)
    }
}

impl Converter<ScanIndex, Im> for WrappedScan2ImConverterSDK {
    fn convert(&self, value: ScanIndex) -> Im {
        // *self
        //     .forward
        //     .get(&value)
        //     .expect("ScanIndex not found in converter")
        self.forward[usize::from(value)]
    }
}

impl Converter<Im, ScanIndex> for WrappedScan2ImConverterSDK {
    fn convert(&self, value: Im) -> ScanIndex {
        *self.reverse.get(&value).expect("Mz not found in converter")
    }
}

fn parse_value<T: FromStr>(
    hash_map: &HashMap<String, String>,
    key: &str,
) -> Option<T> {
    let value: T = hash_map.get(key)?.parse().ok()?;
    Some(value)
}

#[derive(Deserialize)]
struct KvRow {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

fn get_metadata(path: &str) -> HashMap<String, String> {
    let tdf_path = std::path::Path::new(path);
    SqlReader::from(tdf_path.to_str().unwrap())
        .unwrap()
        .from_table::<KvRow>("GlobalMetadata")
        .unwrap()
        .read_all()
        .unwrap()
        .into_iter()
        .map(|r| (r.key, r.value))
        .collect()
}

#[derive(Deserialize)]
struct FrameRow {
    #[serde(rename = "NumScans")]
    num_scans: u32,
}
