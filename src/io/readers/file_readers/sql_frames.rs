use super::sql_reader::SqlReadable;

#[derive(Debug, PartialEq)]
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

impl SqlReadable for SqlFrame {
    fn get_sql_query() -> String {
        "SELECT Id, ScanMode, MsMsType, NumPeaks, Time, NumScans, TimsId, AccumulationTime FROM Frames".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        SqlFrame {
            id: row.get(0).unwrap_or_default(),
            scan_mode: row.get(1).unwrap_or_default(),
            msms_type: row.get(2).unwrap_or_default(),
            peak_count: row.get(3).unwrap_or_default(),
            rt: row.get(4).unwrap_or_default(),
            scan_count: row.get(5).unwrap_or_default(),
            binary_offset: row.get(6).unwrap_or_default(),
            accumulation_time: row.get(7).unwrap_or_default(),
        }
    }
}
