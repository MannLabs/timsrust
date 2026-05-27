use serde::Deserialize;

use super::ReadableSqlTable;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct SqlPasefFrameMsMs {
    #[serde(rename = "Frame")]
    pub frame: usize,
    #[serde(rename = "ScanNumBegin")]
    pub scan_start: usize,
    #[serde(rename = "ScanNumEnd")]
    pub scan_end: usize,
    #[serde(rename = "IsolationMz")]
    pub isolation_mz: f64,
    #[serde(rename = "IsolationWidth")]
    pub isolation_width: f64,
    #[serde(rename = "CollisionEnergy")]
    pub collision_energy: f64,
    #[serde(rename = "Precursor")]
    pub precursor: usize,
}

impl ReadableSqlTable for SqlPasefFrameMsMs {
    fn table_name() -> &'static str {
        "PasefFrameMsMsInfo"
    }
}
