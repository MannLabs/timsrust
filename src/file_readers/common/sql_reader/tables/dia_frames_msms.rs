use crate::file_readers::common::sql_reader::{ReadableFromSql, SqlReader};

#[derive(Debug)]
pub struct DiaFramesMsMsTable {
    pub group: Vec<usize>,
    pub scan_start: Vec<u16>,
    pub scan_end: Vec<u16>,
    pub mz_center: Vec<f64>,
    pub mz_width: Vec<f64>,
}

impl ReadableFromSql for DiaFramesMsMsTable {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let table_name: &str = "DiaFrameMsMsWindows";
        DiaFramesMsMsTable {
            group: sql_reader.read_column_from_table("WindowGroup", table_name),
            scan_start: sql_reader
                .read_column_from_table("ScanNumBegin", table_name),
            scan_end: sql_reader
                .read_column_from_table("ScanNumEnd", table_name),
            mz_center: sql_reader
                .read_column_from_table("IsolationMz", table_name),
            mz_width: sql_reader
                .read_column_from_table("IsolationWidth", table_name),
        }
    }
}
