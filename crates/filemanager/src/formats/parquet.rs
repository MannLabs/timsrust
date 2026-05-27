use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use arrow::datatypes::{FieldRef, Schema};
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use serde_arrow::schema::{SchemaLike, TracingOptions};

use crate::Uri;

/// Errors from Parquet read/write operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ParquetError {
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A write error occurred.
    #[error("parquet write error: {0}")]
    Write(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// A read error occurred.
    #[error("parquet read error: {0}")]
    Read(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The requested row range is out of bounds.
    #[error("out of bounds: {start}..{end} (rows {rows})")]
    OutOfBounds {
        start: usize,
        end: usize,
        rows: usize,
    },
    #[error(transparent)]
    Cache(#[from] crate::CacheError),
}

/// Writes records to a Parquet file using Apache Arrow columnar format.
///
/// # Examples
///
/// ```
/// use filemanager::formats::parquet::ParquetWriter;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Row { id: u32, value: f64 }
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("data.parquet");
/// let mut writer = ParquetWriter::<Row>::new(&path).unwrap();
/// writer.write_batch(vec![
///     Row { id: 1, value: 1.5 },
///     Row { id: 2, value: 2.5 },
/// ]).unwrap();
/// ```
#[derive(Debug)]
pub struct ParquetWriter<T> {
    writer: Option<ArrowWriter<std::fs::File>>,
    rows: usize,
    cols: usize,
    _marker: PhantomData<T>,
}

impl<T: serde::Serialize + for<'de> serde::Deserialize<'de>> ParquetWriter<T> {
    /// Creates a new `ParquetWriter` targeting the given local path.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::ParquetWriter;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Row { x: i32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("out.parquet");
    /// let _w = ParquetWriter::<Row>::new(&path).unwrap();
    /// ```
    pub fn new(path: impl Into<Uri>) -> Result<Self, ParquetError> {
        let uri: Uri = path.into();
        let p = uri.as_path().ok_or_else(|| {
            ParquetError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a local path",
            ))
        })?;

        let fields = Vec::<FieldRef>::from_type::<T>(TracingOptions::default())
            .map_err(|e| ParquetError::Write(Box::new(e)))?;
        let cols = fields.len();
        let schema = Arc::new(Schema::new(fields));
        let file = std::fs::File::create(p)?;
        let props = WriterProperties::builder().build();
        let writer = ArrowWriter::try_new(file, schema, Some(props))
            .map_err(|e| ParquetError::Write(Box::new(e)))?;

        Ok(ParquetWriter {
            writer: Some(writer),
            rows: 0,
            cols,
            _marker: PhantomData,
        })
    }

    /// Writes a batch of records to the Parquet file.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::ParquetWriter;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Row { id: u32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let mut writer = ParquetWriter::<Row>::new(&path).unwrap();
    /// writer.write_batch(vec![Row { id: 1 }, Row { id: 2 }]).unwrap();
    /// assert_eq!(writer.shape(), (2, 1));
    /// ```
    pub fn write_batch(&mut self, batch: Vec<T>) -> Result<(), ParquetError> {
        let fields = Vec::<FieldRef>::from_type::<T>(TracingOptions::default())
            .map_err(|e| ParquetError::Write(Box::new(e)))?;
        let record_batch = serde_arrow::to_record_batch(&fields, &batch)
            .map_err(|e| ParquetError::Write(Box::new(e)))?;

        let n = record_batch.num_rows();

        if let Some(ref mut w) = self.writer {
            w.write(&record_batch)
                .map_err(|e| ParquetError::Write(Box::new(e)))?;
            w.flush().map_err(|e| ParquetError::Write(Box::new(e)))?;
        }

        self.rows += n;
        Ok(())
    }

    /// Returns the current `(rows, cols)` shape of data written so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::ParquetWriter;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Row { a: i32, b: i32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let writer = ParquetWriter::<Row>::new(&path).unwrap();
    /// assert_eq!(writer.shape(), (0, 2));
    /// ```
    #[must_use]
    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
}

impl<T> Drop for ParquetWriter<T> {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            let _ = writer.close();
        }
    }
}

/// Reads records from a Parquet file.
///
/// # Examples
///
/// ```
/// use filemanager::formats::parquet::{ParquetWriter, ParquetReader};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq)]
/// struct Row { id: u32, val: f64 }
///
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("data.parquet");
/// let mut writer = ParquetWriter::<Row>::new(&path).unwrap();
/// writer.write_batch(vec![Row { id: 1, val: 1.0 }, Row { id: 2, val: 2.0 }]).unwrap();
/// drop(writer);
///
/// let reader = ParquetReader::<Row>::from(&path).unwrap();
/// let rows = reader.read_all().unwrap();
/// assert_eq!(rows.len(), 2);
/// ```
#[derive(Debug)]
pub struct ParquetReader<T> {
    path: PathBuf,
    rows: usize,
    cols: usize,
    _marker: PhantomData<T>,
}

