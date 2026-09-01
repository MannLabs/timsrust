use core::slice;
use std::{
    alloc::{Layout, alloc, dealloc},
    collections::HashMap,
    ffi::CString,
    os::raw::c_void,
    path::PathBuf,
};

use libc::c_char;
use timsrust_core::io::Uri;

pub struct Scan {
    pub num_peaks: u32,
    pub indices: Vec<u32>,
    pub intensities: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Copy, Default)]
#[repr(C)]
pub enum PressureCompensationStrategy {
    #[default]
    NoPressureCompensation = 0,
    AnalyisGlobalPressureCompensation = 1,
    PerFramePressureCompensation = 2,
    PerFramePressureCompensationWithMissingReference = 3,
}

#[derive(Debug, Default, PartialEq)]
pub struct TimsData {
    pub analysis_directory_name: PathBuf,
    pub use_recalibrated_state: bool,
    pub pressure_compensation_strategy: PressureCompensationStrategy,
    pub handle: u64,
}

impl Clone for TimsData {
    fn clone(&self) -> Self {
        TimsData::new(
            self.analysis_directory_name.clone(),
            self.use_recalibrated_state,
            self.pressure_compensation_strategy,
        )
    }
}

impl TimsData {
    pub fn new(
        analysis_directory_name: PathBuf,
        use_recalibrated_state: bool,
        pressure_compensation_strategy: PressureCompensationStrategy,
    ) -> Self {
        let analysis_directory_name =
            Uri::from(analysis_directory_name).soft_cache();
        let analysis_directory_name = analysis_directory_name
            .as_path()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let input_analysis = CString::new(
            analysis_directory_name
                .clone()
                .into_os_string()
                .into_string()
                .unwrap(),
        )
        .unwrap();
        let recalibrated_state = if use_recalibrated_state { 1 } else { 0 };
        let handle = unsafe {
            tims_open_v2(
                input_analysis.as_ptr(),
                recalibrated_state,
                PressureCompensationStrategy::NoPressureCompensation,
            )
        };

        if handle == 0 {
            panic!("{}", get_last_error());
        }

        TimsData {
            analysis_directory_name,
            use_recalibrated_state,
            pressure_compensation_strategy,
            handle,
        }
    }

    pub fn with_num_threads(
        analysis_directory_name: PathBuf,
        use_recalibrated_state: bool,
        pressure_compensation_strategy: PressureCompensationStrategy,
        num_threads: u32,
    ) -> Self {
        let input_analysis = CString::new(
            analysis_directory_name
                .clone()
                .into_os_string()
                .into_string()
                .unwrap(),
        )
        .unwrap();
        let recalibrated_state = if use_recalibrated_state { 1 } else { 0 };

        let handle = unsafe {
            tims_open_v2(
                input_analysis.as_ptr(),
                recalibrated_state,
                PressureCompensationStrategy::NoPressureCompensation,
            )
        };

        unsafe {
            tims_set_num_threads(num_threads);
        }
        if handle == 0 {
            panic!("{}", get_last_error());
        }

        TimsData {
            analysis_directory_name,
            use_recalibrated_state,
            pressure_compensation_strategy,
            handle,
        }
    }

    pub fn close(self) {
        unsafe { tims_close(self.handle) };
    }

    pub fn read_scans(
        &mut self,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
    ) -> Vec<Scan> {
        let mut initial_frame_buffer_size: u32 = 128;
        let buf: *mut c_void;

        let mut ptr: *mut u8;
        let mut layout: Layout;

        loop {
            let tmp_buf: *mut c_void;
            let current_len = 4 * initial_frame_buffer_size;

            layout = Layout::from_size_align(
                current_len as usize,
                std::mem::align_of::<u32>(),
            )
            .unwrap();
            ptr = unsafe { alloc(layout) };
            let required_len = unsafe {
                tmp_buf = ptr as *mut c_void;
                tims_read_scans_v2(
                    self.handle,
                    frame_id,
                    scan_begin,
                    scan_end,
                    tmp_buf,
                    current_len,
                )
            };

            if required_len == 0 {
                unsafe {
                    dealloc(ptr, layout);
                }
                panic!("{}", get_last_error());
            }

            if required_len > current_len {
                if required_len > 16777216 {
                    unsafe {
                        dealloc(ptr, layout);
                    }
                    panic!("Maximum expected frame size exceeded.");
                }
                //initial_frame_buffer_size = required_len / 4 + 1;
                unsafe {
                    dealloc(ptr, layout);
                }
                initial_frame_buffer_size = required_len / 4;
            } else {
                buf = tmp_buf;
                break;
            }
        }

        let mut d: usize = (scan_end - scan_begin) as usize;
        let mut npeaks: u32;

        let casted_data = unsafe {
            std::slice::from_raw_parts_mut(
                buf as *mut u32,
                initial_frame_buffer_size as usize,
            )
        };
        //let t = casted_data.to_vec();
        let mut scans: Vec<Scan> = Vec::with_capacity(d);

        for i in scan_begin..scan_end {
            let npeaks_index = (i - scan_begin) as usize;
            npeaks = casted_data[npeaks_index];

            let npeaks_usize = usize::try_from(npeaks).unwrap();

            let current_indices = casted_data[d..d + npeaks_usize].to_vec();
            d += npeaks_usize;

            let current_intensities = casted_data[d..d + npeaks_usize].to_vec();
            //let current_intensities = &casted_data[d..d+npeaks_usize];
            d += npeaks_usize;

            let new_scan = Scan {
                num_peaks: npeaks,
                indices: current_indices,
                intensities: current_intensities,
            };

            scans.push(new_scan);
        }
        unsafe {
            dealloc(ptr, layout);
        }
        scans
    }

    /// Gets MSMS Spectra for the given Frame ID. Returns a HashMap where the key is the precursor ID and the value is a tuple of two vectors. The first one contains mz_values and the second one area_values.
    pub fn read_pasef_msms_for_frame(
        &mut self,
        frame_id: i64,
    ) -> HashMap<i64, (Vec<f64>, Vec<f32>)> {
        let results: HashMap<i64, (Vec<f64>, Vec<f32>)> = HashMap::new();

        /// Callback function to store provided data from native code.
        unsafe extern "C" fn store_data(
            id: i64,
            num_peaks: u32,
            mz_values: *const f64,
            area_values: *const f32,
            user_data: *mut c_void,
        ) {
            let mut new_mz_values: Vec<f64> = Vec::new();
            let mut new_area_values: Vec<f32> = Vec::new();

            if num_peaks != 0 && !mz_values.is_null() && !area_values.is_null()
            {
                new_mz_values = unsafe {
                    slice::from_raw_parts(mz_values, num_peaks as usize)
                }
                .to_vec();
                new_area_values = unsafe {
                    slice::from_raw_parts(area_values, num_peaks as usize)
                }
                .to_vec();
            }

            let recovered_ptr: *mut HashMap<i64, (Vec<f64>, Vec<f32>)> =
                user_data as *mut HashMap<i64, (Vec<f64>, Vec<f32>)>;

            unsafe {
                (*recovered_ptr).insert(id, (new_mz_values, new_area_values))
            };
        }

        let callback_function: MsmsSpectrumFunction =
            MsmsSpectrumFunction::Some(store_data);

        let pointer_to_results = Box::into_raw(Box::new(results));

        let r = unsafe {
            let user_data = pointer_to_results as *mut c_void;
            tims_read_pasef_msms_for_frame_v2(
                self.handle,
                frame_id,
                callback_function,
                user_data,
            )
        };

        if r == 0 {
            // Error
            panic!(
                "Could not get spectra from frame {}. Error: {}",
                frame_id,
                get_last_error()
            )
        }

        let recovered_map = unsafe { Box::from_raw(pointer_to_results) };

        *recovered_map
    }

