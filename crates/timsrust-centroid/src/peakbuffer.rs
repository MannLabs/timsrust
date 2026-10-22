use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Peak, buffer::RollingSparseBuffer, smoothing::ScanSmoother};

type Options = FxHashMap<u32, (u64, u32)>;

#[derive(Debug)]
pub(crate) struct PeakBuffer {
    buffer: RollingSparseBuffer,
    options: Vec<Options>,
    tofs: Vec<FxHashMap<u32, u64>>,
    prev_hits: FxHashMap<u64, Vec<Peak>>,
    new_hits: FxHashMap<u64, Vec<Peak>>,
    peak_queue: Vec<Peak>,
    scan_smoother: ScanSmoother,
    tof_kernel_len: usize,
    frame_index: u32,
    total_len: usize,
    scan_kernel_apex: u32,
    tof_kernel_apex: u32,
    tof_index: usize,
}

impl PeakBuffer {
    pub(crate) fn new(
        mut tofs: Vec<FxHashMap<u32, u64>>,
        scan_smoother: ScanSmoother,
        tof_kernel_len: usize,
        frame_index: u32,
        scan_kernel_apex: u32,
        tof_kernel_apex: u32,
    ) -> Self {
        assert!(tof_kernel_len >= 3);
        let max_scan = tofs
            .iter()
            .flat_map(|tof_map| tof_map.keys())
            .max()
            .copied()
            .unwrap_or(0)
            + 1
            + scan_kernel_apex
            + scan_smoother.len() as u32;
        let mut buffer = RollingSparseBuffer::new(max_scan);
        // Complete the moving TOF sum's tail, then close terminal plateaus.
        for _ in 0..(2 * tof_kernel_len - 1) {
            tofs.push(FxHashMap::default());
        }
        let total_len = tofs.len();
        let mut options = Vec::with_capacity(tof_kernel_len);
        options.push(
            tofs[0]
                .iter()
                .map(|(&scan, &apex_intensity)| (scan, (apex_intensity, 1)))
                .collect::<Options>(),
        );
        for tof_index in tofs.iter().take(tof_kernel_len).skip(1) {
            let mut new_options =
                options.last().expect("No last option").clone();
            Self::add_to_options(tof_index, &mut new_options);
            options.push(new_options);
        }
        options.iter().take(buffer.len()).for_each(|opt| {
            let opt = scan_smoother.smooth(opt);
            buffer.rollover(opt);
        });
        let prev_hits: FxHashMap<u64, Vec<Peak>> = FxHashMap::default();
        let new_hits: FxHashMap<u64, Vec<Peak>> = FxHashMap::default();
        let peak_queue: Vec<Peak> = Vec::new();
        Self {
            buffer,
            options,
            tofs,
            prev_hits,
            new_hits,
            peak_queue,
            scan_smoother,
            tof_kernel_len,
            frame_index,
            total_len,
            scan_kernel_apex,
            tof_kernel_apex,
            tof_index: 0,
        }
    }

    pub(crate) fn rollover(&mut self) {
        self.collect_peaks();
        let new_options = self.get_new_options();
        self.options.push(new_options);
        self.options.remove(0);
        let opt = self
            .scan_smoother
            .smooth(&self.options[self.buffer.len() - 1]);
        self.buffer.rollover(opt);
        self.tof_index += 1;
    }

    fn get_new_options(&self) -> Options {
        let mut new_options =
            self.options.last().expect("No last option").clone();
        Self::add_to_options(
            &self.tofs[self.tof_index + self.tof_kernel_len],
            &mut new_options,
        );
        Self::remove_from_options(&self.tofs[self.tof_index], &mut new_options);
        new_options
    }

    fn remove_from_options(
        tofs: &FxHashMap<u32, u64>,
        new_options: &mut Options,
    ) {
        for (&scan, &apex_intensity) in tofs.iter() {
            let scan_index = scan;
            let entry = new_options.entry(scan_index).or_insert((0, 0));
            entry.0 -= apex_intensity;
            entry.1 -= 1;
            if entry.1 == 0 {
                new_options.remove(&scan_index);
            }
        }
    }

    fn add_to_options(tofs: &FxHashMap<u32, u64>, new_options: &mut Options) {
        for (&scan, &apex_intensity) in tofs.iter() {
            let scan_index = scan;
            let entry = new_options.entry(scan_index).or_insert((0, 0));
            entry.0 += apex_intensity;
            entry.1 += 1;
        }
    }

