use std::marker::PhantomData;
use std::path::PathBuf;

use crate::Uri;

/// Errors from JSON read/write operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A serialization error occurred.
    #[error("serialize error: {0}")]
    Serialize(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// A deserialization error occurred.
    #[error("deserialize error: {0}")]
    Deserialize(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// Binary read error occurred while reading the underlying file.
    #[error(transparent)]
    BinaryReader(#[from] crate::formats::binary::BinaryError),
}

/// Writes a single JSON value to a local file.
///
/// Each call to [`write`](JsonWriter::write) overwrites the entire file.
///
/// # Examples
///
/// ```
/// use filemanager::formats::json::JsonWriter;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record { id: u32, name: String }
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("out.json");
/// let mut writer = JsonWriter::<Record>::new(&path).unwrap();
/// writer.write(Record { id: 1, name: "Alice".into() }).unwrap();
/// ```
#[derive(Debug)]
pub struct JsonWriter<T> {
    path: PathBuf,
    _marker: PhantomData<T>,
}

impl<T: serde::Serialize> JsonWriter<T> {
    /// Creates a new `JsonWriter` targeting the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::json::JsonWriter;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.json");
    /// let _w = JsonWriter::<serde_json::Value>::new(&path).unwrap();
    /// ```
    pub fn new(path: impl Into<Uri>) -> Result<Self, JsonError> {
        let uri: Uri = path.into();
        let p = uri.as_path().ok_or_else(|| {
            JsonError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a local path",
            ))
        })?;
        Ok(JsonWriter {
            path: p,
            _marker: PhantomData,
        })
    }

    /// Serializes `value` and writes it to the file (truncates any existing content).
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::json::JsonWriter;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.json");
    /// let mut writer = JsonWriter::<Vec<u32>>::new(&path).unwrap();
    /// writer.write(vec![1, 2, 3]).unwrap();
    /// ```
    pub fn write(&mut self, value: T) -> Result<(), JsonError> {
        let file = std::fs::File::create(&self.path)?;
        serde_json::to_writer(file, &value)
            .map_err(|e| JsonError::Serialize(Box::new(e)))?;
        Ok(())
    }
}

/// Reads a JSON value from a [`ManagedFile`].
///
/// # Examples
///
/// ```
/// use filemanager::formats::json::{JsonWriter, JsonReader};
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("data.json");
/// let mut writer = JsonWriter::<Vec<u32>>::new(&path).unwrap();
/// writer.write(vec![10, 20, 30]).unwrap();
/// drop(writer);
///
/// let reader = JsonReader::<Vec<u32>>::from(&path).unwrap();
/// assert_eq!(reader.read_all().unwrap(), vec![10, 20, 30]);
/// ```
#[derive(Debug)]
pub struct JsonReader<T> {
    data: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T: for<'de> serde::Deserialize<'de>> JsonReader<T> {
    /// Creates a `JsonReader` from a [`ManagedFile`].
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::json::JsonReader;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.json");
    /// std::fs::write(&path, b"42").unwrap();
    /// let _reader = JsonReader::<u32>::from(&path).unwrap();
    /// ```
    pub fn from(uri: impl Into<Uri>) -> Result<Self, JsonError> {
        let binary_reader = crate::formats::binary::BinaryReader::from(uri)?;
        let data = binary_reader.read_range(..)?;
        Ok(JsonReader {
            data,
            _marker: PhantomData,
        })
    }

    /// Deserializes and returns the value from the file.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::json::JsonReader;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("test.json");
    /// std::fs::write(&path, b"[1,2,3]").unwrap();
    /// let reader = JsonReader::<Vec<u32>>::from(&path).unwrap();
    /// assert_eq!(reader.read_all().unwrap(), vec![1, 2, 3]);
    /// ```
    pub fn read_all(&self) -> Result<T, JsonError> {
        serde_json::from_slice(&self.data)
            .map_err(|e| JsonError::Deserialize(Box::new(e)))
    }
}

pub trait JsonFormat: Sized {
    fn write_to_json(self, uri: impl Into<Uri>) -> Result<(), JsonError>
    where
        Self: serde::Serialize,
    {
        let mut writer = JsonWriter::new(uri)?;
        writer.write(self)
    }

    fn read_from_json(uri: impl Into<Uri>) -> Result<Self, JsonError>
    where
        Self: for<'de> serde::Deserialize<'de>,
    {
        JsonReader::from(uri)?.read_all()
    }
}
