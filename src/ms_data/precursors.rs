/// The MS1 precursor that got selected for fragmentation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Precursor {
    pub mz: f64,
    pub rt: f64,
    pub im: f64,
    pub charge: Option<usize>,
    pub intensity: Option<f64>,
    pub index: usize,
    pub frame_index: usize,
}
