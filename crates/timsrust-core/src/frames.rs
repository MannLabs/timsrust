use rayon::iter::IntoParallelIterator;
use timsrust_utils::{
    custom_error,
    reader::{IndexedReader, Reader},
};
// use timsrust_utils::reader::{IndexedReader, Reader};

use crate::{Intensity, IntensityIndex, Mz, TofIndex, coordinates::Converter};

use super::{AcquisitionType, QuadrupoleSettings};
use rayon::prelude::*;
use std::sync::Arc;

/// A frame with all unprocessed data as it was acquired.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    ions: FrameIons,
    info: FrameInfo,
}

impl Frame {
    pub fn get_corrected_intensity(&self, index: usize) -> f64 {
        let v = u32::from(self.ions.intensities[index]) as f64;
        self.info.intensity_correction_factor * v
    }

    pub fn info(&self) -> &FrameInfo {
        &self.info
    }

    pub fn ions(&self) -> &FrameIons {
        &self.ions
    }

    pub fn set_data(
        &mut self,
        scan_offsets: Vec<usize>,
        tof_indices: Vec<TofIndex>,
        intensities: Vec<IntensityIndex>,
    ) {
        self.ions = FrameIons::new(scan_offsets, tof_indices, intensities);
    }

    pub fn set_ions(&mut self, ions: FrameIons) {
        self.ions = ions;
    }

    pub fn set_info(&mut self, info: FrameInfo) {
        self.info = info;
    }

    pub fn len(&self) -> usize {
        self.ions.intensities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ions.is_empty()
    }

    pub fn index(&self) -> usize {
        self.info().index
    }
}

/// The MS level used.
#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum MSLevel {
    MS1,
    MS2,
    /// Default value.
    #[default]
    Unknown,
}

