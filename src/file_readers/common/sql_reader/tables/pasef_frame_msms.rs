use crate::file_readers::common::sql_reader::{ReadableFromSql, SqlReader};

#[derive(Debug)]
pub struct PasefFrameMsMsTable {
    pub frame: Vec<usize>,
    pub scan_start: Vec<usize>,
    pub scan_end: Vec<usize>,
    pub mz_center: Vec<f64>,
    pub mz_width: Vec<f64>,
    pub collision_energy: Vec<f64>,
    pub precursor: Vec<usize>,
}

impl ReadableFromSql for PasefFrameMsMsTable {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let table_name: &str = "PasefFrameMsMsInfo";
        PasefFrameMsMsTable {
            frame: sql_reader.read_column_from_table("Frame", table_name),
            scan_start: sql_reader
                .read_column_from_table("ScanNumBegin", table_name),
            scan_end: sql_reader
                .read_column_from_table("ScanNumEnd", table_name),
            mz_center: sql_reader
                .read_column_from_table("IsolationMz", table_name),
            mz_width: sql_reader
                .read_column_from_table("IsolationWidth", table_name),
            collision_energy: sql_reader
                .read_column_from_table("CollisionEnergy", table_name),
            precursor: sql_reader
                .read_column_from_table("Precursor", table_name),
        }
    }
}
