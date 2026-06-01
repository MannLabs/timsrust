use serde::Deserialize;

use super::ReadableSqlTable;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct SqlWindowGroup {
    #[serde(rename = "Frame")]
    pub frame: usize,
    #[serde(rename = "WindowGroup")]
    pub window_group: usize,
}

impl ReadableSqlTable for SqlWindowGroup {
    fn table_name() -> &'static str {
        "DiaFrameMsMsInfo"
    }
}
