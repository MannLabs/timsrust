use crate::Spectrum;

use super::errors::SpectrumReaderError;

pub(crate) trait SpectrumReaderTrait: Sync + Send {
    fn get(&self, index: usize) -> Result<Spectrum, SpectrumReaderError>;
    fn len(&self) -> usize;
    fn calibrate(&mut self);
}
