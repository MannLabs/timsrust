use crate::file_readers::common::sql_reader::{ReadableFromSql, SqlReader};

#[derive(Debug)]
pub struct PrecursorTable {
    pub id: Vec<usize>,
    pub mz: Vec<f64>,
    pub charge: Vec<usize>,
    pub scan_average: Vec<f64>,
    pub intensity: Vec<f64>,
    pub precursor_frame: Vec<usize>,
}

impl ReadableFromSql for PrecursorTable {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let table_name: &str = "Precursors";
        PrecursorTable {
            id: sql_reader.read_column_from_table("Id", table_name),
            mz: sql_reader.read_column_from_table("MonoisotopicMz", table_name),
            charge: sql_reader.read_column_from_table("Charge", table_name),
            scan_average: sql_reader
                .read_column_from_table("ScanNumber", table_name),
            intensity: sql_reader
                .read_column_from_table("Intensity", table_name),
            precursor_frame: sql_reader
                .read_column_from_table("Parent", table_name),
        }
    }
}
