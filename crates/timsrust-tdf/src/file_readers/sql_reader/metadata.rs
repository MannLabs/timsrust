use super::ReadableSqlHashMap;

pub(crate) struct SqlMetadata;

impl ReadableSqlHashMap for SqlMetadata {
    fn table_name() -> &'static str {
        "GlobalMetadata"
    }
}
