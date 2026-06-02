use std::marker::PhantomData;
use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};

use crate::{CacheError, Uri, UriError};

/// Errors from SQL read operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum SqlError {
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A SQLite error occurred.
    #[error("sql error: {0}")]
    Sql(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The requested table was not found.
    #[error("table not found: {0}")]
    TableNotFound(String),
    /// A deserialization error occurred.
    #[error("deserialize error: {0}")]
    Deserialize(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The requested row range is out of bounds.
    #[error("out of bounds: {start}..{end} (rows {rows})")]
    OutOfBounds {
        start: usize,
        end: usize,
        rows: usize,
    },
    /// Binary read error occurred while reading the underlying file.
    #[error(transparent)]
    Uri(#[from] UriError),
    #[error(transparent)]
    Cache(#[from] CacheError),
}

/// Reads SQLite databases from a [`ManagedFile`].
///
/// # Examples
///
/// ```
/// use filemanager::formats::sql::SqlReader;
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("test.db");
/// {
///     let conn = rusqlite::Connection::open(&path).unwrap();
///     conn.execute_batch("CREATE TABLE items (id INTEGER, name TEXT);").unwrap();
/// }
/// let reader = SqlReader::from(&path).unwrap();
/// let tables = reader.tables().unwrap();
/// assert!(tables.contains(&"items".to_string()));
/// ```
#[derive(Debug)]
pub struct SqlReader {
    path: PathBuf,
    original_uri: Uri,
    effective_uri: Uri,
}

impl SqlReader {
    /// Creates a `SqlReader` from a [`ManagedFile`].
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch("CREATE TABLE t (x INTEGER);").unwrap();
    /// }
    /// let _reader = SqlReader::from(&path).unwrap();
    /// ```
    pub fn from(uri: impl Into<Uri>) -> Result<Self, SqlError> {
        let original_uri = uri.into();
        let effective_uri = original_uri.force_cache()?;
        let path = effective_uri.as_path().ok_or_else(|| {
            SqlError::Uri(crate::UriError::InvalidUri(effective_uri.clone()))
        })?;
        // Verify the file is readable
        let _conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| SqlError::Sql(Box::new(e)))?;
        Ok(SqlReader {
            path,
            original_uri,
            effective_uri,
        })
    }

    /// Returns the names of all tables in the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch("CREATE TABLE foo (x INTEGER); CREATE TABLE bar (y TEXT);").unwrap();
    /// }
    /// let reader = SqlReader::from(&path).unwrap();
    /// let mut tables = reader.tables().unwrap();
    /// tables.sort();
    /// assert_eq!(tables, vec!["bar", "foo"]);
    /// ```
    pub fn tables(&self) -> Result<Vec<String>, SqlError> {
        let conn = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| SqlError::Sql(Box::new(e)))?;
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .map_err(|e| SqlError::Sql(Box::new(e)))?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| SqlError::Sql(Box::new(e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| SqlError::Sql(Box::new(e)))?;
        Ok(names)
    }

    /// Returns a typed [`SqlTable`] for the given table name.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug)]
    /// struct Item { id: i64, name: String }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch(
    ///         "CREATE TABLE items (id INTEGER, name TEXT); INSERT INTO items VALUES (1, 'Alice');"
    ///     ).unwrap();
    /// }
    /// let reader = SqlReader::from(&path).unwrap();
    /// let table = reader.from_table::<Item>("items").unwrap();
    /// assert_eq!(table.shape(), (1, 2));
    /// ```
    pub fn from_table<T>(&self, name: &str) -> Result<SqlTable<T>, SqlError> {
        let conn = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| SqlError::Sql(Box::new(e)))?;

        // Check table exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![name],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| SqlError::Sql(Box::new(e)))?
            > 0;

        if !exists {
            return Err(SqlError::TableNotFound(name.to_string()));
        }

        let rows: usize =
            conn.query_row(
                &format!("SELECT COUNT(*) FROM \"{}\"", name),
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| SqlError::Sql(Box::new(e)))? as usize;

        let mut pragma_stmt = conn
            .prepare(&format!("PRAGMA table_info(\"{}\")", name))
            .map_err(|e| SqlError::Sql(Box::new(e)))?;
        let cols = pragma_stmt
            .query_map([], |_| Ok(()))
            .map_err(|e| SqlError::Sql(Box::new(e)))?
            .count();

        Ok(SqlTable {
            db_path: self.path.clone(),
            table_name: name.to_string(),
            rows,
            cols,
            _marker: PhantomData,
        })
    }

    pub fn original_uri(&self) -> &Uri {
        &self.original_uri
    }

    pub fn effective_uri(&self) -> &Uri {
        &self.effective_uri
    }
}

/// A typed view of a SQLite table.
///
/// # Examples
///
/// ```
/// use filemanager::formats::sql::SqlReader;
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Item { id: i64, name: String }
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("test.db");
/// {
///     let conn = rusqlite::Connection::open(&path).unwrap();
///     conn.execute_batch(
///         "CREATE TABLE items (id INTEGER, name TEXT); INSERT INTO items VALUES (1, 'Alice');"
///     ).unwrap();
/// }
/// let reader = SqlReader::from(&path).unwrap();
/// let table = reader.from_table::<Item>("items").unwrap();
/// let items = table.read_all().unwrap();
/// assert_eq!(items[0], Item { id: 1, name: "Alice".into() });
/// ```
#[derive(Debug)]
pub struct SqlTable<T> {
    db_path: PathBuf,
    table_name: String,
    rows: usize,
    cols: usize,
    _marker: PhantomData<T>,
}

impl<T: for<'de> serde::Deserialize<'de>> SqlTable<T> {
    /// Returns the `(rows, cols)` shape of the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Row { id: i64 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch("CREATE TABLE t (id INTEGER); INSERT INTO t VALUES (1);").unwrap();
    /// }
    /// let reader = SqlReader::from(&path).unwrap();
    /// let table = reader.from_table::<Row>("t").unwrap();
    /// assert_eq!(table.shape(), (1, 1));
    /// ```
    #[must_use]
    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// Returns `true` if the table has no rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Row { id: i64 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch("CREATE TABLE empty_table (id INTEGER);").unwrap();
    /// }
    /// let reader = SqlReader::from(&path).unwrap();
    /// let table = reader.from_table::<Row>("empty_table").unwrap();
    /// assert!(table.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// Reads all rows from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Row { val: i64 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch(
    ///         "CREATE TABLE t (val INTEGER); INSERT INTO t VALUES (10); INSERT INTO t VALUES (20);"
    ///     ).unwrap();
    /// }
    /// let reader = SqlReader::from(&path).unwrap();
    /// let table = reader.from_table::<Row>("t").unwrap();
    /// assert_eq!(table.read_all().unwrap(), vec![Row { val: 10 }, Row { val: 20 }]);
    /// ```
    pub fn read_all(&self) -> Result<Vec<T>, SqlError> {
        self.query(self.rows, 0)
    }

    /// Reads a row range from the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::sql::SqlReader;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Row { val: i64 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.db");
    /// {
    ///     let conn = rusqlite::Connection::open(&path).unwrap();
    ///     conn.execute_batch(
    ///         "CREATE TABLE t (val INTEGER); INSERT INTO t VALUES (1); INSERT INTO t VALUES (2); INSERT INTO t VALUES (3);"
    ///     ).unwrap();
    /// }
    /// let reader = SqlReader::from(&path).unwrap();
    /// let table = reader.from_table::<Row>("t").unwrap();
    /// assert_eq!(table.read_range(1..3).unwrap(), vec![Row { val: 2 }, Row { val: 3 }]);
    /// ```
    pub fn read_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Result<Vec<T>, SqlError> {
        if range.end > self.rows {
            return Err(SqlError::OutOfBounds {
                start: range.start,
                end: range.end,
                rows: self.rows,
            });
        }
        let limit = range.end - range.start;
        let offset = range.start;
        self.query(limit, offset)
    }

    fn query(&self, limit: usize, offset: usize) -> Result<Vec<T>, SqlError> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| SqlError::Sql(Box::new(e)))?;

        let sql = format!(
            "SELECT * FROM \"{}\" LIMIT {} OFFSET {}",
            self.table_name, limit, offset
        );
        let mut stmt =
            conn.prepare(&sql).map_err(|e| SqlError::Sql(Box::new(e)))?;

        let col_names: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt
            .query_map([], |row| {
                let mut map = serde_json::Map::new();
                for (i, col) in col_names.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i)?;
                    map.insert(col.clone(), rusqlite_value_to_json(val));
                }
                Ok(map)
            })
            .map_err(|e| SqlError::Sql(Box::new(e)))?;

        let mut result = Vec::with_capacity(limit);
        for row_result in rows {
            let map = row_result.map_err(|e| SqlError::Sql(Box::new(e)))?;
            let value = serde_json::Value::Object(map);
            // if let Ok(item) = serde_json::from_value::<T>(value) {
            //     result.push(item);
            // }
            let item = serde_json::from_value::<T>(value)
                .map_err(|e| SqlError::Deserialize(Box::new(e)))?;
            result.push(item);
        }
        Ok(result)
    }
}

fn rusqlite_value_to_json(v: rusqlite::types::Value) -> serde_json::Value {
    match v {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
        rusqlite::types::Value::Real(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
        rusqlite::types::Value::Blob(_) => serde_json::Value::Null,
    }
}
