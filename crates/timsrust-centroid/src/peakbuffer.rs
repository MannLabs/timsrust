use rustc_hash::FxHashMap;

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
        for _ in 0..buffer.len() {
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
                    let peak = Self::pick(ambiguous_peaks);
                    self.peak_queue.push(peak);
                },
            }
        }
        std::mem::swap(&mut self.prev_hits, &mut self.new_hits);
    }

    fn pick(ambiguous_peaks: Vec<Peak>) -> Peak {
        // TODO
        let len = ambiguous_peaks.len() as u32;
        let mut sum = Peak::default();
        for peak in ambiguous_peaks.iter() {
            sum.frame += peak.frame;
            sum.scan += peak.scan;
            sum.tof += peak.tof;
            sum.apex_intensity += peak.apex_intensity;
        }
        Peak {
            frame: sum.frame / len,
            scan: sum.scan / len,
            tof: sum.tof / len,
            apex_intensity: sum.apex_intensity / len as u64,
        }
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
                return None;
            }
            self.rollover();
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

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
