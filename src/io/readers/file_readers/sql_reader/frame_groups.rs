use super::{ParseDefault, ReadableSqlTable};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SqlWindowGroup {
    pub frame: usize,
    pub window_group: u8,
}

impl ReadableSqlTable for SqlWindowGroup {
    fn get_sql_query() -> String {
        "SELECT Frame, WindowGroup FROM DiaFrameMsMsInfo".to_string()
    }

    fn from_sql_row(row: &rusqlite::Row) -> Self {
        Self {
            frame: row.parse_default(0),
            window_group: row.parse_default(1),
        }
    }
}