    fn collect_peaks(&mut self) {
        for (scan_index, apex_intensity, unique_peak) in
            self.buffer.iter_peaks(self.scan_smoother.len() / 2)
        {
            let peak = Peak {
                frame: self.frame_index,
                scan: scan_index - self.scan_kernel_apex,
                tof: 1 + self.tof_index as u32 - self.tof_kernel_apex,
                apex_intensity,
            };
            if unique_peak {
                self.peak_queue.push(peak);
            } else {
                self.new_hits.entry(apex_intensity).or_default().push(peak);
            }
        }
        for (apex_intensity, ambiguous_peaks) in self.prev_hits.drain() {
            match self.new_hits.get_mut(&apex_intensity) {
                Some(existing_peaks) => {
                    existing_peaks.extend(ambiguous_peaks);
                },
                None => {
                    self.peak_queue
                        .extend(Self::resolve_plateaus(ambiguous_peaks));
                },
            }
        }
        std::mem::swap(&mut self.prev_hits, &mut self.new_hits);
    }

    fn resolve_plateaus(ambiguous_peaks: Vec<Peak>) -> Vec<Peak> {
        let mut remaining: FxHashSet<_> = ambiguous_peaks
            .iter()
            .map(|peak| (peak.scan, peak.tof))
            .collect();
        let mut peaks = Vec::new();
        for seed in ambiguous_peaks {
            if !remaining.remove(&(seed.scan, seed.tof)) {
                continue;
            }
            let mut plateau = vec![seed];
            let mut cursor = 0;
            while cursor < plateau.len() {
                let peak = &plateau[cursor];
                let (scan, tof) = (peak.scan, peak.tof);
                let frame = peak.frame;
                let apex_intensity = peak.apex_intensity;
                for scan in scan.saturating_sub(1)..=scan.saturating_add(1) {
                    for tof in tof.saturating_sub(1)..=tof.saturating_add(1) {
                        if remaining.remove(&(scan, tof)) {
                            plateau.push(Peak {
                                frame,
                                scan,
                                tof,
                                apex_intensity,
                            });
                        }
                    }
                }
                cursor += 1;
            }
            peaks.push(Self::pick(plateau));
        }
        peaks
    }

    fn pick(plateau: Vec<Peak>) -> Peak {
        let len = plateau.len() as u128;
        let scan = (plateau
            .iter()
            .map(|peak| u128::from(peak.scan))
            .sum::<u128>()
            / len) as u32;
        let tof = (plateau
            .iter()
            .map(|peak| u128::from(peak.tof))
            .sum::<u128>()
            / len) as u32;
        // A nonrectangular plateau's mean need not be a peak candidate.
        plateau
            .into_iter()
            .min_by_key(|peak| {
                let ds = u128::from(peak.scan.abs_diff(scan));
                let dt = u128::from(peak.tof.abs_diff(tof));
                (ds * ds + dt * dt, peak.scan, peak.tof)
            })
            .expect("Plateau must contain a peak candidate")
    }
}

impl Iterator for PeakBuffer {
    type Item = Peak;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(peak) = self.peak_queue.pop() {
                return Some(peak);
            }
            if self.tof_index > (self.total_len - self.tof_kernel_len - 1) {
                for (_, ambiguous_peaks) in self.prev_hits.drain() {
                    self.peak_queue
                        .extend(Self::resolve_plateaus(ambiguous_peaks));
                }
                return self.peak_queue.pop();
            }
            self.rollover();
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

    fn centroid(
        points: &[(u32, usize, u64)],
        tof_kernel_len: usize,
        trailing_zeros: usize,
    ) -> Vec<Peak> {
        let len = points.iter().map(|point| point.1).max().unwrap_or(0) + 1;
        let mut tofs = vec![FxHashMap::default(); len + trailing_zeros];
        for &(scan, tof, intensity) in points {
            tofs[tof].insert(scan, intensity);
        }
        let mut peaks: Vec<_> = PeakBuffer::new(
            tofs,
            ScanSmoother::new(vec![1, 2, 1], 1),
            tof_kernel_len,
            42,
            1,
            (tof_kernel_len / 2) as u32,
        )
        .collect();
        peaks.sort_unstable_by_key(|peak| (peak.scan, peak.tof));
        peaks
    }

