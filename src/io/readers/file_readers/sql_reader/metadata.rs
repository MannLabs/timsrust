use super::ReadableSqlHashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SqlMetadata;

impl ReadableSqlHashMap for SqlMetadata {
    fn get_sql_query() -> String {
        "SELECT Key, Value FROM GlobalMetadata".to_string()
    }
}
