mod metadata;
mod tables;

pub use tables::*;

use rusqlite::{Connection, Result, Statement};
use std::path::Path;

#[derive(Debug)]
pub struct SqlReader {
    pub path: String,
}

impl SqlReader {
    fn read_column_from_table<T: rusqlite::types::FromSql + Default>(
        &self,
        column_name: &str,
        table_name: &str,
    ) -> Vec<T> {
        let column_names: Vec<String> =
            self.get_table_columns(table_name).unwrap();
        let order_by: String = column_names.join(", ");
        let query: String = format!(
            "SELECT {} FROM {} ORDER BY {}",
            column_name, table_name, order_by
        );

        self.get_data_from_sql(&query)
    }

    pub fn get_data_from_sql<T: rusqlite::types::FromSql + Default>(
        &self,
        query: &String,
    ) -> Vec<T> {
        let connection: Connection = get_sql_connection(&self.path);
        let mut stmt: Statement = connection.prepare(&query).unwrap();
        let rows = stmt
            .query_map(
                [],
                // |row| row.get::<usize, T>(0)
                |row| match row.get::<usize, T>(0) {
                    Ok(value) => Ok(value),
                    _ => Ok(T::default()),
                },
            )
            .unwrap();
        rows.collect::<Result<Vec<T>>>().unwrap()
    }

    fn get_table_columns(&self, table_name: &str) -> Result<Vec<String>> {
        let connection: Connection = get_sql_connection(&self.path);
        let query = format!("PRAGMA table_info({})", table_name);
        let mut stmt: Statement = connection.prepare(&query)?;
        let rows = stmt.query_map([], |row| row.get::<usize, String>(1))?;
        rows.collect()
    }
}

fn get_sql_connection(path: &String) -> Connection {
    let db_file_path: std::path::PathBuf = Path::new(path).join("analysis.tdf");
    let connection: Connection = Connection::open(&db_file_path).unwrap();
    connection
}

pub trait ReadableFromSql {
    fn from_sql(sql_reader: &SqlReader) -> Self;
}
