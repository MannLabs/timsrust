use serde::Deserialize;

use super::ReadableSqlTable;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct SqlQuadSettings {
    #[serde(rename = "WindowGroup")]
    pub window_group: usize,
    #[serde(rename = "ScanNumBegin")]
    pub scan_start: usize,
    #[serde(rename = "ScanNumEnd")]
    pub scan_end: usize,
    #[serde(rename = "IsolationMz")]
    pub mz_center: f64,
    #[serde(rename = "IsolationWidth")]
    pub mz_width: f64,
    #[serde(rename = "CollisionEnergy")]
    pub collision_energy: f64,
}

impl ReadableSqlTable for SqlQuadSettings {
    fn table_name() -> &'static str {
        "DiaFrameMsMsWindows"
    }
}