    #[test]
    fn disconnected_equal_response_peaks_remain_separate() {
        let peaks = centroid(&[(20, 30, 100), (40, 30, 100)], 3, 20);
        assert_eq!(
            peaks,
            [20, 40].map(|scan| Peak {
                frame: 42,
                scan,
                tof: 30,
                apex_intensity: 200,
            })
        );
    }

    #[test]
    fn terminal_peak_is_independent_of_trailing_zero_bins() {
        for kernel_len in [3, 5, 7] {
            let points = [(20, 30, 100)];
            let expected = vec![Peak {
                frame: 42,
                scan: 20,
                tof: 30,
                apex_intensity: 200,
            }];
            assert_eq!(centroid(&points, kernel_len, 20), expected);
            assert_eq!(centroid(&points, kernel_len, 0), expected);
        }
    }

    #[test]
    fn terminal_peak_is_independent_of_distant_raw_background() {
        let original = centroid(&[(20, 30, 100)], 3, 0);
        let mut with_background = centroid(&[(20, 30, 100), (80, 60, 1)], 3, 0);
        assert_eq!(with_background.len(), 2);
        with_background.retain(|peak| peak.scan == 20);
        assert_eq!(original, with_background);
        assert_eq!(original.len(), 1);
    }

    #[test]
    fn connected_plateau_retains_its_centroid_and_response() {
        assert_eq!(
            centroid(&[(20, 30, 100), (21, 30, 100)], 3, 20),
            vec![Peak {
                frame: 42,
                scan: 20,
                tof: 30,
                apex_intensity: 300,
            }]
        );
    }

    #[test]
    fn empty_input_is_exhausted_repeatedly() {
        let mut buffer = PeakBuffer::new(
            Vec::new(),
            ScanSmoother::new(vec![1, 2, 1], 1),
            7,
            42,
            1,
            3,
        );
        assert_eq!(buffer.next(), None);
        assert_eq!(buffer.next(), None);
    }

    fn candidates(coordinates: &[(u32, u32)]) -> Vec<Peak> {
        coordinates
            .iter()
            .map(|&(scan, tof)| Peak {
                frame: u32::MAX,
                scan,
                tof,
                apex_intensity: u64::MAX,
            })
            .collect()
    }

    #[test]
    fn plateau_connectivity_includes_diagonals_but_not_gaps() {
        let mut peaks = PeakBuffer::resolve_plateaus(candidates(&[
            (10, 20),
            (11, 21),
            (13, 21),
            (14, 22),
        ]));
        peaks.sort_unstable_by_key(|peak| (peak.scan, peak.tof));
        assert_eq!(peaks, candidates(&[(10, 20), (13, 21)]));
    }

    #[test]
    fn plateau_centroid_is_an_actual_candidate_with_stable_tie_breaking() {
        let points = [(1, 1), (1, 2), (1, 3), (2, 3), (3, 3), (3, 2), (3, 1)];
        let mut input = candidates(&points);
        let expected = candidates(&[(1, 2)]);
        assert_eq!(PeakBuffer::resolve_plateaus(input.clone()), expected);
        input.reverse();
        assert_eq!(PeakBuffer::resolve_plateaus(input), expected);
    }

    #[test]
    fn short_tof_input_is_supported() {
        assert_eq!(centroid(&[], 7, 0), Vec::<Peak>::new());
        assert_eq!(
            centroid(&[(20, 1, 100)], 3, 0),
            vec![Peak {
                frame: 42,
                scan: 20,
                tof: 1,
                apex_intensity: 200,
            }]
        );
    }

    #[test]
    fn test_peakbuffer_new_and_iter() {
        let tofs = vec![
            FxHashMap::from_iter([(1u32, 100u64), (2u32, 200u64)]),
            FxHashMap::from_iter([(1u32, 150u64), (3u32, 250u64)]),
            FxHashMap::from_iter([(2u32, 120u64)]),
            FxHashMap::from_iter([(1u32, 180u64)]),
        ];
        let scan_kernel = vec![1, 2, 1];
        let scan_smoother = ScanSmoother::new(scan_kernel, 1);
        let tof_len = 3;
        let frame_index = 0;
        let scan_kernel_apex = 1;
        let tof_kernel_apex = 1;
        let mut pb = PeakBuffer::new(
            tofs,
            scan_smoother,
            tof_len,
            frame_index,
            scan_kernel_apex,
            tof_kernel_apex,
        );
        let _ = pb.next();
    }
}
