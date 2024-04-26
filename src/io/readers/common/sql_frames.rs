use super::sql_reader::SqlReadable;

#[derive(Debug, Default, PartialEq)]
pub struct SqlFrame {
    pub id: usize,
    pub scan_mode: u8,
    pub msms_type: u8,
    pub peak_count: u64,
    pub rt: f64,
    pub scan_count: u64,
    pub binary_offset: usize,
}

impl SqlReadable for SqlFrame {
    fn get_sql_query() -> String {
        "SELECT Id, ScanMode, MsMsType, NumPeaks, Time, NumScans, TimsId FROM Frames".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        SqlFrame {
            id: row.get(0).unwrap_or_default(),
            scan_mode: row.get(1).unwrap_or_default(),
            msms_type: row.get(2).unwrap_or_default(),
            peak_count: row.get(3).unwrap_or_default(),
            rt: row.get(4).unwrap_or_default(),
            scan_count: row.get(5).unwrap_or_default(),
            binary_offset: row.get(6).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::readers::common::sql_reader::SqlReader;

    #[test]
    fn test_get() {
        let reader =
            SqlReader::open("tests/test.d/analysis.tdf".to_string()).unwrap();
        let sql_frames = SqlFrame::from_sql_reader(&reader).unwrap();
        let target = [
            SqlFrame {
                id: 1,
                scan_mode: 8,
                msms_type: 0,
                peak_count: 10,
                rt: 0.1,
                scan_count: 4,
                binary_offset: 0,
            },
            SqlFrame {
                id: 2,
                scan_mode: 8,
                msms_type: 8,
                peak_count: 26,
                rt: 0.2,
                scan_count: 4,
                binary_offset: 48,
            },
            SqlFrame {
                id: 3,
                scan_mode: 8,
                msms_type: 0,
                peak_count: 42,
                rt: 0.3,
                scan_count: 4,
                binary_offset: 130,
            },
            SqlFrame {
                id: 4,
                scan_mode: 8,
                msms_type: 8,
                peak_count: 58,
                rt: 0.4,
                scan_count: 4,
                binary_offset: 235,
            },
        ];
        assert_eq!(sql_frames, target);
    }
}
