/// The quadrupole settings used for fragmentation.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct QuadrupoleSettings {
    is_used: bool,
    scan_starts: Vec<u16>,
    scan_ends: Vec<u16>,
    isolation_mz: Vec<f32>,
    isolation_width: Vec<f32>,
    collision_energy: Vec<f32>,
}
