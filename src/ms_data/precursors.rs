/// The MS1 precursor that got selected for fragmentation.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Precursor {
    pub mz: f64,
    pub rt: f64,
    pub im: f64,
    pub charge: usize,
    pub intensity: f64,
    pub index: usize,
    pub frame_index: usize,
}
