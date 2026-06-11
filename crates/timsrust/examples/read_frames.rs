//! Read raw 2D frame data from a TDF dataset and convert indices to physical
//! units using the matching converters.
//!
//! Usage:
//!     cargo run --release --example read_frames -- /path/to/data.d
//!
//! Frame access is only supported for TDF (`.d`) inputs; the example will
//! report an error otherwise.

use timsrust::{TimsTofPath, core::Converter};

const PREVIEW_FRAMES: usize = 3;
const PREVIEW_IONS_PER_FRAME: usize = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_path = std::env::args()
        .nth(1)
        .expect("usage: read_frames <path-to-tdf-data>");

    let path = TimsTofPath::new(&raw_path)?;
    let frame_reader = path.frame_reader()?;
    let mz_converter = path.mz_converter().expect("no m/z calibration");
    let im_converter = path.im_converter().expect("no IM calibration");

    let indices: Vec<_> =
        frame_reader.iter_indices().take(PREVIEW_FRAMES).collect();

    for index in indices {
        let frame = frame_reader.get_frame(index)?;
        let info = frame.info();
        let ions = frame.ions();
        println!(
            "frame {} @ rt={:.3}s, ms_level={:?}, {} scans, {} ions",
            info.index(),
            info.rt_in_seconds(),
            info.ms_level(),
            ions.scan_count(),
            ions.intensities().len(),
        );

        let mut shown = 0;
        'outer: for scan_index in 0..ions.scan_count() {
            let scan_index_typed = match u32::try_from(scan_index)
                .ok()
                .and_then(|v| timsrust::core::ScanIndex::try_from(v).ok())
            {
                Some(v) => v,
                None => continue,
            };
            let im = im_converter.convert(scan_index_typed);
            for (tof_index, intensity_index) in ions.read_scan(scan_index) {
                let mz = mz_converter.convert(tof_index);
                let intensity = u32::from(intensity_index);
                println!(
                    "    scan={scan_index} im={im} m/z={mz} intensity={intensity}",
                );
                shown += 1;
                if shown >= PREVIEW_IONS_PER_FRAME {
                    break 'outer;
                }
            }
        }
    }
    Ok(())
}
