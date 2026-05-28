use timsrust_core::io::formats::parquet::ParquetReader;
use timsrust_core::utils::simple_error;

#[derive(Debug)]
pub struct ParquetPrecursorReader {
    precursors: Vec<Precursor>,
}

impl ParquetPrecursorReader {
    pub fn new(ms1_path: impl AsRef<str>) -> Self {
        let precursors = ParquetReader::<Precursor>::from(ms1_path.as_ref())
            .unwrap()
            .read_all()
            .unwrap();
        Self { precursors }
    }

    pub fn len(&self) -> usize {
        self.precursors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize)]
pub struct Precursor {
    pub frame: u32,
    pub scan: u32,
    pub tof: u32,
    pub apex_intensity: u64,
    pub rt: f64,
    pub im: f64,
    pub mz: f64,
    pub start: u64,
    pub end: u64,
    pub charge: u8,
    pub index: u32,
    pub isolation_mz: f64,
    pub isolation_width: f64,
    pub ce: f64,
}

impl From<Precursor> for timsrust_core::Precursor {
    fn from(value: Precursor) -> timsrust_core::Precursor {
        timsrust_core::Precursor::new(
            timsrust_core::Mz::from(value.mz),
            timsrust_core::Im::from(value.im),
            timsrust_core::Rt::from(value.rt),
            timsrust_core::ScanIndex::try_from(value.scan).unwrap(),
            timsrust_core::Charge::try_from(value.charge).ok(),
            Some(value.apex_intensity as f64),
            value.index as usize,
            timsrust_core::FrameIndex::try_from(value.frame).unwrap(),
        )
    }
}

impl timsrust_core::utils::reader::IndexedReader<Precursor>
    for ParquetPrecursorReader
{
    type Iter = std::ops::Range<usize>;
    fn iter(&self) -> Self::Iter {
        0..self.len()
    }
}

impl timsrust_core::utils::reader::Reader<Precursor>
    for ParquetPrecursorReader
{
    type Error = ParquetPrecursorReaderError;

    fn get(&self, index: usize) -> Result<Precursor, Self::Error> {
        self.precursors
            .get(index)
            .cloned()
            .ok_or(ParquetPrecursorReaderError())
    }
}

impl timsrust_core::utils::reader::Reader<timsrust_core::Precursor>
    for ParquetPrecursorReader
{
    type Error = ParquetPrecursorReaderError;

    fn get(
        &self,
        index: usize,
    ) -> Result<timsrust_core::Precursor, Self::Error> {
        let precursor: Precursor = self.get(index)?;
        Ok(precursor.into())
    }
}

simple_error!(pub ParquetPrecursorReaderError);
