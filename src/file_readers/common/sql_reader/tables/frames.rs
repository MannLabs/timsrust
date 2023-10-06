use crate::file_readers::common::sql_reader::{ReadableFromSql, SqlReader};

#[derive(Debug)]
pub struct FrameTable {
    pub id: Vec<usize>,
    pub scan_mode: Vec<u8>,
    pub msms_type: Vec<u8>,
    pub peak_count: Vec<u64>,
    pub rt: Vec<f64>,
    pub scan_count: Vec<u64>,
    pub offsets: Vec<u64>,
}

impl ReadableFromSql for FrameTable {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let table_name: &str = "Frames";
        FrameTable {
            id: sql_reader.read_column_from_table("Id", table_name),
            scan_mode: sql_reader
                .read_column_from_table("ScanMode", table_name),
            msms_type: sql_reader
                .read_column_from_table("MsMsType", table_name),
            peak_count: sql_reader
                .read_column_from_table("NumPeaks", table_name),
            rt: sql_reader.read_column_from_table("Time", table_name),
            scan_count: sql_reader
                .read_column_from_table("NumScans", table_name),
            offsets: sql_reader.read_column_from_table("TimsId", "Frames"),
        }
    }
}
