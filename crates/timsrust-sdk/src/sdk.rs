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
