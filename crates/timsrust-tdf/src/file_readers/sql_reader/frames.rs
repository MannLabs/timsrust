use serde::Deserialize;

use super::ReadableSqlTable;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct SqlFrame {
    #[serde(rename = "Id")]
    pub id: usize,
    #[serde(rename = "ScanMode")]
    pub scan_mode: u8,
    #[serde(rename = "MsMsType")]
    pub msms_type: u8,
    #[serde(rename = "NumPeaks")]
    pub peak_count: u64,
    #[serde(rename = "Time")]
    pub rt: f64,
    #[serde(rename = "NumScans")]
    pub scan_count: u64,
    #[serde(rename = "TimsId")]
    pub binary_offset: usize,
    #[serde(rename = "AccumulationTime")]
    pub accumulation_time: f64,
}

impl ReadableSqlTable for SqlFrame {
    fn table_name() -> &'static str {
        "Frames"
    }
}
