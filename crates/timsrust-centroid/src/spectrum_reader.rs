pub mod narrow_spectrum_reader;
pub mod wide_spectrum_reader;

use std::sync::Arc;

use rayon::prelude::*;
use timsrust_core::{
    Converter, FrameInfo, FrameIons, FrameReader, Im, InvertibleConverter, Mz,
    QuadrupoleSettings, ScanIndex, Spectrum, TofIndex,
};

pub use narrow_spectrum_reader::NarrowSpectrumReader;
use timsrust_core::utils::reader::{IndexedReader, ParIterableReader};
pub use wide_spectrum_reader::WideSpectrumReader;

use crate::{PeakReader, error::TimsResult};

/// Restricts scan-sorted peaks to the acquisition interval containing `scan`.
///
/// The returned range indexes `peaks`. An uncovered representative scan or
/// an interval without peaks returns `None`; neighboring windows are not used.
pub fn isolated_peak_range(
    peaks: &[crate::Peak],
    settings: &QuadrupoleSettings,
    scan: usize,
) -> Option<std::ops::Range<usize>> {
    assert!(peaks.is_sorted_by(|a, b| a.scan <= b.scan));
    let index = settings.get_isolation_window_index(scan)?;
    let start = settings.scan_starts[index];
    let end = settings.scan_ends[index];
    let lower = peaks.partition_point(|peak| (peak.scan as usize) < start);
    let upper = peaks.partition_point(|peak| (peak.scan as usize) < end);
    (lower < upper).then_some(lower..upper)
}

/// Reads and extracts spectra from frames.
pub enum SpectrumReader<
    IonReader,
    InfoReader,
    IM: InvertibleConverter<ScanIndex, Im>,
    MZ: Converter<TofIndex, Mz>,
> {
    Narrow(NarrowSpectrumReader<IonReader, InfoReader, IM, MZ>),
    Wide(WideSpectrumReader<IonReader, InfoReader, IM>),
}

impl<IonReader, InfoReader, IM, MZ>
    SpectrumReader<IonReader, InfoReader, IM, MZ>
