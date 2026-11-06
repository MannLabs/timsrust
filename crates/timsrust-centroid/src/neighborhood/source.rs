use timsrust_core::{
    Converter, FrameInfo, FrameIons, Im, Mz, ScanIndex, TofIndex,
    utils::reader::{IndexedReader, Reader},
};

use crate::{Peak, PeakReader, TimsError, TimsResult};

pub trait NativeSource {
    fn load_trajectory_candidates(
        &self,
        _frame: usize,
        _include_weak: bool,
    ) -> TimsResult<super::TransmissionFrame<super::MassTransmissionFootprint>>
    {
        Err(TimsError::new(
            "source does not expose trajectory transmission footprints",
        ))
    }

    fn load_transmission_candidates(
        &self,
        _frame: usize,
        _include_weak: bool,
    ) -> TimsResult<super::TransmissionFrame> {
        Err(TimsError::new(
            "source does not expose transmission footprints",
        ))
    }

    fn frame_infos(&self) -> TimsResult<Vec<FrameInfo>>;
    fn load_frame(&self, frame: usize) -> TimsResult<NativeFrame>;
    fn scan_fwhm(&self) -> usize;
    fn mobility(&self, scan: usize) -> TimsResult<f64>;
    fn mz(&self, tof: u32) -> TimsResult<f64>;
    fn load_rt_candidates(
        &self,
        _frame: usize,
    ) -> TimsResult<super::RtCandidateFrame> {
        Err(TimsError::new(
            "source does not support RT raw-candidate detection",
        ))
    }
}

pub struct NativeFrame {
    pub peaks: Vec<Peak>,
    pub scan_count: usize,
}

pub(super) fn validated_peaks(
    source: &impl NativeSource,
    info: &FrameInfo,
) -> TimsResult<Vec<Peak>> {
    validate_peaks(source.load_frame(info.index())?, info)
}

pub(super) fn validate_peaks(
    frame: NativeFrame,
    info: &FrameInfo,
) -> TimsResult<Vec<Peak>> {
    if info
        .quadrupole_settings()
        .scan_ends
        .iter()
        .any(|&end| end > frame.scan_count)
    {
        return Err(TimsError::new(
            "acquisition interval exceeds actual native scan count",
        ));
    }
    let mut peaks = frame.peaks;
    if peaks.iter().any(|p| {
        p.frame as usize != info.index()
            || p.apex_intensity == 0
            || p.scan as usize >= frame.scan_count
    }) {
        return Err(TimsError::new(
            "native source returned invalid frame/response",
        ));
    }
    peaks.sort_by_key(|p| (p.scan, p.tof));
    if peaks
        .windows(2)
        .any(|p| p[0].scan == p[1].scan && p[0].tof == p[1].tof)
    {
        return Err(TimsError::new(
            "native source returned duplicate peak coordinates",
        ));
    }
    Ok(peaks)
}

pub struct CentroidSource<IR, InfoR, IM, MZ> {
    reader: PeakReader<IR, InfoR>,
    im: IM,
    mz: MZ,
}

impl<IR, InfoR, IM, MZ> CentroidSource<IR, InfoR, IM, MZ> {
    pub fn new(reader: PeakReader<IR, InfoR>, im: IM, mz: MZ) -> Self {
        Self { reader, im, mz }
    }
}

impl<IR, InfoR, IM, MZ> NativeSource for CentroidSource<IR, InfoR, IM, MZ>
where
    IR: Reader<FrameIons> + Sync + Send,
    InfoR: Reader<FrameInfo> + IndexedReader<FrameInfo> + Sync + Send,
    IM: Converter<ScanIndex, Im>,
    MZ: Converter<TofIndex, Mz>,
{
    fn frame_infos(&self) -> TimsResult<Vec<FrameInfo>> {
        self.reader
            .frame_reader()
            .iter_indices()
            .map(|id| {
                self.reader
                    .frame_reader()
                    .get_info(id)
                    .map_err(|e| TimsError::new(e.to_string()))
            })
            .collect()
    }

    fn load_frame(&self, frame: usize) -> TimsResult<NativeFrame> {
        let frame = self
            .reader
            .frame_reader()
            .get_frame(frame)
            .map_err(|e| TimsError::new(e.to_string()))?;
        let scan_count = frame
            .ions()
            .scan_offsets()
            .len()
            .checked_sub(1)
            .ok_or_else(|| TimsError::new("native frame lacks scan offsets"))?;
        Ok(NativeFrame {
            peaks: self.reader.centroid_frame(&frame)?,
            scan_count,
        })
    }

    fn scan_fwhm(&self) -> usize {
        self.reader.scan_fwhm()
    }

    fn load_rt_candidates(
        &self,
        frame: usize,
    ) -> TimsResult<super::RtCandidateFrame> {
        let loaded = self
            .reader
            .frame_reader()
            .get_frame(frame)
            .map_err(|e| TimsError::new(e.to_string()))?;
        self.reader.rt_consistency_frame(&loaded)
    }

    fn load_transmission_candidates(
        &self,
        frame: usize,
        include_weak: bool,
    ) -> TimsResult<super::TransmissionFrame> {
        let loaded = self
            .reader
            .frame_reader()
            .get_frame(frame)
            .map_err(|e| TimsError::new(e.to_string()))?;
        self.reader.transmission_candidates(&loaded, include_weak)
    }

    fn load_trajectory_candidates(
        &self,
        frame: usize,
        include_weak: bool,
    ) -> TimsResult<super::TransmissionFrame<super::MassTransmissionFootprint>>
    {
        let loaded = self
            .reader
            .frame_reader()
            .get_frame(frame)
            .map_err(|e| TimsError::new(e.to_string()))?;
        self.reader.trajectory_candidates(&loaded, include_weak)
    }

    fn mobility(&self, scan: usize) -> TimsResult<f64> {
        let scan = ScanIndex::try_from(scan)
            .map_err(|e| TimsError::new(e.to_string()))?;
        Ok(f64::from(self.im.convert(scan)))
    }

    fn mz(&self, tof: u32) -> TimsResult<f64> {
        let tof = TofIndex::try_from(tof)
            .map_err(|e| TimsError::new(e.to_string()))?;
        Ok(f64::from(self.mz.convert(tof)))
    }
}
