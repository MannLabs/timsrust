use super::{ParseDefault, ReadableSqlTable};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SqlPrecursor {
    pub id: usize,
    pub mz: f64,
    pub charge: usize,
    pub scan_average: f64,
    pub intensity: f64,
    pub precursor_frame: usize,
}

impl ReadableSqlTable for SqlPrecursor {
    fn get_sql_query() -> String {
        "SELECT Id, MonoisotopicMz, Charge, ScanNumber, Intensity, Parent FROM Precursors".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            id: row.parse_default(0),
            mz: row.parse_default(1),
            charge: row.parse_default(2),
            scan_average: row.parse_default(3),
            intensity: row.parse_default(4),
            precursor_frame: row.parse_default(5),
        }
    }
}
