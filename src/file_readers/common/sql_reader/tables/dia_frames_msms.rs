use std::collections::HashMap;

use crate::file_readers::common::sql_reader::{ReadableFromSql, SqlReader};

#[derive(Debug)]
pub struct DiaFramesMsMsTable {
    pub group: Vec<usize>,
    pub scan_start: Vec<u16>,
    pub scan_end: Vec<u16>,
    pub mz_center: Vec<f64>,
    pub mz_width: Vec<f64>,
}

#[derive(Debug)]
pub struct MsmsIsolationWindow {
    pub scan_start: usize,
    pub scan_end: usize,
    pub mz_center: f64,
    pub mz_width: f64,
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

impl DiaFramesMsMsTable {
    /// Returns a HashMap of MsmsIsolationWindow objects, grouped by group number.
    /// 
    /// # Example
    /// let tbl = DiaFramesMsMsTable{
    ///     group: vec![1, 1, 2, 2, 2, 3, 3, 3],
    ///     scan_start: vec![1, 20, 15, 30, 45, 1, 20, 30],
    ///     scan_end: vec![10, 30, 25, 40, 55, 19, 29, 40],
    ///     mz_center: vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0],
    ///     mz_width: vec![40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0],
    /// };
    /// let hashmap = tbl.as_hashmap();
    /// assert_eq!(hashmap.len(), 3);
    /// assert_eq!(hashmap.get(&1).unwrap().len(), 2);
    /// assert_eq!(hashmap.get(&2).unwrap().len(), 3);
    pub fn as_hashmap(&self) -> HashMap<usize, Vec<MsmsIsolationWindow>> {
        let mut hashmap: HashMap<usize, Vec<MsmsIsolationWindow>> =
            HashMap::new();
        for (index, &group) in self.group.iter().enumerate() {
            let scan_start: usize = self.scan_start[index] as usize;
            let scan_end: usize = self.scan_end[index] as usize;
            let mz_center: f64 = self.mz_center[index];
            let mz_width: f64 = self.mz_width[index];
            let window: MsmsIsolationWindow = MsmsIsolationWindow {
                scan_start,
                scan_end,
                mz_center,
                mz_width,
            };
            if hashmap.contains_key(&group) {
                hashmap.get_mut(&group).unwrap().push(window);
            } else {
                hashmap.insert(group, vec![window]);
            }
        }
        hashmap

    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hashmap_works(){
        use crate::file_readers::common::sql_reader::DiaFramesMsMsTable;
        let tbl = DiaFramesMsMsTable{
            group: vec![1, 1, 2, 2, 2, 3, 3, 3],
            scan_start: vec![1, 20, 15, 30, 45, 1, 20, 30],
            scan_end: vec![10, 30, 25, 40, 55, 19, 29, 40],
            mz_center: vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0],
            mz_width: vec![40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0, 40.0],
        };
        let hashmap = tbl.as_hashmap();
        assert_eq!(hashmap.len(), 3);
        assert_eq!(hashmap.get(&1).unwrap().len(), 2);
        assert_eq!(hashmap.get(&2).unwrap().len(), 3);
    }
}
