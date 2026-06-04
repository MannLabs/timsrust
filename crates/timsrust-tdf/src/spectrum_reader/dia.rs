use timsrust_core::utils::vec::group_and_sum;
use timsrust_core::{Im, InvertibleConverter, QuadrupoleSettings, ScanIndex};

use crate::TdfFrameReader;
use crate::{
    FrameReaderError, QuadrupoleSettingsReader, QuadrupoleSettingsReaderError,
    file_readers::sql_reader::{SqlReader, SqlReaderError},
    quad_settings_reader::FrameWindowSplittingStrategy,
    spectrum_reader::raw_spectra::{
        RawSpectrum, RawSpectrumReaderError, RawSpectrumReaderTrait,
    },
};

#[derive(Debug)]
pub(crate) struct DIARawSpectrumReader {
    expanded_quadrupole_settings: Vec<QuadrupoleSettings>,
    frame_reader: TdfFrameReader,
}

impl DIARawSpectrumReader {
    pub(crate) fn new<ImC: InvertibleConverter<ScanIndex, Im>>(
        tdf_sql_reader: &SqlReader,
        frame_reader: TdfFrameReader,
        splitting_strategy: FrameWindowSplittingStrategy<ImC>,
    ) -> Result<Self, DIARawSpectrumReaderError> {
        let expanded_quadrupole_settings =
            QuadrupoleSettingsReader::from_splitting(
                tdf_sql_reader,
                splitting_strategy,
            )?;
        let reader = Self {
            expanded_quadrupole_settings,
            frame_reader,
        };
        Ok(reader)
    }

    fn _get(
        &self,
        index: usize,
    ) -> Result<RawSpectrum, DIARawSpectrumReaderError> {
        let quad_settings = &self.expanded_quadrupole_settings[index];

        let collision_energy =
            quad_settings.isolation_windows[0].collision_energy();
        let isolation_mz = quad_settings.isolation_windows[0].center();
        let isolation_width = quad_settings.isolation_windows[0].width();
        let scan_start = quad_settings.scan_starts[0];
        let scan_end = quad_settings.scan_ends[0];
        let frame_index = quad_settings.index;
        let frame = self
            .frame_reader
            .get_frame(frame_index)
            .map_err(FrameReaderError::from)?;
        let scan_offsets = frame.ions().scan_offsets();
        let offset_start = scan_offsets[scan_start];
        let offset_end = scan_offsets[scan_end];
        let tof_indices = &frame.ions().tof_indices()[offset_start..offset_end];
        let intensities = &frame.ions().intensities()[offset_start..offset_end];
        let (raw_tof_indices, raw_intensities) = group_and_sum(
            tof_indices.to_vec(),
            intensities.iter().map(|&x| u64::from(x)).collect(),
        );
        let raw_spectrum = RawSpectrum {
            tof_indices: raw_tof_indices.iter().map(|&x| x.into()).collect(),
            intensities: raw_intensities,
            index,
            collision_energy,
            isolation_mz: f64::from(isolation_mz),
            isolation_width: f64::from(isolation_width),
        };
        Ok(raw_spectrum)
    }
}

impl RawSpectrumReaderTrait for DIARawSpectrumReader {
    fn get(&self, index: usize) -> Result<RawSpectrum, RawSpectrumReaderError> {
        Ok(self._get(index)?)
    }

    fn len(&self) -> usize {
        self.expanded_quadrupole_settings.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DIARawSpectrumReaderError {
    #[error("{0}")]
    Sql(#[from] SqlReaderError),
    #[error("{0}")]
    QuadrupoleSettings(#[from] QuadrupoleSettingsReaderError),
    #[error("{0}")]
    Frame(#[from] FrameReaderError),
}
