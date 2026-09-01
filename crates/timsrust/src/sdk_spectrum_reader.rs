use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use timsrust_core::io::formats::sql::SqlReader;
use timsrust_core::utils::reader::{IndexedReader, Reader};
use timsrust_core::utils::thread::Synced;
use timsrust_core::{
    BitConverter, Converter, IsolationWindow, Mz, Spectrum, TofIndex,
};
use timsrust_sdk::{PressureCompensationStrategy, TimsData};

use crate::{PrecursorReader, PrecursorReaderError, TimsTofPathLike};

/// Lazy DDA-PASEF spectrum reader backed by the Bruker SDK
/// (`tims_read_pasef_msms_v2`).
///
/// Precursor metadata is supplied by the TDF precursor reader; the SDK returns
/// calibrated centroided m/z, which is bit-encoded into `TofIndex` and
/// recovered downstream with a [`BitConverter`].
pub struct SdkSpectrumReader {
    sdk: Synced<TimsData>,
    precursor_reader: PrecursorReader,
    isolation_windows: HashMap<usize, IsolationWindow>,
}

impl SdkSpectrumReader {
    /// `tdf_file` is the path to `analysis.tdf` (used to open the SDK handle
    /// and read isolation windows); `dataset_path` is the `.d` dataset (used to
    /// build the precursor reader).
    pub fn new(
        tdf_file: impl Into<PathBuf>,
        dataset_path: impl TimsTofPathLike,
    ) -> Result<Self, SdkSpectrumReaderError> {
        let tdf_file = tdf_file.into();
        let sdk = TimsData::new(
            tdf_file.clone(),
            false,
            PressureCompensationStrategy::default(),
        );
        let precursor_reader = PrecursorReader::new(dataset_path)?;
        let isolation_windows = read_isolation_windows(&tdf_file);
        Ok(Self {
            sdk: Synced::from(sdk),
            precursor_reader,
            isolation_windows,
        })
    }

    pub fn len(&self) -> usize {
        self.precursor_reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Reader<Spectrum> for SdkSpectrumReader {
    type Error = SdkSpectrumReaderError;

    fn get(&self, index: usize) -> Result<Spectrum, Self::Error> {
        let precursor = self.precursor_reader.get(index)?;
        let id = precursor.index() as i64;
        let (mz_values, area_values) = self
            .sdk
            .with_lock(|sdk| sdk.read_pasef_msms(&[id]))
            .map_err(|_| SdkSpectrumReaderError::Lock)?
            .remove(&id)
            .unwrap_or_default();
        let bit = BitConverter();
        let coordinates: Vec<TofIndex> = mz_values
            .iter()
            .map(|&mz| bit.convert(Mz::from(mz)))
            .collect();
        let intensities: Vec<f64> =
            area_values.iter().map(|&area| area as f64).collect();
        let isolation_window = self
            .isolation_windows
            .get(&precursor.index())
            .cloned()
            .unwrap_or_default();
        Ok(Spectrum::new(
            intensities,
            precursor.index(),
            Some(precursor),
            coordinates,
            isolation_window,
        ))
    }
}

impl IndexedReader<Spectrum> for SdkSpectrumReader {
    type Iter = std::ops::Range<usize>;

    fn iter(&self) -> Self::Iter {
        0..self.len()
    }
}

/// Builds a `precursor id -> isolation window` map from `PasefFrameMsMsInfo`.
/// Best-effort: returns an empty map (default windows) if the table is absent.
fn read_isolation_windows(tdf_file: &Path) -> HashMap<usize, IsolationWindow> {
    #[derive(Deserialize)]
    struct Row {
        #[serde(rename = "Precursor")]
        precursor: usize,
        #[serde(rename = "IsolationMz")]
        isolation_mz: f64,
        #[serde(rename = "IsolationWidth")]
        isolation_width: f64,
        #[serde(rename = "CollisionEnergy")]
        collision_energy: f64,
    }

    let Some(path) = tdf_file.to_str() else {
        return HashMap::new();
    };
    let rows = SqlReader::from(path)
        .ok()
        .and_then(|reader| reader.from_table::<Row>("PasefFrameMsMsInfo").ok())
        .and_then(|table| table.read_all().ok())
        .unwrap_or_default();

    let mut windows = HashMap::new();
    for row in rows {
        windows.insert(
            row.precursor,
            IsolationWindow::new_from_center(
                Mz::from(row.isolation_mz),
                Mz::from(row.isolation_width),
                row.collision_energy,
            ),
        );
    }
    windows
}

#[derive(Debug, thiserror::Error)]
pub enum SdkSpectrumReaderError {
    #[error("{0}")]
    Precursor(#[from] PrecursorReaderError),
    #[error("SDK reader mutex was poisoned")]
    Lock,
}
