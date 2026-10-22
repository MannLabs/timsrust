use std::hash::{Hash, Hasher};

use crate::Mz;

/// The quadrupole settings used for fragmentation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadrupoleSettings {
    pub index: usize,
    /// Inclusive, zero-based scan starts.
    pub scan_starts: Vec<usize>,
    /// Exclusive, zero-based scan ends.
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
        self.get_isolation_window_index(scan_index)
            .map(|index| self.isolation_windows[index].clone())
    }

    /// Returns the interval containing a scan, using half-open bounds.
    ///
    /// Scans before, between, or after the declared intervals return `None`.
    pub fn get_isolation_window_index(
        &self,
        scan_index: usize,
    ) -> Option<usize> {
        self.scan_starts
            .iter()
            .zip(self.scan_ends.iter())
            .take(self.len())
            .position(|(&start, &end)| scan_index >= start && scan_index < end)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_lookup_is_half_open_and_does_not_fill_gaps() {
        let windows = vec![
            IsolationWindow::new_from_center(
                Mz::from(500.0),
                Mz::from(20.0),
                30.0,
            ),
            IsolationWindow::new_from_center(
                Mz::from(600.0),
                Mz::from(30.0),
                40.0,
            ),
        ];
        let mut settings = QuadrupoleSettings {
            index: 1,
            scan_starts: vec![10, 20],
            scan_ends: vec![20, 30],
            isolation_windows: windows.clone(),
        };
        for (scan, expected) in [
            (9, None),
            (10, Some(0)),
            (19, Some(0)),
            (20, Some(1)),
            (29, Some(1)),
            (30, None),
        ] {
            assert_eq!(settings.get_isolation_window_index(scan), expected);
            assert_eq!(
                settings.get_isolation_window(scan),
                expected.map(|index| windows[index].clone())
            );
        }
        settings.scan_starts[1] = 25;
        assert_eq!(settings.get_isolation_window_index(20), None);
        assert_eq!(settings.get_isolation_window(24), None);
        assert_eq!(settings.get_isolation_window_index(25), Some(1));
        assert_eq!(QuadrupoleSettings::default().get_isolation_window(0), None);
    }
}
