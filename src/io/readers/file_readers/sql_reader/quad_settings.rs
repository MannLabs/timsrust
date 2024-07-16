use super::{ParseDefault, ReadableSqlTable};

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
            window_group: row.parse_default(0),
            scan_start: row.parse_default(1),
            scan_end: row.parse_default(2),
            mz_center: row.parse_default(3),
            mz_width: row.parse_default(4),
            collision_energy: row.parse_default(5),
        }
    }
}
