// type HashMap = rustc_hash::FxHashMap<u32, u64>;
type HashMap = timsrust_core::utils::hash_sets::FiniteHashMap<u64>;

const BUFFER_WIDTH: usize = 3;

#[derive(Debug)]
pub(crate) struct RollingSparseBuffer {
    candidates: [HashMap; BUFFER_WIDTH],
}

impl RollingSparseBuffer {
    pub(crate) fn new(capacity: u32) -> Self {
        Self {
            candidates: std::array::from_fn(|_| {
                HashMap::with_capacity(capacity)
            }),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn rollover(
        &mut self,
        candidates: rustc_hash::FxHashMap<u32, u64>,
    ) {
        self.candidates[0].clear();
        candidates.iter().for_each(|(&k, &v)| {
            self.candidates[0].insert(k, v);
        });
        // self.candidates[0] = candidates;
        self.candidates.rotate_left(1);
    }

    pub(crate) fn iter_peaks(
        &self,
        window: usize,
    ) -> impl Iterator<Item = (u32, u64, bool)> + '_ {
        let mut iter = self.candidates[1].iter_sorted();
        std::iter::from_fn(move || {
            for (scan_index, &value) in iter.by_ref() {
                let i = scan_index as usize;
                let center = value;
                let mut is_peak = true;
                let mut is_semi_peak = true;
                for dj in 0..=window as isize {
                    for di in 0..BUFFER_WIDTH {
                        if di == 1 && dj == 0 {
                            continue;
                        }
                        let ni = di;
                        for nj in [i + dj as usize, i - dj as usize] {
                            if let Some(&x) = self.candidates[ni].get(nj as u32)
                            {
                                is_peak &= x < center;
                                is_semi_peak &= x <= center;
                            }
                        }
                    }
                    if !is_semi_peak {
                        break;
                    }
                }
                if is_semi_peak {
                    return Some((scan_index, center, is_peak));
                }
            }
            None
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_new_and_len() {
//         let buf = RollingSparseBuffer::new();
//         assert_eq!(buf.len(), BUFFER_WIDTH);
//     }

//     #[test]
//     fn test_rollover_and_candidates() {
//         let mut buf = RollingSparseBuffer::new();
//         let mut map1 = HashMap::default();
//         map1.insert(1, 10);
//         buf.rollover(map1.clone());
//         assert_eq!(buf.candidates[buf.len() - 1], map1);
//     }

//     #[test]
//     fn test_iter_peaks_basic() {
//         let mut buf = RollingSparseBuffer::new();
//         let mut map0 = HashMap::default();
//         let mut map1 = HashMap::default();
//         let mut map2 = HashMap::default();
//         // Simulate a peak at scan 2 in the center buffer
//         map0.insert(1, 5);
//         map1.insert(2, 10);
//         map2.insert(3, 5);
//         buf.candidates[0] = map0;
//         buf.candidates[1] = map1;
//         buf.candidates[2] = map2;
//         let peaks: Vec<_> = buf.iter_peaks(1).collect();
//         // Only one entry in center buffer, should be returned as a peak
//         assert_eq!(peaks.len(), 1);
//         assert_eq!(peaks[0].0, 2);
//         assert_eq!(peaks[0].1, 10);
//         // is_peak should be true if all neighbors are less
//         assert!(peaks[0].2);
//     }

//     #[test]
//     fn test_iter_peaks_no_peaks() {
//         let mut buf = RollingSparseBuffer::new();
//         let mut map1 = HashMap::default();
//         map1.insert(1, 1);
//         buf.candidates[1] = map1;
//         let peaks: Vec<_> = buf.iter_peaks(1).collect();
//         // Only one entry, but no neighbors, so should still be a peak
//         assert_eq!(peaks.len(), 1);
//         assert_eq!(peaks[0].0, 1);
//         assert_eq!(peaks[0].1, 1);
//     }

//     #[test]
//     fn test_iter_peaks_multiple() {
//         let mut buf = RollingSparseBuffer::new();
//         let mut map0 = HashMap::default();
//         let mut map1 = HashMap::default();
//         let mut map2 = HashMap::default();
//         map0.insert(1, 2);
//         map0.insert(2, 2);
//         map1.insert(1, 5);
//         map1.insert(2, 1);
//         map2.insert(1, 2);
//         map2.insert(2, 2);
//         buf.candidates[0] = map0;
//         buf.candidates[1] = map1;
//         buf.candidates[2] = map2;
//         let peaks: Vec<_> = buf.iter_peaks(1).collect();
//         // Only (1,5) is a peak
//         assert_eq!(peaks.len(), 1);
//         assert_eq!(peaks[0].0, 1);
//         assert_eq!(peaks[0].1, 5);
//         assert!(peaks[0].2);
//     }
// }