    pub fn index_to_mz(&mut self, frame_id: i64, in_: Vec<f64>) -> Vec<f64> {
        let input_count = in_.len() as u32;

        // Output buffer, same length as input
        let mut output_values: Vec<f64> = vec![0.0; input_count as usize]; // allocate space for the output

        let result = unsafe {
            tims_index_to_mz(
                self.handle,
                frame_id,
                in_.as_ptr(),
                output_values.as_mut_ptr(),
                input_count,
            )
        };

        if result == 0 {
            // Error
            panic!(
                "Could not get mz index from frame {}. Error: {}",
                frame_id,
                get_last_error()
            )
        }

        output_values
    }

    pub fn mz_to_index(&mut self, frame_id: i64, in_: Vec<f64>) -> Vec<f64> {
        let input_count = in_.len() as u32;

        // Output buffer, same length as input
        let mut output_values: Vec<f64> = vec![0.0; input_count as usize]; // allocate space for the output

        let result = unsafe {
            tims_mz_to_index(
                self.handle,
                frame_id,
                in_.as_ptr(),
                output_values.as_mut_ptr(),
                input_count,
            )
        };

        if result == 0 {
            // Error
            panic!(
                "Could not get mz index from frame {}. Error: {}",
                frame_id,
                get_last_error()
            )
        }

        output_values
    }

    pub fn scan_num_to_one_over_k0(
        &self,
        frame_id: i64,
        in_: Vec<f64>,
    ) -> Vec<f64> {
        let input_count = in_.len() as u32;

        // Output buffer, same length as input
        let mut output_values: Vec<f64> = vec![0.0; input_count as usize]; // allocate space for the output

        let result = unsafe {
            tims_scannum_to_oneoverk0(
                self.handle,
                frame_id,
                in_.as_ptr(),
                output_values.as_mut_ptr(),
                input_count,
            )
        };

        if result == 0 {
            // Error
            panic!(
                "Could not get 1/K0 from frame {}. Error: {}",
                frame_id,
                get_last_error()
            )
        }

        output_values
    }

    pub fn one_over_k0_to_scan_number(
        &self,
        frame_id: i64,
        in_: Vec<f64>,
    ) -> Vec<f64> {
        let input_count = in_.len() as u32;

        // Output buffer, same length as input
        let mut output_values: Vec<f64> = vec![0.0; input_count as usize]; // allocate space for the output

        let result = unsafe {
            tims_oneoverk0_to_scannum(
                self.handle,
                frame_id,
                in_.as_ptr(),
                output_values.as_mut_ptr(),
                input_count,
            )
        };

        if result == 0 {
            // Error
            panic!(
                "Could not get 1/K0 from frame {}. Error: {}",
                frame_id,
                get_last_error()
            )
        }

        output_values
    }

    pub fn set_num_threads(self, num_threads: u32) {
        unsafe { tims_set_num_threads(num_threads) }
    }

    /// Opens a data set using a specific re-calibration identified by its UUID.
    pub fn open_recalibration_id(
        analysis_directory_name: PathBuf,
        use_calibration_id: &str,
    ) -> Self {
        let input_analysis = CString::new(
            analysis_directory_name
                .clone()
                .into_os_string()
                .into_string()
                .unwrap(),
        )
        .unwrap();
        let calibration_id = CString::new(use_calibration_id).unwrap();
        let handle = unsafe {
            tims_open_recalibration_id(
                input_analysis.as_ptr(),
                calibration_id.as_ptr(),
            )
        };
        if handle == 0 {
            panic!("{}", get_last_error());
        }
        TimsData {
            analysis_directory_name,
            use_recalibrated_state: true,
            pressure_compensation_strategy:
                PressureCompensationStrategy::default(),
            handle,
        }
    }

    /// Returns `true` if a re-calibrated state is present and currently in use.
    pub fn has_recalibrated_state(&self) -> bool {
        unsafe { tims_has_recalibrated_state(self.handle) != 0 }
    }

