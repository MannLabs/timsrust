pub mod frame_groups;
pub mod frames;
pub mod metadata;
pub mod pasef_frame_msms;
pub mod precursors;
pub mod quad_settings;

use std::collections::HashMap;

use rusqlite::{types::FromSql, Connection};

use crate::readers::{TimsTofPathError, TimsTofPathLike};

#[derive(Debug)]
pub struct SqlReader {
    connection: Connection,
}

impl SqlReader {
    pub fn open(path: impl TimsTofPathLike) -> Result<Self, SqlReaderError> {
        let path = path.to_timstof_path()?;
        let connection = Connection::open(&path.tdf()?)?;
        Ok(Self { connection })
    }

    pub fn read_column_from_table<T: rusqlite::types::FromSql + Default>(
        &self,
        column_name: &str,
        table_name: &str,
    ) -> Result<Vec<T>, SqlReaderError> {
        let query = format!("SELECT {} FROM {}", column_name, table_name);
        let mut stmt = self.connection.prepare(&query)?;
        let rows = stmt.query_map([], |row| match row.get::<usize, T>(0) {
            Ok(value) => Ok(value),
            _ => Ok(T::default()),
        })?;
        let result = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
}

pub trait ReadableSqlTable {
    fn get_sql_query() -> String;

    fn from_sql_row(row: &rusqlite::Row) -> Self;

    fn from_sql_reader(reader: &SqlReader) -> Result<Vec<Self>, SqlReaderError>
    where
        Self: Sized,
    {
        let query = Self::get_sql_query();
        let mut stmt = reader.connection.prepare(&query)?;
        let rows = stmt.query_map([], |row| Ok(Self::from_sql_row(row)))?;
        let result = rows.collect::<Result<Vec<_>, _>>()?;
        if result.len() == 0 {
            Err(SqlReaderError::SqlError(
                rusqlite::Error::QueryReturnedNoRows,
            ))
        } else {
            Ok(result)
        }
    }
}

pub trait ReadableSqlHashMap {
    fn get_sql_query() -> String;

    fn from_sql_reader(
        reader: &SqlReader,
    ) -> Result<HashMap<String, String>, SqlReaderError>
    where
        Self: Sized,
    {
        let query = Self::get_sql_query();
        let mut stmt = reader.connection.prepare(&query)?;
        let mut result = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            result.insert(key, value);
            Ok(())
        })?;
        rows.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
}

pub trait ParseDefault {
    fn parse_default<T: Default + FromSql>(&self, index: usize) -> T;
}

impl ParseDefault for rusqlite::Row<'_> {
    fn parse_default<T: Default + FromSql>(&self, index: usize) -> T {
        self.get(index).unwrap_or_default()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SqlReaderError {
    #[error("{0}")]
    SqlError(#[from] rusqlite::Error),
    #[error("{0}")]
    TimsTofPathError(#[from] TimsTofPathError),
}
