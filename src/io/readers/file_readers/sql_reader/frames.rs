use super::{ParseDefault, ReadableSqlTable};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SqlFrame {
    pub id: usize,
    pub scan_mode: u8,
    pub msms_type: u8,
    pub peak_count: u64,
    pub rt: f64,
    pub scan_count: u64,
    pub binary_offset: usize,
    pub accumulation_time: f64,
}

impl ReadableSqlTable for SqlFrame {
    fn get_sql_query() -> String {
        "SELECT Id, ScanMode, MsMsType, NumPeaks, Time, NumScans, TimsId, AccumulationTime FROM Frames".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            id: row.parse_default(0),
            scan_mode: row.parse_default(1),
            msms_type: row.parse_default(2),
            peak_count: row.parse_default(3),
            rt: row.parse_default(4),
            scan_count: row.parse_default(5),
            binary_offset: row.parse_default(6),
            accumulation_time: row.parse_default(7),
        }
    }
}