impl MSLevel {
    pub fn read_from_msms_type(msms_type: u8) -> MSLevel {
        match msms_type {
            0 => MSLevel::MS1,
            8 => MSLevel::MS2,
            9 => MSLevel::MS2,
            _ => MSLevel::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FrameIons {
    scan_offsets: Vec<usize>,
    tof_indices: Vec<TofIndex>,
    intensities: Vec<IntensityIndex>,
}

impl FrameIons {
    pub fn new(
        scan_offsets: Vec<usize>,
        tof_indices: Vec<TofIndex>,
        intensities: Vec<IntensityIndex>,
    ) -> Self {
        Self {
            scan_offsets,
            tof_indices,
            intensities,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.intensities.is_empty()
    }

    pub fn scan_offsets(&self) -> &Vec<usize> {
        &self.scan_offsets
    }

    pub fn tof_indices(&self) -> &Vec<TofIndex> {
        &self.tof_indices
    }

    pub fn mz_values<C: Converter<TofIndex, Mz>>(
        &self,
        converter: &C,
    ) -> Vec<Mz> {
        converter.batch_convert(&self.tof_indices)
    }

    pub fn intensities(&self) -> &Vec<IntensityIndex> {
        &self.intensities
    }

    pub fn intensity_values<C: Converter<IntensityIndex, Intensity>>(
        &self,
        converter: &C,
    ) -> Vec<Intensity> {
        converter.batch_convert(&self.intensities)
    }

    pub fn read_scan(
        &self,
        scan_index: usize,
    ) -> impl Iterator<Item = (TofIndex, IntensityIndex)> + '_ {
        let (start, end) = if scan_index >= self.scan_count() {
            (0, 0)
        } else {
            (
                self.scan_offsets[scan_index],
                self.scan_offsets[scan_index + 1],
            )
        };
        (start..end).map(move |index| {
            (self.tof_indices[index], self.intensities[index])
        })
    }

    pub fn scan_count(&self) -> usize {
        self.scan_offsets.len() - 1
    }

    pub fn add_info(self, info: FrameInfo) -> Frame {
        Frame { ions: self, info }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameInfo {
    quadrupole_settings: Arc<QuadrupoleSettings>,
    index: usize,
    rt_in_seconds: f64,
    intensity_correction_factor: f64,
    acquisition_type: AcquisitionType,
    ms_level: MSLevel,
    window_group: u8,
    cycle_index: Option<usize>,
}

impl FrameInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        quadrupole_settings: Arc<QuadrupoleSettings>,
        index: usize,
        rt_in_seconds: f64,
        intensity_correction_factor: f64,
        acquisition_type: AcquisitionType,
        ms_level: MSLevel,
        window_group: u8,
        cycle_index: Option<usize>,
    ) -> Self {
        Self {
            quadrupole_settings,
            index,
            rt_in_seconds,
            intensity_correction_factor,
            acquisition_type,
            ms_level,
            window_group,
            cycle_index,
        }
    }

    pub fn quadrupole_settings(&self) -> &Arc<QuadrupoleSettings> {
        &self.quadrupole_settings
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn cycle_index(&self) -> Option<usize> {
        self.cycle_index
    }

    pub fn rt_in_seconds(&self) -> f64 {
        self.rt_in_seconds
    }

    pub fn intensity_correction_factor(&self) -> f64 {
        self.intensity_correction_factor
    }

    pub fn acquisition_type(&self) -> AcquisitionType {
        self.acquisition_type
    }

    pub fn ms_level(&self) -> MSLevel {
        self.ms_level
    }

    pub fn window_group(&self) -> u8 {
        self.window_group
    }

    pub fn add_ions(self, ions: FrameIons) -> Frame {
        Frame { ions, info: self }
    }
}
#[derive(Debug)]
pub struct FrameReader<IonReader, InfoReader> {
    ion_reader: IonReader,
    info_reader: InfoReader,
}

impl<IonReader, InfoReader> FrameReader<IonReader, InfoReader>
where
    IonReader: Reader<FrameIons>,
    InfoReader: Reader<FrameInfo> + IndexedReader<FrameInfo>,
{
    pub fn new(ion_reader: IonReader, info_reader: InfoReader) -> Self {
        Self {
            ion_reader,
            info_reader,
        }
    }

    pub fn ion_reader(&self) -> &IonReader {
        &self.ion_reader
    }

    pub fn info_reader(&self) -> &InfoReader {
        &self.info_reader
    }

    pub fn ion_reader_index(
        &self,
        index: usize,
    ) -> Result<usize, FrameReaderError> {
        Ok(index)
    }

    pub fn info_reader_index(
        &self,
        index: usize,
    ) -> Result<usize, FrameReaderError> {
        Ok(index)
    }

    pub fn get_ions(
        &self,
        index: usize,
    ) -> Result<FrameIons, FrameReaderError> {
        let index = self.ion_reader_index(index)?;
        let result = self
            .ion_reader()
            .get(index)
            .map_err(|e| FrameReaderError::new(e.to_string()))?;
        Ok(result)
    }

    pub fn get_info(
        &self,
        index: usize,
    ) -> Result<FrameInfo, FrameReaderError> {
        let index = self.info_reader_index(index)?;
        let result = self
            .info_reader()
            .get(index)
            .map_err(|e| FrameReaderError::new(e.to_string()))?;
        Ok(result)
    }

    pub fn get_frame(&self, index: usize) -> Result<Frame, FrameReaderError> {
        let info = self.get_info(index)?;
        let ions = self.get_ions(index)?;
        let mut frame = Frame::default();
        frame.set_ions(ions);
        frame.set_info(info);
        Ok(frame)
    }

    /// Return a [`Frame`] populated with only [`FrameInfo`] (no ion data loaded).
    pub fn get_partial_frame_without_ions(
        &self,
        index: usize,
    ) -> Result<Frame, FrameReaderError> {
        let info = self.get_info(index)?;
        let mut frame = Frame::default();
        frame.set_info(info);
        Ok(frame)
    }

    pub fn len(&self) -> usize {
        self.iter_indices().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // pub fn size(&self) -> Option<usize> {
    //     self.info_reader().size()
    // }

    pub fn iter_indices(&self) -> impl Iterator<Item = usize> {
        self.info_reader().iter()
    }

    pub fn filter<'a, F: Fn(&Frame) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl Iterator<Item = Result<Frame, FrameReaderError>> {
        self.iter_indices().filter_map(move |x| {
            match self.info_reader().get(x) {
                Ok(frame_info) => {
                    let mut partial_frame = Frame::default();
                    partial_frame.set_info(frame_info);
                    if predicate(&partial_frame) {
                        let ions = self.get_ions(x).unwrap();
                        partial_frame.set_ions(ions);
                        Some(Ok(partial_frame))
                    } else {
                        None
                    }
                },
                Err(e) => Some(Err(FrameReaderError::new(e.to_string()))),
            }
        })
    }

    pub fn parallel_filter<'a, F: Fn(&Frame) -> bool + Sync + Send + 'a>(
        &'a self,
        predicate: F,
    ) -> impl ParallelIterator<Item = Result<Frame, FrameReaderError>> + 'a
    where
        Self: Sync + Send + 'a,
    {
        self.iter_indices()
            .collect::<Vec<_>>()
            .into_par_iter()
            .filter_map(move |x| match self.info_reader().get(x) {
                Ok(frame_info) => {
                    let mut partial_frame = Frame::default();
                    partial_frame.set_info(frame_info);
                    if predicate(&partial_frame) {
                        let ions = self.get_ions(x).unwrap();
                        partial_frame.set_ions(ions);
                        Some(Ok(partial_frame))
                    } else {
                        None
                    }
                },
                Err(e) => Some(Err(FrameReaderError::new(e.to_string()))),
            })
    }
}

custom_error!(pub FrameReaderError);
