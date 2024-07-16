use super::ReadableSqlTable;

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
            id: row.get(0).unwrap_or_default(),
            mz: row.get(1).unwrap_or_default(),
            charge: row.get(2).unwrap_or_default(),
            scan_average: row.get(3).unwrap_or_default(),
            intensity: row.get(4).unwrap_or_default(),
            precursor_frame: row.get(5).unwrap_or_default(),
        }
    }
}
