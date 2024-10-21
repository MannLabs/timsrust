use std::hash::{Hash, Hasher};

/// The quadrupole settings used for fragmentation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadrupoleSettings {
    pub index: usize,
    pub scan_starts: Vec<usize>,
    pub scan_ends: Vec<usize>,
    pub isolation_mz: Vec<f64>,
    pub isolation_width: Vec<f64>,
    pub collision_energy: Vec<f64>,
}

impl Hash for QuadrupoleSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.scan_starts.hash(state);
        self.scan_ends.hash(state);
        for mz in &self.isolation_mz {
            mz.to_bits().hash(state);
        }
        for width in &self.isolation_width {
            width.to_bits().hash(state);
        }
        for energy in &self.collision_energy {
            energy.to_bits().hash(state);
        }
    }
}

impl QuadrupoleSettings {
    pub fn len(&self) -> usize {
        self.isolation_mz.len()
    }
}
