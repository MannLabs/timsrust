use crate::file_readers::common::sql_reader::{ReadableFromSql, SqlReader};

#[derive(Debug)]
pub struct DiaFramesInfoTable {
    pub frame: Vec<usize>,
    pub group: Vec<usize>,
}

impl ReadableFromSql for DiaFramesInfoTable {
    fn from_sql(sql_reader: &SqlReader) -> Self {
        let table_name: &str = "DiaFrameMsMsInfo";
        DiaFramesInfoTable {
            frame: sql_reader.read_column_from_table("Frame", table_name),
            group: sql_reader.read_column_from_table("WindowGroup", table_name),
        }
    }
}