    /// Returns the UUID of the active re-calibration, or `None` when the raw
    /// instrument calibration is in use.
    pub fn get_calibration_id(&self) -> Option<String> {
        let mut buffer = vec![0u8; 256];
        let len = unsafe {
            tims_get_calibration_id(
                self.handle,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as u32,
            )
        };
        if len == 0 {
            return None;
        }
        let end = (len as usize).min(buffer.len()).saturating_sub(1);
        Some(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }

    /// Reads centroided MS/MS spectra for a list of PASEF precursor IDs.
    pub fn read_pasef_msms(
        &mut self,
        precursors: &[i64],
    ) -> HashMap<i64, (Vec<f64>, Vec<f32>)> {
        let results: HashMap<i64, (Vec<f64>, Vec<f32>)> = HashMap::new();
        let pointer_to_results = Box::into_raw(Box::new(results));
        let r = unsafe {
            tims_read_pasef_msms_v2(
                self.handle,
                precursors.as_ptr(),
                precursors.len() as u32,
                Some(collect_centroided_into_map),
                pointer_to_results as *mut c_void,
            )
        };
        if r == 0 {
            drop(unsafe { Box::from_raw(pointer_to_results) });
            panic!(
                "Could not read PASEF MS/MS spectra. Error: {}",
                get_last_error()
            );
        }
        *unsafe { Box::from_raw(pointer_to_results) }
    }

    /// Reads "quasi profile" MS/MS spectra for a list of PASEF precursor IDs.
    pub fn read_pasef_profile_msms(
        &mut self,
        precursors: &[i64],
    ) -> HashMap<i64, Vec<i32>> {
        let results: HashMap<i64, Vec<i32>> = HashMap::new();
        let pointer_to_results = Box::into_raw(Box::new(results));
        let r = unsafe {
            tims_read_pasef_profile_msms_v2(
                self.handle,
                precursors.as_ptr(),
                precursors.len() as u32,
                Some(collect_profile_into_map),
                pointer_to_results as *mut c_void,
            )
        };
        if r == 0 {
            drop(unsafe { Box::from_raw(pointer_to_results) });
            panic!(
                "Could not read PASEF profile MS/MS spectra. Error: {}",
                get_last_error()
            );
        }
        *unsafe { Box::from_raw(pointer_to_results) }
    }

    /// Reads "quasi profile" MS/MS spectra for all PASEF precursors in a frame.
    pub fn read_pasef_profile_msms_for_frame(
        &mut self,
        frame_id: i64,
    ) -> HashMap<i64, Vec<i32>> {
        let results: HashMap<i64, Vec<i32>> = HashMap::new();
        let pointer_to_results = Box::into_raw(Box::new(results));
        let r = unsafe {
            tims_read_pasef_profile_msms_for_frame_v2(
                self.handle,
                frame_id,
                Some(collect_profile_into_map),
                pointer_to_results as *mut c_void,
            )
        };
        if r == 0 {
            drop(unsafe { Box::from_raw(pointer_to_results) });
            panic!(
                "Could not read PASEF profile MS/MS spectra for frame {}. Error: {}",
                frame_id,
                get_last_error()
            );
        }
        *unsafe { Box::from_raw(pointer_to_results) }
    }

    /// Extracts a single centroided spectrum for a frame (Bruker default resolution).
    pub fn extract_centroided_spectrum_for_frame(
        &mut self,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
    ) -> CentroidedSpectrum {
        let handle = self.handle;
        collect_single_centroided(frame_id, |user_data| unsafe {
            tims_extract_centroided_spectrum_for_frame_v2(
                handle,
                frame_id,
                scan_begin,
                scan_end,
                Some(collect_centroided_into_map),
                user_data,
            )
        })
    }

    /// Same as [`Self::extract_centroided_spectrum_for_frame`], using the v3
    /// implementation (optimized for sparse data).
    pub fn extract_centroided_spectrum_for_frame_v3(
        &mut self,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
    ) -> CentroidedSpectrum {
        let handle = self.handle;
        collect_single_centroided(frame_id, |user_data| unsafe {
            tims_extract_centroided_spectrum_for_frame_v3(
                handle,
                frame_id,
                scan_begin,
                scan_end,
                Some(collect_centroided_into_map),
                user_data,
            )
        })
    }

    /// Extracts a single centroided spectrum with a custom peak-picker resolution.
    pub fn extract_centroided_spectrum_for_frame_ext(
        &mut self,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        peak_finder_resolution: f64,
    ) -> CentroidedSpectrum {
        let handle = self.handle;
        collect_single_centroided(frame_id, |user_data| unsafe {
            tims_extract_centroided_spectrum_for_frame_ext(
                handle,
                frame_id,
                scan_begin,
                scan_end,
                peak_finder_resolution,
                Some(collect_centroided_into_map),
                user_data,
            )
        })
    }

    /// Same as [`Self::extract_centroided_spectrum_for_frame_ext`], v3 (sparse-optimized).
    pub fn extract_centroided_spectrum_for_frame_ext_v3(
        &mut self,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        peak_finder_resolution: f64,
    ) -> CentroidedSpectrum {
        let handle = self.handle;
        collect_single_centroided(frame_id, |user_data| unsafe {
            tims_extract_centroided_spectrum_for_frame_ext_v3(
                handle,
                frame_id,
                scan_begin,
                scan_end,
                peak_finder_resolution,
                Some(collect_centroided_into_map),
                user_data,
            )
        })
    }

    /// Extracts a single "quasi profile" spectrum for a frame.
    pub fn extract_profile_for_frame(
        &mut self,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
    ) -> Vec<i32> {
        let handle = self.handle;
        collect_single_profile(frame_id, |user_data| unsafe {
            tims_extract_profile_for_frame(
                handle,
                frame_id,
                scan_begin,
                scan_end,
                Some(collect_profile_into_map),
                user_data,
            )
        })
    }

    /// Converts scan numbers to TIMS voltages for the given frame.
    pub fn scan_num_to_voltage(
        &self,
        frame_id: i64,
        in_: Vec<f64>,
    ) -> Vec<f64> {
        convert_values(
            self.handle,
            frame_id,
            in_,
            tims_scannum_to_voltage,
            "voltage",
        )
    }

    /// Converts TIMS voltages to scan numbers for the given frame.
    pub fn voltage_to_scan_num(
        &self,
        frame_id: i64,
        in_: Vec<f64>,
    ) -> Vec<f64> {
        convert_values(
            self.handle,
            frame_id,
            in_,
            tims_voltage_to_scannum,
            "scan number",
        )
    }

    /// Number of fragmentation experiments defined for a frame.
    pub fn number_of_fragmentation_experiments(&self, frame_id: i64) -> i64 {
        let n = unsafe {
            tims_get_number_of_fragmentation_experiments(self.handle, frame_id)
        };
        if n < 0 {
            panic!(
                "Could not get number of fragmentation experiments for frame {}. Error: {}",
                frame_id,
                get_last_error()
            );
        }
        n
    }

    /// Number of steps in a given fragmentation experiment of a frame.
    pub fn number_of_fragmentation_experiment_steps(
        &self,
        frame_id: i64,
        experiment_index: i32,
    ) -> i64 {
        let n = unsafe {
            tims_get_number_of_fragmentation_experiment_steps(
                self.handle,
                frame_id,
                experiment_index,
            )
        };
        if n < 0 {
            panic!(
                "Could not get number of fragmentation steps for frame {} experiment {}. Error: {}",
                frame_id,
                experiment_index,
                get_last_error()
            );
        }
        n
    }

    /// Details of a single fragmentation step.
    pub fn fragmentation_experiment_step(
        &self,
        frame_id: i64,
        experiment_index: i32,
        step_index: i32,
    ) -> FragmentationStep {
        let mut raw: IsolationFragmentationStep = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            tims_get_fragmentation_experiment_step(
                self.handle,
                frame_id,
                experiment_index,
                step_index,
                &mut raw,
            )
        };
        if ok == 0 {
            panic!(
                "Could not get fragmentation step ({}, {}) for frame {}. Error: {}",
                experiment_index,
                step_index,
                frame_id,
                get_last_error()
            );
        }
        FragmentationStep::from(raw)
    }

    /// Reads all fragmentation experiments of a frame, each as its list of steps.
    pub fn read_fragmentation_experiments(
        &self,
        frame_id: i64,
    ) -> Vec<Vec<FragmentationStep>> {
        let num_experiments =
            self.number_of_fragmentation_experiments(frame_id);
        let mut experiments = Vec::with_capacity(num_experiments as usize);
        for experiment_index in 0..num_experiments as i32 {
            let num_steps = self.number_of_fragmentation_experiment_steps(
                frame_id,
                experiment_index,
            );
            let mut steps = Vec::with_capacity(num_steps as usize);
            for step_index in 0..num_steps as i32 {
                steps.push(self.fragmentation_experiment_step(
                    frame_id,
                    experiment_index,
                    step_index,
                ));
            }
            experiments.push(steps);
        }
        experiments
    }

