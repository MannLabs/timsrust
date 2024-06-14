pub mod frames;
pub mod pasef_frame_msms;
pub mod precursors;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

#[derive(Debug)]
pub struct SqlReader {
    path: PathBuf,
    connection: Connection,
}

impl SqlReader {
    pub fn open(file_name: impl AsRef<Path>) -> Result<SqlReader, SqlError> {
        let path = file_name.as_ref().to_path_buf();
        let connection = Connection::open(&path)?;
        Ok(Self { path, connection })
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}

pub trait SqlReadable {
    fn get_sql_query() -> String;

    fn from_sql_row(row: &rusqlite::Row) -> Self;

    fn from_sql_reader(reader: &SqlReader) -> Result<Vec<Self>, SqlError>
    where
        Self: Sized,
    {
        let query = Self::get_sql_query();
        let mut stmt = reader.connection.prepare(&query)?;
        let rows = stmt.query_map([], |row| Ok(Self::from_sql_row(row)))?;
        let result = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("SqlError: {0}")]
pub struct SqlError(#[from] rusqlite::Error);
