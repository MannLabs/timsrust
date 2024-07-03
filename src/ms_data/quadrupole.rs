/// The quadrupole settings used for fragmentation.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct QuadrupoleSettings {
    pub index: usize,
    pub scan_starts: Vec<usize>,
    pub scan_ends: Vec<usize>,
    pub isolation_mz: Vec<f64>,
    pub isolation_width: Vec<f64>,
    pub collision_energy: Vec<f64>,
}