    /// Extracts MS1-only chromatograms for the given jobs. Jobs must be ordered
    /// by ascending `time_begin`.
    pub fn extract_chromatograms(
        &mut self,
        jobs: Vec<TimsChromatogramJob>,
    ) -> Vec<ChromatogramTrace> {
        struct ChromCtx {
            jobs: std::vec::IntoIter<TimsChromatogramJob>,
            traces: Vec<ChromatogramTrace>,
        }

        unsafe extern "C" fn generate_job(
            job: *mut TimsChromatogramJob,
            user_data: *mut c_void,
        ) -> u32 {
            let ctx = unsafe { &mut *(user_data as *mut ChromCtx) };
            match ctx.jobs.next() {
                Some(next_job) => {
                    unsafe { *job = next_job };
                    1
                },
                None => 2,
            }
        }

        unsafe extern "C" fn deliver_trace(
            id: i64,
            num_points: u32,
            frame_ids: *const i64,
            values: *const u64,
            user_data: *mut c_void,
        ) -> u32 {
            let ctx = unsafe { &mut *(user_data as *mut ChromCtx) };
            let frame_ids = if num_points != 0 && !frame_ids.is_null() {
                unsafe { slice::from_raw_parts(frame_ids, num_points as usize) }
                    .to_vec()
            } else {
                Vec::new()
            };
            let values = if num_points != 0 && !values.is_null() {
                unsafe { slice::from_raw_parts(values, num_points as usize) }
                    .to_vec()
            } else {
                Vec::new()
            };
            ctx.traces.push(ChromatogramTrace {
                id,
                frame_ids,
                values,
            });
            1
        }

        let ctx = Box::into_raw(Box::new(ChromCtx {
            jobs: jobs.into_iter(),
            traces: Vec::new(),
        }));
        let r = unsafe {
            tims_extract_chromatograms(
                self.handle,
                Some(generate_job),
                Some(deliver_trace),
                ctx as *mut c_void,
            )
        };
        let ctx = *unsafe { Box::from_raw(ctx) };
        if r == 0 {
            panic!(
                "Could not extract chromatograms. Error: {}",
                get_last_error()
            );
        }
        ctx.traces
    }
}

/// Gets the last error from the native library as a String.
fn get_last_error() -> String {
    unsafe {
        let new_string = std::iter::repeat_n(" ", 256).collect::<String>();

        let msg = CString::new(new_string).unwrap();
        //let msg = CString::new(BULLSHIT).unwrap();
        let raw = msg.into_raw();
        tims_get_last_error_string(raw, 256);
        let r = CString::from_raw(raw).into_string(); //.unwrap();
        match r {
            Ok(s) => s,
            Err(e) => panic!("{}", e.to_string()),
        }
    }
}

