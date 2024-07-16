use super::ReadableSqlTable;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SqlQuadSettings {
    pub window_group: usize,
    pub scan_start: usize,
    pub scan_end: usize,
    pub mz_center: f64,
    pub mz_width: f64,
    pub collision_energy: f64,
}

impl ReadableSqlTable for SqlQuadSettings {
    fn get_sql_query() -> String {
        "SELECT WindowGroup, ScanNumBegin, ScanNumEnd, IsolationMz, IsolationWidth, CollisionEnergy FROM DiaFrameMsMsWindows".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            window_group: row.get(0).unwrap_or_default(),
            scan_start: row.get(1).unwrap_or_default(),
            scan_end: row.get(2).unwrap_or_default(),
            mz_center: row.get(3).unwrap_or_default(),
            mz_width: row.get(4).unwrap_or_default(),
            collision_energy: row.get(5).unwrap_or_default(),
        }
    }
}
