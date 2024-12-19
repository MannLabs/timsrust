use crate::{
    io::readers::{
        file_readers::sql_reader::{
            pasef_frame_msms::SqlPasefFrameMsMs, ReadableSqlTable, SqlReader,
            SqlReaderError,
        },
        FrameReader, FrameReaderError,
    },
    utils::vec_utils::{argsort, group_and_sum},
};

use super::raw_spectra::{
    RawSpectrum, RawSpectrumReaderError, RawSpectrumReaderTrait,
};

#[derive(Debug)]
pub struct DDARawSpectrumReader {
    order: Vec<usize>,
    offsets: Vec<usize>,
    pasef_frames: Vec<SqlPasefFrameMsMs>,
    frame_reader: FrameReader,
}

impl DDARawSpectrumReader {
    pub fn new(
        tdf_sql_reader: &SqlReader,
        frame_reader: FrameReader,
    ) -> Result<Self, DDARawSpectrumReaderError> {
        let pasef_frames = SqlPasefFrameMsMs::from_sql_reader(&tdf_sql_reader)?;
        let pasef_precursors =
            &pasef_frames.iter().map(|x| x.precursor).collect();
        let order: Vec<usize> = argsort(&pasef_precursors);
        let max_precursor = pasef_precursors
            .iter()
            .max()
            .expect("SqlReader cannot return empty vecs, so there is always a max precursor index");
        let mut offsets: Vec<usize> = Vec::with_capacity(max_precursor + 1);
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

    pub fn iterate_over_pasef_frames(
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
            let frame_index: usize = pasef_frame.frame - 1;
            let frame = self.frame_reader.get(frame_index)?;
            if frame.intensities.len() == 0 {
                continue;
            }
            let scan_start: usize = pasef_frame.scan_start;
            let scan_end: usize = pasef_frame.scan_end;
            let offset_start: usize = frame.scan_offsets[scan_start] as usize;
            let offset_end: usize = frame.scan_offsets[scan_end] as usize;
            let tof_selection: &[u32] =
                &frame.tof_indices[offset_start..offset_end];
            let intensity_selection: &[u32] =
                &frame.intensities[offset_start..offset_end];
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
            index: index,
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
pub enum DDARawSpectrumReaderError {
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
    #[error("{0}")]
    FrameReaderError(#[from] FrameReaderError),
}