// -------------------------------------------------------------------------
// Fragmentation experiment types (raw FFI layout, mirrors timsdata_common.h)
// -------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentationSettingsCollisionEnergy {
    pub ev: f64,
    pub percent: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentationSettingsCcid {
    pub collision_energy: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentationSettingsIscid {
    pub collision_energy: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FragmentationSettingsCcidSweep {
    pub collision_energy: f64,
    pub sweep_energies: *mut FragmentationSettingsCollisionEnergy,
    pub nr_sweep_energies: usize,
    pub first_sweep_energy: FragmentationSettingsCollisionEnergy,
    pub last_sweep_energy: FragmentationSettingsCollisionEnergy,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentationSettingsEtd {
    pub reaction_time_ms: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentationSettingsExd {
    pub reaction_time_ms: i64,
    pub electron_energy: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union FragmentationSettingsUnion {
    pub ccid: FragmentationSettingsCcid,
    pub ccid_sweep: FragmentationSettingsCcidSweep,
    pub etd: FragmentationSettingsEtd,
    pub exd: FragmentationSettingsExd,
    pub iscid: FragmentationSettingsIscid,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FragmentationSettings {
    pub type_: ::std::os::raw::c_int,
    pub settings: FragmentationSettingsUnion,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TimsIsolation {
    pub scan_begin: ::std::os::raw::c_int,
    pub scan_end: ::std::os::raw::c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IsolationSetting {
    pub mz: f64,
    pub width: f64,
    pub optional_tims_isolation: TimsIsolation,
    pub has_tims_isolation: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsolationFragmentationStep {
    pub optional_isolation: IsolationSetting,
    pub has_isolation: bool,
    pub fragmentation: FragmentationSettings,
    pub optional_precursor_id: i64,
    pub has_precursor_id: bool,
    pub optional_prm_target_id: i64,
    pub has_prm_target_id: bool,
}

pub const FRAGMENTATION_TYPE_NONE: i32 = 0;
pub const FRAGMENTATION_TYPE_CCID: i32 = 1;
pub const FRAGMENTATION_TYPE_CCID_SWEEPING: i32 = 2;
pub const FRAGMENTATION_TYPE_ETD: i32 = 3;
pub const FRAGMENTATION_TYPE_EXD: i32 = 4;
pub const FRAGMENTATION_TYPE_ISCID: i32 = 5;
pub const FRAGMENTATION_TYPE_UNKNOWN: i32 = 100;

// -------------------------------------------------------------------------
// Safe, owned result types
// -------------------------------------------------------------------------

/// A centroided (peak-picked) spectrum returned by the extract/read functions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CentroidedSpectrum {
    pub mz_values: Vec<f64>,
    pub area_values: Vec<f32>,
}

/// A finished chromatogram trace produced by [`TimsData::extract_chromatograms`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChromatogramTrace {
    pub id: i64,
    pub frame_ids: Vec<i64>,
    pub values: Vec<u64>,
}

/// Isolation settings of a fragmentation step.
#[derive(Clone, Debug, PartialEq)]
pub struct Isolation {
    pub mz: f64,
    pub width: f64,
    /// `(scan_begin, scan_end)` (end exclusive) when TIMS isolation is defined.
    pub tims_isolation: Option<(i32, i32)>,
}

/// Fragmentation method and its settings for a single step.
#[derive(Clone, Debug, PartialEq)]
pub enum Fragmentation {
    None,
    Ccid {
        collision_energy: f64,
    },
    CcidSweeping {
        collision_energy: f64,
        first_sweep_energy: (f64, f64),
        last_sweep_energy: (f64, f64),
    },
    Etd {
        reaction_time_ms: i64,
    },
    Exd {
        reaction_time_ms: i64,
        electron_energy: f64,
    },
    Iscid {
        collision_energy: f64,
    },
    Unknown,
}

/// A single fragmentation step of a fragmentation experiment.
#[derive(Clone, Debug, PartialEq)]
pub struct FragmentationStep {
    pub isolation: Option<Isolation>,
    pub fragmentation: Fragmentation,
    pub precursor_id: Option<i64>,
    pub prm_target_id: Option<i64>,
}

impl From<IsolationFragmentationStep> for FragmentationStep {
    fn from(raw: IsolationFragmentationStep) -> Self {
        let isolation = if raw.has_isolation {
            let iso = raw.optional_isolation;
            Some(Isolation {
                mz: iso.mz,
                width: iso.width,
                tims_isolation: if iso.has_tims_isolation {
                    Some((
                        iso.optional_tims_isolation.scan_begin,
                        iso.optional_tims_isolation.scan_end,
                    ))
                } else {
                    None
                },
            })
        } else {
            None
        };

        // Reading the union is sound: the active variant is selected by `type_`.
        let fragmentation = unsafe {
            let s = &raw.fragmentation.settings;
            match raw.fragmentation.type_ {
                FRAGMENTATION_TYPE_NONE => Fragmentation::None,
                FRAGMENTATION_TYPE_CCID => Fragmentation::Ccid {
                    collision_energy: s.ccid.collision_energy,
                },
                FRAGMENTATION_TYPE_ISCID => Fragmentation::Iscid {
                    collision_energy: s.iscid.collision_energy,
                },
                FRAGMENTATION_TYPE_ETD => Fragmentation::Etd {
                    reaction_time_ms: s.etd.reaction_time_ms,
                },
                FRAGMENTATION_TYPE_EXD => Fragmentation::Exd {
                    reaction_time_ms: s.exd.reaction_time_ms,
                    electron_energy: s.exd.electron_energy,
                },
                FRAGMENTATION_TYPE_CCID_SWEEPING => {
                    let sw = s.ccid_sweep;
                    Fragmentation::CcidSweeping {
                        collision_energy: sw.collision_energy,
                        first_sweep_energy: (
                            sw.first_sweep_energy.ev,
                            sw.first_sweep_energy.percent,
                        ),
                        last_sweep_energy: (
                            sw.last_sweep_energy.ev,
                            sw.last_sweep_energy.percent,
                        ),
                    }
                },
                _ => Fragmentation::Unknown,
            }
        };

        FragmentationStep {
            isolation,
            fragmentation,
            precursor_id: raw
                .has_precursor_id
                .then_some(raw.optional_precursor_id),
            prm_target_id: raw
                .has_prm_target_id
                .then_some(raw.optional_prm_target_id),
        }
    }
}

// -------------------------------------------------------------------------
// Shared callbacks and helpers
// -------------------------------------------------------------------------

/// Callback that stores a centroided spectrum into a `HashMap<i64, (Vec<f64>, Vec<f32>)>`.
unsafe extern "C" fn collect_centroided_into_map(
    id: i64,
    num_peaks: u32,
    mz_values: *const f64,
    area_values: *const f32,
    user_data: *mut c_void,
) {
    let map =
        unsafe { &mut *(user_data as *mut HashMap<i64, (Vec<f64>, Vec<f32>)>) };
    let (mz, area) = if num_peaks != 0
        && !mz_values.is_null()
        && !area_values.is_null()
    {
        (
            unsafe { slice::from_raw_parts(mz_values, num_peaks as usize) }
                .to_vec(),
            unsafe { slice::from_raw_parts(area_values, num_peaks as usize) }
                .to_vec(),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    map.insert(id, (mz, area));
}

/// Callback that stores a profile spectrum into a `HashMap<i64, Vec<i32>>`.
unsafe extern "C" fn collect_profile_into_map(
    id: i64,
    num_points: u32,
    intensity_values: *const i32,
    user_data: *mut c_void,
) {
    let map = unsafe { &mut *(user_data as *mut HashMap<i64, Vec<i32>>) };
    let values = if num_points != 0 && !intensity_values.is_null() {
        unsafe { slice::from_raw_parts(intensity_values, num_points as usize) }
            .to_vec()
    } else {
        Vec::new()
    };
    map.insert(id, values);
}

/// Runs a single-frame centroided extraction and returns the produced spectrum.
fn collect_single_centroided(
    frame_id: i64,
    call: impl FnOnce(*mut c_void) -> u32,
) -> CentroidedSpectrum {
    let results: HashMap<i64, (Vec<f64>, Vec<f32>)> = HashMap::new();
    let pointer_to_results = Box::into_raw(Box::new(results));
    let r = call(pointer_to_results as *mut c_void);
    if r == 0 {
        drop(unsafe { Box::from_raw(pointer_to_results) });
        panic!(
            "Could not extract centroided spectrum for frame {}. Error: {}",
            frame_id,
            get_last_error()
        );
    }
    let mut map = *unsafe { Box::from_raw(pointer_to_results) };
    let (mz_values, area_values) = map.remove(&frame_id).unwrap_or_default();
    CentroidedSpectrum {
        mz_values,
        area_values,
    }
}

/// Runs a single-frame profile extraction and returns the produced spectrum.
fn collect_single_profile(
    frame_id: i64,
    call: impl FnOnce(*mut c_void) -> u32,
) -> Vec<i32> {
    let results: HashMap<i64, Vec<i32>> = HashMap::new();
    let pointer_to_results = Box::into_raw(Box::new(results));
    let r = call(pointer_to_results as *mut c_void);
    if r == 0 {
        drop(unsafe { Box::from_raw(pointer_to_results) });
        panic!(
            "Could not extract profile spectrum for frame {}. Error: {}",
            frame_id,
            get_last_error()
        );
    }
    let mut map = *unsafe { Box::from_raw(pointer_to_results) };
    map.remove(&frame_id).unwrap_or_default()
}

/// Shared implementation for the frame-dependent coordinate conversions.
fn convert_values(
    handle: u64,
    frame_id: i64,
    in_: Vec<f64>,
    func: unsafe extern "C" fn(u64, i64, *const f64, *mut f64, u32) -> u32,
    what: &str,
) -> Vec<f64> {
    let input_count = in_.len() as u32;
    let mut output_values = vec![0.0f64; input_count as usize];
    let result = unsafe {
        func(
            handle,
            frame_id,
            in_.as_ptr(),
            output_values.as_mut_ptr(),
            input_count,
        )
    };
    if result == 0 {
        panic!(
            "Could not get {} from frame {}. Error: {}",
            what,
            frame_id,
            get_last_error()
        );
    }
    output_values
}

// -------------------------------------------------------------------------
// Standalone (handle-less) functions
// -------------------------------------------------------------------------

/// Converts a 1/K0 value to CCS (Å²) using the Mason-Schamp equation.
pub fn one_over_k0_to_ccs_for_mz(ook0: f64, charge: i32, mz: f64) -> f64 {
    unsafe {
        tims_oneoverk0_to_ccs_for_mz(ook0, charge as ::std::os::raw::c_int, mz)
    }
}

/// Converts a CCS (Å²) value to 1/K0 using the Mason-Schamp equation.
pub fn ccs_to_one_over_k0_for_mz(ccs: f64, charge: i32, mz: f64) -> f64 {
    unsafe {
        tims_ccs_to_oneoverk0_for_mz(ccs, charge as ::std::os::raw::c_int, mz)
    }
}

/// Calculates a mass axis from a transformator string. Returns `None` on failure.
pub fn get_mass_axis_from_trafo_string(
    trafo_string: &str,
    num_values: usize,
) -> Option<Vec<f64>> {
    let c_trafo = CString::new(trafo_string).ok()?;
    let mut buffer = vec![0.0f64; num_values];
    let code = unsafe {
        getMassAxisFromTrafoString(
            c_trafo.as_ptr(),
            buffer.as_mut_ptr(),
            num_values as ::std::os::raw::c_int,
        )
    };
    if code == 0 { Some(buffer) } else { None }
}

#[doc = " Function type that takes a centroided peak list."]
pub type MsmsSpectrumFunction = ::std::option::Option<
    unsafe extern "C" fn(
        id: i64,
        num_peaks: u32,
        mz_values: *const f64,
        area_values: *const f32,
        user_data: *mut c_void,
    ),
>;

#[doc = " Function type that takes a (non-centroided) profile spectrum."]
pub type MsmsProfileSpectrumFunction = ::std::option::Option<
    unsafe extern "C" fn(
        id: i64,
        num_points: u32,
        intensity_values: *const i32,
        user_data: *mut ::std::os::raw::c_void,
    ),
>;

#[doc = " A function that transforms every value of the input array 'in' to a corresponding value in"]
#[doc = " the output array 'out'. How many values it transforms is specified by the last argument. The"]
#[doc = " individual transformations are independent of each other."]
pub type BdalTimsConversionFunction = ::std::option::Option<
    unsafe extern "C" fn(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32,
>;

#[doc = " A chromatogram extraction job, i.e., the definition of a chromatogram trace. The value of a"]
#[doc = " chromatogram trace point at a given time (i.e., for a given frame in the TDF) is determined"]
#[doc = " by summing up the intensities of all peaks in that frame which fall into the specified m/z"]
#[doc = " and 1/K0 window."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TimsChromatogramJob {
    pub id: i64,
    pub time_begin: f64,
    pub time_end: f64,
    pub mz_min: f64,
    pub mz_max: f64,
    pub ook0_min: f64,
    pub ook0_max: f64,
}

#[doc = " A user-provided function called by the DLL whenever it is ready to process a new"]
#[doc = " job. \\returns 0 on error (chromatogram generation will then stop with an error), 1 when a"]
#[doc = " new job has been produced, 2 when no more jobs are available"]
pub type ChromatogramJobGenerator = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *mut TimsChromatogramJob,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32,
>;

#[doc = " Callback used to send a finished chromatogram traces from the DLL to the user. \\returns 0 on"]
#[doc = " error (chromatogram generation will then stop with an error), 1 when no error."]
pub type ChromatogramTraceSink = ::std::option::Option<
    unsafe extern "C" fn(
        id: i64,
        num_points: u32,
        frame_ids: *const i64,
        values: *const u64,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32,
>;

#[link(name = "timsdata")]
unsafe extern "C" {

    #[doc = " Return the last error as a string (thread-local)."]
    #[doc = ""]
    #[doc = " \\param buf pointer to a buffer into which the error string will be written."]
    #[doc = ""]
    #[doc = " \\param length length of the buffer"]
    #[doc = ""]
    #[doc = " \\returns the actual length of the error message (including the final zero"]
    #[doc = " byte). If this is longer than the input parameter 'length', you know that the"]
    #[doc = " returned error string was truncated to fit in the provided buffer."]
    #[doc = ""]
    pub fn tims_get_last_error_string(buf: *mut c_char, length: u32) -> u32;

    #[doc = " Open data set."]
    #[doc = ""]
    #[doc = " On success, returns a non-zero instance handle that needs to be passed to"]
    #[doc = " subsequent API calls, in particular to the required call to tims_close()."]
    #[doc = ""]
    #[doc = " On failure, returns 0, and you can use tims_get_last_error_string() to obtain a"]
    #[doc = " string describing the problem."]
    #[doc = ""]
    #[doc = " Uses NoPressureCompensation."]
    #[doc = ""]
    #[doc = " \\param analysis_directory_name the name of the directory in the file system that"]
    #[doc = " contains the analysis data, in UTF-8 encoding."]
    #[doc = ""]
    #[doc = " \\param use_recalibrated_state if non-zero, use the most recent recalibrated state"]
    #[doc = " of the analysis, if there is one; if zero, use the original \"raw\" calibration"]
    #[doc = " written during acquisition time."]
    #[doc = ""]
    pub fn tims_open(
        analysis_directory_name: *const ::std::os::raw::c_char,
        use_recalibrated_state: u32,
    ) -> u64;

    #[doc = " Open data set."]
    #[doc = ""]
    #[doc = " On success, returns a non-zero instance handle that needs to be passed to"]
    #[doc = " subsequent API calls, in particular to the required call to tims_close()."]
    #[doc = ""]
    #[doc = " On failure, returns 0, and you can use tims_get_last_error_string() to obtain a"]
    #[doc = " string describing the problem."]
    #[doc = ""]
    #[doc = " \\param analysis_directory_name the name of the directory in the file system that"]
    #[doc = " contains the analysis data, in UTF-8 encoding."]
    #[doc = ""]
    #[doc = " \\param use_recalibrated_state if non-zero, use the most recent recalibrated state"]
    #[doc = " of the analysis, if there is one; if zero, use the original \"raw\" calibration"]
    #[doc = " written during acquisition time."]
    #[doc = ""]
    #[doc = " \\param pressure_compensation_strategy the pressure compensation strategy"]
    #[doc = ""]
    pub fn tims_open_v2(
        analysis_directory_name: *const c_char,
        use_recalibrated_state: u32,
        pressure_compensation_strategy: PressureCompensationStrategy,
    ) -> u64;

    #[doc = " Close data set."]
    #[doc = ""]
    #[doc = " \\param handle obtained by tims_open(); passing 0 is ok and has no effect."]
    #[doc = ""]
    pub fn tims_close(handle: u64);

    #[doc = " Returns 1 if the raw data have been recalibrated after acquisition, e.g. in the"]
    #[doc = " DataAnalysis software. Note that masses and 1/K0 values in the raw-data SQLite"]
    #[doc = " file are always in the raw calibration state, not the recalibrated state."]
    #[doc = ""]
    pub fn tims_has_recalibrated_state(handle: u64) -> u32;

    #[doc = " Read a range of scans from a single frame."]
    #[doc = ""]
    #[doc = " Output layout: (N = scan_end - scan_begin = number of requested scans)"]
    #[doc = "   N x uint32_t: number of peaks in each of the N requested scans"]
    #[doc = "   N x (two uint32_t arrays: first indices, then intensities)"]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error, otherwise the number of buffer bytes necessary for the output"]
    #[doc = " of this call (if this is larger than the provided buffer length, the result is not"]
    #[doc = " complete)."]
    #[doc = ""]
    pub fn tims_read_scans_v2(
        handle: u64,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        buf: *mut c_void,
        length: u32,
    ) -> u32;

    #[doc = " Read peak-picked MS/MS spectra for a list of PASEF precursors."]
    #[doc = ""]
    #[doc = " Given a list of PASEF precursor IDs, this function reads all necessary PASEF"]
    #[doc = " frames, sums up the corresponding scan-number ranges into synthetic profile"]
    #[doc = " spectra for each precursor, performs centroiding using an algorithm and parameters"]
    #[doc = " suggested by Bruker, and returns the resulting MS/MS spectra (one for each"]
    #[doc = " precursor ID)."]
    #[doc = ""]
    #[doc = " Note: the order of the returned MS/MS spectra does not necessarily match the"]
    #[doc = " order in the specified precursor ID list. The parameter id in the callback is the"]
    #[doc = " precursor ID."]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_read_pasef_msms_v2(
        handle: u64,
        precursors: *const i64,
        num_precursors: u32,
        callback: MsmsSpectrumFunction,
        user_data: *mut c_void,
    ) -> u32;

    #[doc = " Read peak-picked MS/MS spectra for all PASEF precursors from a given frame."]
    #[doc = ""]
    #[doc = " Given a frame id, this function reads all contained PASEF precursors the necessary PASEF"]
    #[doc = " frames in the same way as tims_read_pasef_msms."]
    #[doc = ""]
    #[doc = " Note: the order of the returned MS/MS spectra does not necessarily match the"]
    #[doc = " order in the specified precursor ID list. The parameter id in the callback is the"]
    #[doc = " precursor ID."]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_read_pasef_msms_for_frame_v2(
        handle: u64,
        frame_id: i64,
        callback: MsmsSpectrumFunction,
        user_data: *mut c_void,
    ) -> u32;

    #[doc = " Read \"quasi profile\" MS/MS spectra for all PASEF precursors from a given frame."]
    #[doc = ""]
    #[doc = " Given a list of PASEF precursor IDs, this function reads all necessary PASEF"]
    #[doc = " frames, sums up the corresponding scan-number ranges into synthetic profile"]
    #[doc = " spectra for each precursor. These \"quasi\" profile spectra are passed back - one"]
    #[doc = " for each precursor ID."]
    #[doc = ""]
    #[doc = " Note: the order of the returned MS/MS spectra does not necessarily match the"]
    #[doc = " order in the specified precursor ID list. The parameter id in the callback is the"]
    #[doc = " precursor ID."]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_read_pasef_profile_msms_v2(
        handle: u64,
        precursors: *const i64,
        num_precursors: u32,
        callback: MsmsProfileSpectrumFunction,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32;

    #[doc = " Read \"quasi profile\" MS/MS spectra for all PASEF precursors from a given frame."]
    #[doc = ""]
    #[doc = " Given a frame id, this function reads for all contained PASEF precursors the necessary PASEF"]
    #[doc = " frames in the same way as tims_read_pasef_profile_msms."]
    #[doc = ""]
    #[doc = " Note: the order of the returned MS/MS spectra does not necessarily match the"]
    #[doc = " order in the specified precursor ID list. The parameter id in the callback is the"]
    #[doc = " precursor ID."]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_read_pasef_profile_msms_for_frame_v2(
        handle: u64,
        frame_id: i64,
        callback: MsmsProfileSpectrumFunction,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32;

    #[doc = " Read peak-picked spectra for a tims frame."]
    #[doc = ""]
    #[doc = " Given a frame ID, this function reads the frame,"]
    #[doc = " sums up the corresponding scan-number ranges into a synthetic profile"]
    #[doc = " spectrum, performs centroiding using an algorithm and parameters"]
    #[doc = " suggested by Bruker, and returns the resulting spectrum (exactly one for"]
    #[doc = " the frame ID)."]
    #[doc = ""]
    #[doc = " Note: Result callback identical to the tims_read_pasef_msms_v2 methods, but"]
    #[doc = " only returns a single result and the parameter id is the frame_id"]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_extract_centroided_spectrum_for_frame_v2(
        handle: u64,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        callback: MsmsSpectrumFunction,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32;

    #[doc = " Read peak-picked spectra for a tims frame with a custom peak picker resolution."]
    #[doc = ""]
    #[doc = " Same as tims_extract_centroided_spectrum_for_frame_v2(),"]
    #[doc = " but a user supplied resolution for the peak picker is applied."]
    #[doc = " Can be used to prevent invalid split peaks in case of low ion statistics."]
    #[doc = " The default suggested value in tims_extract_centroided_spectrum_for_frame_v2()"]
    #[doc = " is determined by the GlobalMetadata entry \"PeakWidthEstimateValue\" as"]
    #[doc = " 1 / PeakWidthEstimateValue for \"PeakWidthEstimateType\" = 1."]
    #[doc = ""]
    #[doc = " Note: Result callback identical to the tims_read_pasef_msms_v2 methods, but"]
    #[doc = " only returns a single result and the parameter id is the frame_id"]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_extract_centroided_spectrum_for_frame_ext(
        handle: u64,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        peakFinderResolution: f64,
        callback: MsmsSpectrumFunction,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32;

    #[doc = " Read \"quasi profile\" spectra for a tims frame."]
    #[doc = ""]
    #[doc = " Given a frame ID, this function reads the frame,"]
    #[doc = " and sums up the corresponding scan-number ranges into a synthetic profile"]
    #[doc = " spectrum. These \"quasi\" profile spectrum is passed back."]
    #[doc = ""]
    #[doc = " Note: Result callback identical to the tims_read_pasef_profile_msms_v2 methods,"]
    #[doc = " but only returns a single result and the parameter id is the frame_id"]
    #[doc = ""]
    #[doc = " Note: different threads must not read scans from the same storage handle"]
    #[doc = " concurrently."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    #[doc = ""]
    pub fn tims_extract_profile_for_frame(
        handle: u64,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        callback: MsmsProfileSpectrumFunction,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32;

    #[doc = " Extract several (MS1-only) chromatograms from an analysis."]
    #[doc = ""]
    #[doc = " The DLL retrieves the jobs (i.e., the chromatogram definitions) from the specified generator"]
    #[doc = " function while iterating through the analysis. The jobs must be delivered in the order of"]
    #[doc = " ascending 'time_begin'."]
    #[doc = ""]
    #[doc = " The DLL delivers chromatogram traces to the specified sink callback as soon as they are"]
    #[doc = " finished. When an error occurs, some of the jobs \"pulled\" so far might not be answered."]
    #[doc = ""]
    #[doc = " \\returns 0 on error"]
    pub fn tims_extract_chromatograms(
        handle: u64,
        get_job: ChromatogramJobGenerator,
        deliver_trace: ChromatogramTraceSink,
        user_data: *mut ::std::os::raw::c_void,
    ) -> u32;

    #[doc = " m/z transformation: convert back and forth between (possibly non-integer) index"]
    #[doc = " values and m/z values."]
    pub fn tims_index_to_mz(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32;

    pub fn tims_mz_to_index(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32;

    #[doc = " mobility transformation: convert back and forth between (possibly non-integer)"]
    #[doc = " scan numbers and 1/K0 values."]
    pub fn tims_scannum_to_oneoverk0(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32;

    pub fn tims_oneoverk0_to_scannum(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32;

    #[doc = " mobility transformation: convert back and forth between (possibly non-integer)"]
    #[doc = " scan numbers and TIMS voltages."]
    pub fn tims_scannum_to_voltage(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32;

    pub fn tims_voltage_to_scannum(
        handle: u64,
        frame_id: i64,
        in_: *const f64,
        out: *mut f64,
        cnt: u32,
    ) -> u32;

    #[doc = " Set the number of threads that this DLL is allowed to use internally. [The"]
    #[doc = " index<->m/z transformation is internally parallelized using OpenMP; this call is"]
    #[doc = " simply forwarded to omp_set_num_threads(). Has no effect on Linux]."]
    #[doc = ""]
    #[doc = " \\param n number of threads to use (n must be >= 1)."]
    #[doc = ""]
    pub fn tims_set_num_threads(n: u32);

    #[doc = " Converts the 1/K0 value to CCS (in Angstrom^2) using the Mason-Shamp equation"]
    #[doc = " \\param ook0 the 1/K0 value in Vs/cm2"]
    #[doc = " \\param charge the charge"]
    #[doc = " \\param mz the mz of the ion"]
    #[doc = " \\returns the CCS value in Angstrom^2"]
    pub fn tims_oneoverk0_to_ccs_for_mz(
        ook0: f64,
        charge: ::std::os::raw::c_int,
        mz: f64,
    ) -> f64;

    #[doc = " Converts the CCS (in Angstrom^2) to 1/K0 using the Mason-Shamp equation"]
    #[doc = " \\param ccs the ccs value in Angstrom^2"]
    #[doc = " \\param charge the charge"]
    #[doc = " \\param mz the mz of the ion"]
    #[doc = " \\returns the 1/K0 value in Vs/cm2"]
    pub fn tims_ccs_to_oneoverk0_for_mz(
        ccs: f64,
        charge: ::std::os::raw::c_int,
        mz: f64,
    ) -> f64;

    #[doc = " Open data set using a specific re-calibration identified by UUID."]
    pub fn tims_open_recalibration_id(
        analysis_directory_name: *const c_char,
        use_calibration_id: *const c_char,
    ) -> u64;

    #[doc = " Provides access to the current (re-)calibration id."]
    pub fn tims_get_calibration_id(
        handle: u64,
        buffer: *mut c_char,
        length: u32,
    ) -> u32;

    #[doc = " Read peak-picked spectra for a tims frame (v3, optimized for sparse data)."]
    pub fn tims_extract_centroided_spectrum_for_frame_v3(
        handle: u64,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        callback: MsmsSpectrumFunction,
        user_data: *mut c_void,
    ) -> u32;

    #[doc = " Read peak-picked spectra for a tims frame with a custom peak picker resolution (v3)."]
    pub fn tims_extract_centroided_spectrum_for_frame_ext_v3(
        handle: u64,
        frame_id: i64,
        scan_begin: u32,
        scan_end: u32,
        peakFinderResolution: f64,
        callback: MsmsSpectrumFunction,
        user_data: *mut c_void,
    ) -> u32;

    #[doc = " Get number of fragmentation experiments for a frame. Returns -1 on error."]
    pub fn tims_get_number_of_fragmentation_experiments(
        handle: u64,
        frame_id: i64,
    ) -> i64;

    #[doc = " Get number of steps in a fragmentation experiment. Returns -1 on error."]
    pub fn tims_get_number_of_fragmentation_experiment_steps(
        handle: u64,
        frame_id: i64,
        experiment_index: i32,
    ) -> i64;

    #[doc = " Get a single fragmentation step. Returns 0 on error, 1 on success."]
    pub fn tims_get_fragmentation_experiment_step(
        handle: u64,
        frame_id: i64,
        experiment_index: i32,
        step_index: i32,
        step: *mut IsolationFragmentationStep,
    ) -> u32;

    #[doc = " Calculate a mass axis from a transformator string. Returns 0 (CALRDR_SUCCESS) on success."]
    #[allow(non_snake_case)]
    pub fn getMassAxisFromTrafoString(
        trafo_string: *const c_char,
        buffer: *mut f64,
        num_values: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}

#[cfg(test)]
mod timsdata_tests {
    use crate::{
        PressureCompensationStrategy, TimsData, tims_close, tims_open_v2,
    };
    use std::ffi::CString;
    use std::path::PathBuf;

    const INPUT_ANALYSIS: &str = "/home/sander/data/raw/200spd/20231219_TIMS03_PaSk_SA_K562_ddaPASEF_50ng_7min_IM0713_S1-A3_1_41558.d/";

    fn get_analysis_path() -> PathBuf {
        INPUT_ANALYSIS.into()
    }

    #[test]
    fn test_open_analysis() {
        let analysis_path = get_analysis_path();
        let input_analysis =
            CString::new(analysis_path.into_os_string().into_string().unwrap())
                .unwrap();
        let handler = unsafe {
            tims_open_v2(
                input_analysis.as_ptr(),
                0,
                PressureCompensationStrategy::NoPressureCompensation,
            )
        };

        if handler == 0 {
            panic!("Handler is 0");
        }
        unsafe { tims_close(handler) };
    }

    #[test]
    fn test_open_analysis_creating_struct() {
        let analysis_path = get_analysis_path();
        let td = TimsData::new(
            analysis_path,
            false,
            PressureCompensationStrategy::NoPressureCompensation,
        );
        td.close();
    }

    #[test]
    fn test_read_scans() {
        let analysis_path = get_analysis_path();
        let mut td = TimsData::new(
            analysis_path,
            false,
            PressureCompensationStrategy::NoPressureCompensation,
        );
        let scans = td.read_scans(1, 0, 671);
        td.close();
        assert_eq!(scans.len(), 671)
    }

    #[test]
    fn test_read_spectra() {
        let analysis_path = get_analysis_path();
        let mut td = TimsData::new(
            analysis_path,
            false,
            PressureCompensationStrategy::NoPressureCompensation,
        );
        let spectra = td.read_pasef_msms_for_frame(1);
        td.close();
        assert_eq!(spectra.len(), 1)
    }
}
