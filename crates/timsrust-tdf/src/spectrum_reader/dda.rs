use timsrust_core::utils::vec::{argsort, group_and_sum};

use crate::{
    FrameReaderError, TdfFrameReader,
    file_readers::sql_reader::{
        ReadableSqlTable, SqlReader, SqlReaderError,
        pasef_frame_msms::SqlPasefFrameMsMs,
    },
};

use super::raw_spectra::{
    RawSpectrum, RawSpectrumReaderError, RawSpectrumReaderTrait,
};

#[derive(Debug)]
pub(crate) struct DDARawSpectrumReader {
    order: Vec<usize>,
    offsets: Vec<usize>,
    pasef_frames: Vec<SqlPasefFrameMsMs>,
    frame_reader: TdfFrameReader,
}

impl DDARawSpectrumReader {
    pub(crate) fn new(
        tdf_sql_reader: &SqlReader,
        frame_reader: TdfFrameReader,
    ) -> Result<Self, DDARawSpectrumReaderError> {
        let pasef_frames = SqlPasefFrameMsMs::from_sql_reader(tdf_sql_reader)?;
        let pasef_precursors =
            &pasef_frames.iter().map(|x| x.precursor).collect::<Vec<_>>();
        let order = argsort(pasef_precursors);
        let max_precursor = pasef_precursors
            .iter()
            .max()
            .expect("SqlReader cannot return empty vecs, so there is always a max precursor index");
        let mut offsets = Vec::with_capacity(max_precursor + 1);
        offsets.push(0);
        for (offset, &index) in order.iter().enumerate().take(order.len() - 1) {
            let second_index: usize = order[offset + 1];
            if pasef_precursors[index] != pasef_precursors[second_index] {
                offsets.push(offset + 1)
            }
        }
        offsets.push(order.len());
        let reader = Self {
            order,
            offsets,
            pasef_frames,
            frame_reader,
        };
        Ok(reader)
    }

    pub(crate) fn iterate_over_pasef_frames(
        &self,
        index: usize,
    ) -> impl Iterator<Item = &SqlPasefFrameMsMs> {
        let start: usize = self.offsets[index];
        let end: usize = self.offsets[index + 1];
        self.order[start..end]
            .iter()
            .map(|&x| &self.pasef_frames[x])
    }

    fn _get(
        &self,
        index: usize,
    ) -> Result<RawSpectrum, DDARawSpectrumReaderError> {
        let mut collision_energy = 0.0;
        let mut isolation_mz = 0.0;
        let mut isolation_width = 0.0;
        let mut tof_indices: Vec<u32> = vec![];
        let mut intensities: Vec<u32> = vec![];
        for pasef_frame in self.iterate_over_pasef_frames(index) {
            collision_energy = pasef_frame.collision_energy;
            isolation_mz = pasef_frame.isolation_mz;
            isolation_width = pasef_frame.isolation_width;
            let frame_index: usize = pasef_frame.frame;
            let frame = self.frame_reader.get_frame(frame_index).map_err(FrameReaderError::from)?;
            if frame.is_empty() {
                continue;
            }
            let scan_start: usize = pasef_frame.scan_start;
            let scan_end: usize = pasef_frame.scan_end;
            let scan_offsets = frame.ions().scan_offsets();
            let offset_start: usize = scan_offsets[scan_start];
            let offset_end: usize = scan_offsets[scan_end];
            let tof_selection = &frame.ions().tof_indices()
                [offset_start..offset_end]
                .iter()
                .map(|&x| x.into())
                .collect::<Vec<_>>();
            let intensity_selection = &frame.ions().intensities()
                [offset_start..offset_end]
                .iter()
                .map(|&x| x.into())
                .collect::<Vec<_>>();
            tof_indices.extend(tof_selection);
            intensities.extend(intensity_selection);
        }
        let (raw_tof_indices, raw_intensities) = group_and_sum(
            tof_indices,
            intensities.iter().map(|x| *x as u64).collect(),
        );
        let raw_spectrum = RawSpectrum {
            tof_indices: raw_tof_indices,
            intensities: raw_intensities,
            index,
            collision_energy,
            isolation_mz,
            isolation_width,
        };
        Ok(raw_spectrum)
    }
}

impl RawSpectrumReaderTrait for DDARawSpectrumReader {
    fn get(&self, index: usize) -> Result<RawSpectrum, RawSpectrumReaderError> {
        Ok(self._get(index)?)
    }

    fn len(&self) -> usize {
        self.offsets.len() - 1
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DDARawSpectrumReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
}
