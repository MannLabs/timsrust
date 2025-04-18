use crate::{Metadata, QuadrupoleSettings};

use super::{
    file_readers::sql_reader::{SqlReader, SqlReaderError},
    FrameReader, FrameReaderError, MetadataReader, MetadataReaderError,
    PrecursorReader, PrecursorReaderError, QuadrupoleSettingsReader,
    QuadrupoleSettingsReaderError, SpectrumReader, SpectrumReaderError,
    TimsTofPath, TimsTofPathError, TimsTofPathLike,
};

pub struct TimsTofData {
    timstof_path: TimsTofPath,
    metadata: Metadata,
    sql_reader: SqlReader,
    frame_reader: Option<FrameReader>,
    spectrum_reader: Option<SpectrumReader>,
    precursor_reader: Option<PrecursorReader>,
    quad_settings: Option<Vec<QuadrupoleSettings>>,
}

impl TimsTofData {
    pub fn new(path: impl TimsTofPathLike) -> Result<Self, TimsTofDataError> {
        let timstof_path = TimsTofPath::new(&path)?;
        #[cfg(feature = "minitdf")]
        {
            use super::TimsTofFileType;
            if timstof_path.file_type() == TimsTofFileType::MiniTDF {
                return Err(TimsTofPathError::UnknownType(
                    path.as_ref().to_path_buf(),
                ))?;
            }
        }
        let sql_reader = SqlReader::new_from_path(&timstof_path)?;
        let metadata = MetadataReader::new_from_sql_reader(&sql_reader)?;
        Ok(Self {
            timstof_path,
            metadata,
            sql_reader,
            frame_reader: None,
            spectrum_reader: None,
            precursor_reader: None,
            quad_settings: None,
        })
    }

    pub fn get_timstof_path(&self) -> &TimsTofPath {
        &self.timstof_path
    }

    pub(crate) fn get_sql_reader(&self) -> &SqlReader {
        &self.sql_reader
    }

    pub fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn get_quad_settings(
        &mut self,
    ) -> Result<&Vec<QuadrupoleSettings>, QuadrupoleSettingsReaderError> {
        if self.quad_settings.is_none() {
            let quad_settings = QuadrupoleSettingsReader::from_sql_settings(
                &self.get_sql_reader(),
            )?;
            self.quad_settings = Some(quad_settings);
        }
        Ok(self.quad_settings.as_ref().expect("Always initialized"))
    }

    pub fn get_frame_reader(
        &mut self,
    ) -> Result<&FrameReader, FrameReaderError> {
        if self.frame_reader.is_none() {
            self.frame_reader = Some(FrameReader::new_from_timstofdata(self)?);
        }
        Ok(self.frame_reader.as_ref().expect("Always initialized"))
    }

    // TODO, reuse TimsTofData and allow bulder pattern
    pub fn get_precursor_reader(
        &mut self,
    ) -> Result<&PrecursorReader, PrecursorReaderError> {
        if self.precursor_reader.is_none() {
            self.precursor_reader =
                Some(PrecursorReader::new(&self.timstof_path)?);
        }
        Ok(self.precursor_reader.as_ref().expect("Always initialized"))
    }

    // TODO, reuse TimsTofData and allow bulder pattern
    pub fn get_spectrum_reader(
        &mut self,
    ) -> Result<&SpectrumReader, SpectrumReaderError> {
        if self.spectrum_reader.is_none() {
            self.spectrum_reader =
                Some(SpectrumReader::new(&self.timstof_path)?);
        }
        Ok(self.spectrum_reader.as_ref().expect("Always initialized"))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TimsTofDataError {
    #[error("{0}")]
    MetadataReaderError(#[from] MetadataReaderError),
    #[error("{0}")]
    TimsTofPathError(#[from] TimsTofPathError),
    #[error("{0}")]
    SqlReaderError(#[from] SqlReaderError),
}