where
    IonReader: timsrust_core::utils::reader::Reader<FrameIons> + Sync + Send,
    InfoReader: timsrust_core::utils::reader::Reader<FrameInfo>
        + IndexedReader<FrameInfo>
        + Sync
        + Send,
    IM: InvertibleConverter<ScanIndex, Im> + Sync + Send,
    MZ: Converter<TofIndex, Mz> + Sync + Send,
{
    /// Create a new [`SpectrumReader`] from a [`FrameReader`] and processing parameters.
    ///
    /// # Errors
    /// Returns an error if kernel extraction fails.
    pub fn new(
        frame_reader: FrameReader<IonReader, InfoReader>,
        min_ms1_ion_count: f64,
        min_ms2_ion_count: f64,
        min_spectrum_size: usize,
        use_precursors: bool,
        im_converter: IM,
        mz_converter: MZ,
    ) -> TimsResult<Self> {
        let spectrum_reader = if use_precursors {
            let peak_reader = PeakReader::new(
                frame_reader,
                min_ms1_ion_count,
                min_ms2_ion_count,
            )?;
            let reader = NarrowSpectrumReader::new(
                peak_reader,
                min_spectrum_size,
                Arc::new(im_converter),
                Arc::new(mz_converter),
            )?;
            SpectrumReader::Narrow(reader)
        } else {
            let peak_reader =
                PeakReader::new(frame_reader, -1.0, min_ms2_ion_count)?;
            let reader = WideSpectrumReader::new(
                peak_reader,
                min_spectrum_size,
                Arc::new(im_converter),
            )?;
            SpectrumReader::Wide(reader)
        };
        Ok(spectrum_reader)
    }

    /// Returns the number of spectra processed so far.
    pub fn len(&self) -> usize {
        match self {
            SpectrumReader::Narrow(reader) => reader.len(),
            SpectrumReader::Wide(reader) => reader.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extract all MS2 spectra from a given frame index.
    pub fn get_spectra_from_frame(&self, frame_index: usize) -> Vec<Spectrum> {
        match self {
            SpectrumReader::Narrow(reader) => {
                reader.get_spectra_from_frame(frame_index)
            },
            SpectrumReader::Wide(reader) => {
                reader.get_spectra_from_frame(frame_index)
            },
        }
    }

    pub fn frame_count(&self) -> usize {
        match self {
            SpectrumReader::Narrow(reader) => reader.frame_count(),
            SpectrumReader::Wide(reader) => reader.frame_count(),
        }
    }

    pub fn tof_fwhm(&self) -> usize {
        match self {
            SpectrumReader::Narrow(reader) => reader.tof_fwhm(),
            SpectrumReader::Wide(reader) => reader.tof_fwhm(),
        }
    }

    pub fn scan_fwhm(&self) -> usize {
        match self {
            SpectrumReader::Narrow(reader) => reader.scan_fwhm(),
            SpectrumReader::Wide(reader) => reader.scan_fwhm(),
        }
    }
}

impl<'a, IonReader, InfoReader, IM, MZ> ParIterableReader<'a, Spectrum>
    for SpectrumReader<IonReader, InfoReader, IM, MZ>
where
    IonReader: timsrust_core::utils::reader::Reader<FrameIons> + Sync + Send,
    InfoReader: timsrust_core::utils::reader::Reader<FrameInfo>
        + IndexedReader<FrameInfo>
        + Sync
        + Send,
    IM: InvertibleConverter<ScanIndex, Im> + Sync + Send,
    MZ: Converter<TofIndex, Mz> + Sync + Send,
{
    type Error = TimsCentroidError;

    fn par_iter(
        &'a self,
    ) -> impl ParallelIterator<Item = Result<Spectrum, Self::Error>> {
        match self {
            SpectrumReader::Narrow(reader) => A::Narrow(reader),
            SpectrumReader::Wide(reader) => A::Wide(reader),
        }
        .map(Ok)
    }
}

enum A<
    'a,
    IonReader,
    InfoReader,
    IM: InvertibleConverter<ScanIndex, Im>,
    MZ: Converter<TofIndex, Mz>,
> {
    Narrow(&'a NarrowSpectrumReader<IonReader, InfoReader, IM, MZ>),
    Wide(&'a WideSpectrumReader<IonReader, InfoReader, IM>),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TimsCentroidError(String);

impl<
    'a,
    IonReader: timsrust_core::utils::reader::Reader<FrameIons> + Sync + Send,
    InfoReader: timsrust_core::utils::reader::Reader<FrameInfo>
        + IndexedReader<FrameInfo>
        + Sync
        + Send,
    IM: InvertibleConverter<ScanIndex, Im> + Sync + Send,
    MZ: Converter<TofIndex, Mz> + Sync + Send,
> ParallelIterator for A<'a, IonReader, InfoReader, IM, MZ>
{
    type Item = timsrust_core::Spectrum;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        match self {
            Self::Narrow(reader) => {
                reader._par_iter().drive_unindexed(consumer)
            },
            Self::Wide(reader) => reader._par_iter().drive_unindexed(consumer),
        }
    }
}

impl<
    IonReader: timsrust_core::utils::reader::Reader<FrameIons> + Sync + Send,
    InfoReader: timsrust_core::utils::reader::Reader<FrameInfo>
        + IndexedReader<FrameInfo>
        + Sync
        + Send,
    IM: InvertibleConverter<ScanIndex, Im> + Sync + Send,
    MZ: Converter<TofIndex, Mz> + Sync + Send,
> ParallelIterator for SpectrumReader<IonReader, InfoReader, IM, MZ>
{
    type Item = timsrust_core::Spectrum;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        match self {
            Self::Narrow(reader) => {
                reader._par_iter().drive_unindexed(consumer)
            },
            Self::Wide(reader) => reader._par_iter().drive_unindexed(consumer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Peak;
    use timsrust_core::IsolationWindow;

    #[test]
    fn isolated_ranges_preserve_source_offsets_and_interval_identity() {
        let window = IsolationWindow::new_from_center(
            Mz::from(500.0),
            Mz::from(20.0),
            30.0,
        );
        let settings = QuadrupoleSettings {
            index: 1,
            scan_starts: vec![10, 30],
            scan_ends: vec![20, 40],
            isolation_windows: vec![window.clone(), window],
        };
        let peaks: Vec<_> = [9, 10, 19, 20, 29, 30, 39, 40]
            .into_iter()
            .map(|scan| Peak {
                scan,
                ..Peak::default()
            })
            .collect();
        assert_eq!(isolated_peak_range(&peaks, &settings, 19), Some(1..3));
        assert_eq!(isolated_peak_range(&peaks, &settings, 30), Some(5..7));
        for scan in [9, 20, 25, 40] {
            assert_eq!(isolated_peak_range(&peaks, &settings, scan), None);
        }
        assert_eq!(isolated_peak_range(&peaks[5..7], &settings, 19), None);
        assert_eq!(isolated_peak_range(&[], &settings, 19), None);
        let original_start = 1;
        let clipped = isolated_peak_range(&peaks[1..7], &settings, 30).unwrap();
        let fragment_offset = 1000;
        assert_eq!(
            (fragment_offset + original_start + clipped.start)
                ..(fragment_offset + original_start + clipped.end),
            1005..1007
        );
    }
}
