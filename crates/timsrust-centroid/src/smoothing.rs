use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub(crate) struct ScanSmoother {
    scan_kernel: Vec<u64>,
    min_count: usize,
}

impl ScanSmoother {
    pub(crate) fn new(scan_kernel: Vec<u64>, min_count: usize) -> Self {
        Self {
            scan_kernel,
            min_count,
        }
    }

    pub(crate) fn kernel(&self) -> &[u64] {
        &self.scan_kernel
    }

    pub(crate) fn len(&self) -> usize {
        self.scan_kernel.len()
    }

    pub(crate) fn min_count(&self) -> usize {
        self.min_count
    }

    pub(crate) fn smooth(
        &self,
        options: &FxHashMap<u32, (u64, u32)>,
    ) -> FxHashMap<u32, u64> {
        let mut opt = FxHashMap::default();
        let mut scans = options.keys().cloned().collect::<Vec<_>>();
        scans.sort_unstable();
        let usable = Self::set_usable_candidates(
            self.scan_kernel.len() as u32,
            options,
            self.min_count,
            &scans,
        );
        for (i, &scan) in scans.iter().enumerate() {
            if !usable[i] {
                continue;
            }
            let &(apex_intensity, count) = options.get(&scan).unwrap();
            for (scan_offset, &value) in self.scan_kernel.iter().enumerate() {
                let key = scan + scan_offset as u32;
                let entry = opt.entry(key).or_insert((0, 0));
                entry.0 += apex_intensity * value;
                entry.1 += count;
            }
        }
        opt.into_iter()
            .filter_map(|(scan, (apex_intensity, count))| {
                if count >= self.min_count as u32 {
                    Some((scan, apex_intensity))
                } else {
                    None
                }
            })
            .collect()
    }

    fn set_usable_candidates(
        kernel_len: u32,
        options: &FxHashMap<u32, (u64, u32)>,
        min_count: usize,
        scans: &[u32],
    ) -> Vec<bool> {
        let mut result = vec![false; scans.len()];
        let mut left = 0;
        let mut right = 0;
        while left < scans.len() {
            while right < scans.len() && scans[right] < scans[left] + kernel_len
            {
                right += 1;
            }
            let window_count: u32 = scans[left..right]
                .iter()
                .map(|scan| options.get(scan).unwrap().1)
                .sum();
            if window_count >= min_count as u32 {
                for i in result.iter_mut().take(right).skip(left) {
                    *i = true;
                }
            }
            left += 1;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_basic() {
        let smoother = ScanSmoother::new(vec![1, 2, 1], 1);
        let mut options = FxHashMap::default();
        options.insert(1, (10, 1));
        options.insert(2, (20, 1));
        options.insert(4, (40, 1));
        let result = smoother.smooth(&options);
        let mut expected = FxHashMap::default();
        expected.insert(1, 10);
        expected.insert(2, 40);
        expected.insert(3, 50);
        expected.insert(4, 60);
        expected.insert(5, 80);
        expected.insert(6, 40);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_smooth_min_count() {
        let smoother = ScanSmoother::new(vec![1, 1], 3);
        let mut options = FxHashMap::default();
        options.insert(1, (5, 1));
        options.insert(2, (10, 1));
        options.insert(3, (15, 1));
        // Each scan has count 1, so any window of size 2 has at most 2 counts
        // min_count is 3, so result should be empty
        let result = smoother.smooth(&options);
        assert!(result.is_empty());
    }

    #[test]
    fn test_set_usable_candidates() {
        let mut options = FxHashMap::default();
        options.insert(1, (10, 1));
        options.insert(2, (20, 2));
        options.insert(4, (40, 1));
        let scans = vec![1, 2, 4];
        let usable =
            ScanSmoother::set_usable_candidates(2, &options, 3, &scans);
        assert_eq!(usable, vec![true, true, false]);
    }
}
