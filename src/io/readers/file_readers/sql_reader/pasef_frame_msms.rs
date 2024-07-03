use super::ReadableSqlTable;

#[derive(Debug, PartialEq)]
pub struct SqlPasefFrameMsMs {
    pub frame: usize,
    pub scan_start: usize,
    pub scan_end: usize,
    pub isolation_mz: f64,
    pub isolation_width: f64,
    pub collision_energy: f64,
    pub precursor: usize,
}

impl ReadableSqlTable for SqlPasefFrameMsMs {
    fn get_sql_query() -> String {
        "SELECT Frame, ScanNumBegin, ScanNumEnd, IsolationMz, IsolationWidth, CollisionEnergy, Precursor FROM PasefFrameMsMsInfo".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            frame: row.get(0).unwrap_or_default(),
            scan_start: row.get(1).unwrap_or_default(),
            scan_end: row.get(2).unwrap_or_default(),
            isolation_mz: row.get(3).unwrap_or_default(),
            isolation_width: row.get(4).unwrap_or_default(),
            collision_energy: row.get(5).unwrap_or_default(),
            precursor: row.get(6).unwrap_or_default(),
        }
    }
}
