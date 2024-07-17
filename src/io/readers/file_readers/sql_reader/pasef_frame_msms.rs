use super::{ParseDefault, ReadableSqlTable};

#[derive(Clone, Debug, Default, PartialEq)]
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
            frame: row.parse_default(0),
            scan_start: row.parse_default(1),
            scan_end: row.parse_default(2),
            isolation_mz: row.parse_default(3),
            isolation_width: row.parse_default(4),
            collision_energy: row.parse_default(5),
            precursor: row.parse_default(6),
        }
    }
}
