use serde::Deserialize;

use super::ReadableSqlTable;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct SqlPrecursor {
    #[serde(rename = "Id")]
    pub id: usize,
    #[serde(rename = "MonoisotopicMz")]
    pub mz: Option<f64>,
    #[serde(rename = "Charge")]
    pub charge: Option<usize>,
    #[serde(rename = "ScanNumber")]
    pub scan_average: f64,
    #[serde(rename = "Intensity")]
    pub intensity: f64,
    #[serde(rename = "Parent")]
    pub precursor_frame: usize,
}

impl ReadableSqlTable for SqlPrecursor {
    fn table_name() -> &'static str {
        "Precursors"
    }
}