impl<T: for<'de> serde::Deserialize<'de>> ParquetReader<T> {
    /// Creates a `ParquetReader` from a [`ManagedFile`].
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::{ParquetWriter, ParquetReader};
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Row { x: i32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let mut w = ParquetWriter::<Row>::new(&path).unwrap();
    /// w.write_batch(vec![Row { x: 1 }]).unwrap();
    /// drop(w);
    /// let _reader = ParquetReader::<Row>::from(&path).unwrap();
    /// ```
    pub fn from(uri: impl Into<Uri>) -> Result<Self, ParquetError> {
        let uri = uri.into();
        let uri = uri.force_cache()?;
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let path = uri.as_path().ok_or_else(|| {
            ParquetError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not a local file",
            ))
        })?;
        let f = std::fs::File::open(&path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(f)
            .map_err(|e| ParquetError::Read(Box::new(e)))?;
        let metadata = builder.metadata();
        let rows: usize = metadata
            .row_groups()
            .iter()
            .map(|rg| rg.num_rows() as usize)
            .sum();
        let schema = builder.schema();
        let cols = schema.fields().len();

        Ok(ParquetReader {
            path,
            rows,
            cols,
            _marker: PhantomData,
        })
    }

    /// Returns the `(rows, cols)` shape of the Parquet file.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::{ParquetWriter, ParquetReader};
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Row { a: i32, b: f64 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let mut w = ParquetWriter::<Row>::new(&path).unwrap();
    /// w.write_batch(vec![Row { a: 1, b: 2.0 }]).unwrap();
    /// drop(w);
    /// let reader = ParquetReader::<Row>::from(&path).unwrap();
    /// assert_eq!(reader.shape(), (1, 2));
    /// ```
    #[must_use]
    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// Returns `true` if the file has no rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::{ParquetWriter, ParquetReader};
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Row { x: i32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let _w = ParquetWriter::<Row>::new(&path).unwrap();
    /// drop(_w);
    /// let reader = ParquetReader::<Row>::from(&path).unwrap();
    /// assert!(reader.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// Reads all records from the Parquet file.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::{ParquetWriter, ParquetReader};
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, PartialEq, Debug)]
    /// struct Row { n: u32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let mut w = ParquetWriter::<Row>::new(&path).unwrap();
    /// w.write_batch(vec![Row { n: 42 }]).unwrap();
    /// drop(w);
    /// let reader = ParquetReader::<Row>::from(&path).unwrap();
    /// assert_eq!(reader.read_all().unwrap(), vec![Row { n: 42 }]);
    /// ```
    pub fn read_all(&self) -> Result<Vec<T>, ParquetError> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let f = std::fs::File::open(&self.path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(f)
            .map_err(|e| ParquetError::Read(Box::new(e)))?;
        let reader = builder
            .build()
            .map_err(|e| ParquetError::Read(Box::new(e)))?;

        let mut result = Vec::with_capacity(self.rows);
        for batch in reader {
            let batch = batch.map_err(|e| ParquetError::Read(Box::new(e)))?;
            let mut items: Vec<T> = serde_arrow::from_record_batch(&batch)
                .map_err(|e| ParquetError::Read(Box::new(e)))?;
            result.append(&mut items);
        }
        Ok(result)
    }

    /// Reads a row range from the Parquet file.
    ///
    /// # Examples
    ///
    /// ```
    /// use filemanager::formats::parquet::{ParquetWriter, ParquetReader};
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize, PartialEq, Debug)]
    /// struct Row { n: u32 }
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("data.parquet");
    /// let mut w = ParquetWriter::<Row>::new(&path).unwrap();
    /// w.write_batch(vec![Row { n: 1 }, Row { n: 2 }, Row { n: 3 }]).unwrap();
    /// drop(w);
    /// let reader = ParquetReader::<Row>::from(&path).unwrap();
    /// assert_eq!(reader.read_range(1..3).unwrap(), vec![Row { n: 2 }, Row { n: 3 }]);
    /// ```
    pub fn read_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> Result<Vec<T>, ParquetError> {
        if range.end > self.rows {
            return Err(ParquetError::OutOfBounds {
                start: range.start,
                end: range.end,
                rows: self.rows,
            });
        }
        let all = self.read_all()?;
        Ok(all
            .into_iter()
            .skip(range.start)
            .take(range.end - range.start)
            .collect())
    }
}
