use crate::{Charge, FrameIndex, Im, Mz, Rt, ScanIndex};

/// The MS1 precursor that got selected for fragmentation.
#[derive(Clone, Debug, PartialEq)]
pub struct Precursor {
    mz: Mz,
    rt: Rt,
    im: Im,
    scan_index: ScanIndex,
    charge: Option<Charge>,
    intensity: Option<f64>,
    index: usize,
    frame_index: FrameIndex,
}

impl Precursor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mz: Mz,
        im: Im,
        rt: Rt,
        scan_index: ScanIndex,
        charge: Option<Charge>,
        intensity: Option<f64>,
        index: usize,
        frame_index: FrameIndex,
    ) -> Self {
        Precursor {
            mz,
            rt,
            im,
            scan_index,
            charge,
            intensity,
            index,
            frame_index,
        }
    }
}

impl Precursor {
    pub fn mz(&self) -> Mz {
        self.mz
    }

    pub fn rt(&self) -> Rt {
        self.rt
    }

    pub fn im(&self) -> Im {
        self.im
    }

    pub fn scan_index(&self) -> ScanIndex {
        self.scan_index
    }

    pub fn charge(&self) -> &Option<Charge> {
        &self.charge
    }

    pub fn intensity(&self) -> &Option<f64> {
        &self.intensity
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn frame_index(&self) -> FrameIndex {
        self.frame_index
    }
}
