use std::hash::{Hash, Hasher};

use crate::Mz;

/// The quadrupole settings used for fragmentation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadrupoleSettings {
    pub index: usize,
    pub scan_starts: Vec<usize>,
    pub scan_ends: Vec<usize>,
    pub isolation_windows: Vec<IsolationWindow>,
}

impl Hash for QuadrupoleSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.scan_starts.hash(state);
        self.scan_ends.hash(state);
        for window in &self.isolation_windows {
            window.hash(state);
        }
    }
}

impl QuadrupoleSettings {
    pub fn len(&self) -> usize {
        self.isolation_windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_isolation_window(
        &self,
        scan_index: usize,
    ) -> Option<IsolationWindow> {
        let index = self
            .scan_starts
            .iter()
            .zip(self.scan_ends.iter())
            .position(|(&start, &end)| scan_index >= start && scan_index < end)
            // todo, should this be inclusive?
            .unwrap_or(self.len());
        if index >= self.len() {
            return None;
        }
        let isolation_window = &self.isolation_windows[index];
        Some(isolation_window.clone())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IsolationWindow {
    lower: Mz,
    upper: Mz,
    ce: f64,
}

impl IsolationWindow {
    pub fn new_from_bounds(lower: Mz, upper: Mz, ce: f64) -> Self {
        Self { lower, upper, ce }
    }

    pub fn new_from_center(center: Mz, width: Mz, ce: f64) -> Self {
        let half_width = Mz::from(f64::from(width) / 2.0);
        Self {
            lower: Mz::from(f64::from(center) - f64::from(half_width)),
            upper: Mz::from(f64::from(center) + f64::from(half_width)),
            ce,
        }
    }

    pub fn lower(&self) -> Mz {
        self.lower
    }

    pub fn upper(&self) -> Mz {
        self.upper
    }

    pub fn center(&self) -> Mz {
        Mz::from((f64::from(self.lower) + f64::from(self.upper)) / 2.0)
    }

    pub fn width(&self) -> Mz {
        Mz::from(f64::from(self.upper) - f64::from(self.lower))
    }

    pub fn collision_energy(&self) -> f64 {
        self.ce
    }
}

impl Hash for IsolationWindow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        f64::from(self.lower).to_bits().hash(state);
        f64::from(self.upper).to_bits().hash(state);
        self.ce.to_bits().hash(state);
    }
}

impl Default for IsolationWindow {
    fn default() -> Self {
        Self {
            lower: Mz::from(0.0),
            upper: Mz::from(0.0),
            ce: 0.0,
        }
    }
}
