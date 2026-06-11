
[![Crates.io](https://img.shields.io/crates/v/timsrust)](https://crates.io/crates/timsrust)
[![docs.rs](https://img.shields.io/docsrs/timsrust)](https://docs.rs/timsrust/)
![License](https://img.shields.io/crates/l/timsrust)

# TimsRust

A high-performance Rust ecosystem for reading and preprocessing Bruker timsTOF mass spectrometry data. TimsRust provides type-safe, composable abstractions for working with liquid chromatography coupled to trapped ion mobility spectrometry (LC-TIMS-TOF) data in multiple formats.

**Status**: Actively developed and production-ready. Used in the [Sage](https://github.com/lazear/sage) proteomics search engine and other mass spectrometry workflows.

## Features

- **Multiple file formats**: TDF (.d folders), miniTDF, TSF (MALDI/imaging), and Parquet
- **Rich data model**: Frames (2D ion mobility arrays), Spectra (traditional m/z vs. intensity), and Precursor information
- **Type-safe coordinates**: Dimensional analysis via compile-time types prevents unit confusion
- **Flexible readers**: Composable reader traits for custom data pipelines
- **High performance**: Parallel I/O with rayon; optional Bruker SDK calibration for maximum accuracy
- **Custom calibration**: Swap `timsrust-patched` implementations via Cargo's `[patch.crates-io]` mechanism
- **Python support**: Bindings via PyO3 for scientific Python workflows
- **CLI tools**: Centroiding, MGF export, and format conversion utilities

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
timsrust = "0.5"
```

Optional features:
- `sdk` — Use Bruker SDK for calibration (requires SDK binary; see [Using the Bruker SDK](#using-the-bruker-sdk))
- `patched` — Use custom algorithms (via `[patch.crates-io]`)

### Basic Usage

```rust
use timsrust::TimsTofPath;

// Auto-detects the format (TDF, miniTDF, Parquet).
let path = TimsTofPath::new("/path/to/data.d")?;

// Read all spectra (centroided m/z + intensity arrays).
let spectrum_reader = path.spectrum_reader()?;
for index in 0..spectrum_reader.len() {
    let spectrum = spectrum_reader.get(index)?;
    if let Some(precursor) = spectrum.precursor() {
        println!("Precursor m/z: {}", precursor.mz());
    }
    for (mz, intensity) in spectrum.mz_values().iter().zip(spectrum.intensities()) {
        println!("  m/z: {}, intensity: {}", mz, intensity);
    }
}

// Read precursor information.
let precursor_reader = path.precursor_reader()?;
for index in 0..precursor_reader.len() {
    let precursor = precursor_reader.get(index)?;
    println!(
        "index: {}, m/z: {}, charge: {:?}, RT: {}, IM: {}",
        precursor.index(),
        precursor.mz(),
        precursor.charge(),
        precursor.rt(),
        precursor.im(),
    );
}

// Read raw 2D frame data (TDF only).
let frame_reader = path.frame_reader()?;
for index in frame_reader.iter_indices() {
    let frame = frame_reader.get_frame(index)?;
    println!("Frame at RT: {} s", frame.info().rt_in_seconds());
    let ions = frame.ions();
    for scan_index in 0..ions.scan_count() {
        for (tof_index, intensity_index) in ions.read_scan(scan_index) {
            // tof_index and intensity_index are raw, need converters for physical units.
            let _ = (scan_index, tof_index, intensity_index);
        }
    }
}
```

With the `sdk` feature enabled, the same code uses Bruker SDK calibration
automatically (see [Using the Bruker SDK](#using-the-bruker-sdk)).

### Examples

Runnable examples live in [crates/timsrust/examples/](crates/timsrust/examples/) and can be executed with `cargo run --release --example <name> -- <args>`:

| Example | Description |
|---------|-------------|
| [read_spectra](crates/timsrust/examples/read_spectra.rs) | Iterate centroided spectra serially and print precursor m/z + peak counts |
| [read_spectra_parallel](crates/timsrust/examples/read_spectra_parallel.rs) | Iterate spectra in parallel via `SpectrumReader::par_iter` |
| [read_precursors](crates/timsrust/examples/read_precursors.rs) | Print the precursor table (m/z, charge, RT, IM) |
| [read_frames](crates/timsrust/examples/read_frames.rs) | Read raw 2D frame data and convert TOF/scan indices to m/z and ion mobility (TDF only) |
| [auto_detect_format](crates/timsrust/examples/auto_detect_format.rs) | Open any supported format through `TimsTofPath` and report capabilities |
| [convert_to_mgf](crates/timsrust/examples/convert_to_mgf.rs) | Export spectra to a Mascot Generic Format file using `timsrust-mgf` |
| [with_sdk](crates/timsrust/examples/with_sdk.rs) | Use the Bruker SDK calibration backend (requires `--features sdk`) |
| [with_patched](crates/timsrust/examples/with_patched.rs) | Use a `timsrust-patched` calibration backend (requires `--features patched`) |

## Understanding TimsRust Data Model

### What is TimsTOF Data?

TimsTOF instruments combine **liquid chromatography (LC)** with **trapped ion mobility spectrometry (TIMS)** and **time-of-flight (TOF)** mass analysis. This creates a 3D data matrix:

1. **Retention Time (RT)** – from LC column
2. **Ion Mobility (IM)** – how fast an ion drifts through the gas-filled TIMS analyzer (depends on shape & charge)
3. **Mass-to-Charge (m/z)** – from TOF measurement

### Key Data Structures

TimsRust exposes three primary abstractions:

#### 1. **Frame** — 2D ion cloud at one retention time
A `Frame` contains all ions recorded during one LC elution (one TIMS cycle).
Its ions are stored as parallel arrays grouped by scan (ion mobility) offsets:
- **Scans**: discrete ion mobility steps (rows)
- **TOF indices**: raw time-of-flight bins per ion (columns)
- **Intensities**: per-ion intensity counts

```rust
use timsrust::core::Converter;

let frame = frame_reader.get_frame(index)?;
let ions = frame.ions();
for scan_index in 0..ions.scan_count() {
    for (tof_index, intensity_index) in ions.read_scan(scan_index) {
        // Use converters to translate raw indices to physical units.
        let mz = mz_converter.convert(tof_index);
        let _ = (scan_index, mz, intensity_index);
    }
}
```

#### 2. **Spectrum** — Traditional m/z vs. intensity array
A `Spectrum` represents centroided MS data: for a given (optional) precursor it
stores parallel arrays of m/z values and summed intensities. This is the
familiar input format for database search engines.

```rust
let spectrum = spectrum_reader.get(index)?;
if let Some(precursor) = spectrum.precursor() {
    println!("Precursor m/z: {}", precursor.mz());
}
for (mz, intensity) in spectrum.mz_values().iter().zip(spectrum.intensities()) {
    // Fragment ion at (mz, intensity).
    let _ = (mz, intensity);
}
```

#### 3. **Precursor** — Metadata about a precursor ion
A `Precursor` captures information about the parent ion before fragmentation:
m/z, charge (optional), intensity (optional), retention time, ion mobility, and
the scan/frame indices where it was selected.

```rust
let precursor = precursor_reader.get(index)?;
println!(
    "m/z: {}, charge: {:?}, intensity: {:?}, RT: {}, IM: {}",
    precursor.mz(),
    precursor.charge(),
    precursor.intensity(),
    precursor.rt(),
    precursor.im(),
);
```

### Coordinate Systems & Converters

Raw data in TDF/miniTDF files uses **index space**: integers representing discrete samples. TimsRust provides converters to transform indices into physical units:

| Converter | Input | Output |
|-----------|-------|--------|
| `MzConverter` | TOF index | m/z (mass-to-charge) |
| `ImConverter` | Scan index | Ion mobility (1/K₀) |
| `RtConverter` | Frame index | Retention time (seconds) |

```rust
use timsrust::core::{Converter, TofIndex};

let mz_converter = path.mz_converter().expect("No calibration data");
let tof_index = TofIndex::try_from(12345_u32)?;
let mz = mz_converter.convert(tof_index);
println!("m/z: {}", mz);
```

### Acquisition Types

TimsTOF instruments support different acquisition modes, captured in `AcquisitionType`:

- **DDA-PASEF** (Data-Dependent Acquisition): Traditional MS/MS: select precursors above intensity threshold, fragment them
- **DIA-PASEF** (Data-Independent Acquisition): Fragment all ions in overlapping mass windows; no precursor selection needed
- **Diagonal DIA-PASEF**: Variant with selective fragmentation

TimsRust's readers work with all modes; your downstream code may respond differently (e.g., DIA doesn't have explicit precursor lists).

## Project Overview

TimsRust is organized as a **modular Rust workspace** with specialized crates for different concerns:

```
timsrust/                           # Workspace root
├── crates/
│   ├── timsrust                   # Facade crate (re-exports public API from all readers)
│   ├── timsrust-core              # Core types: Frame, Spectrum, Precursor, Converters
│   ├── timsrust-tdf               # TDF format reader (.d folders)
│   ├── timsrust-minitdf           # miniTDF format reader (ProteoScape)
│   ├── timsrust-tsf               # TSF format reader (MALDI/imaging)
│   ├── timsrust-parquet-spectra   # Parquet format reader
│   ├── timsrust-centroid          # Centroiding algorithms
│   ├── timsrust-mgf               # MGF export (Mascot Generic Format)
│   ├── timsrust-sdk               # Bruker SDK C FFI bindings
│   ├── timsrust-utils             # Shared utilities (readers, buffers)
│   └── filemanager                # Cross-platform file I/O abstraction
├── clis/
│   ├── timsrust-centroid-cli      # Centroiding CLI tool
│   ├── timsrust-mgf-cli           # MGF export CLI
│   └── ...                        # Additional utilities
└── python/
    └── timsrust-pyo3              # Python bindings (PyO3)
```

### Dependency Hierarchy

- **timsrust-core** is the foundation: defines all data types and reader traits
- **timsrust** is the recommended entry point for most users: a facade that re-exports core + all format readers
- Format-specific readers (timsrust-tdf, timsrust-minitdf, etc.) depend only on timsrust-core
- Task-specific crates (centroid, mgf) are optional and composable
- Python bindings wrap timsrust for PyO3 integration

## Core API

### Entry Point: `TimsTofPath`

`TimsTofPath` auto-detects the file format and provides a unified interface:

```rust
// Create from a path string
let path = TimsTofPath::new("/data/sample.d")?;

// Create readers
let spectrum_reader = path.spectrum_reader()?;
let precursor_reader = path.precursor_reader()?;
let frame_reader = path.frame_reader()?; // TDF only

// Create converters
let mz = path.mz_converter();
let im = path.im_converter();
let rt = path.rt_converter();
```

### Readers: Builders

Readers can also be constructed via their builders, which is the recommended
way when you need to set non-default options:

```rust
use rayon::prelude::*;
use timsrust::SpectrumReader;

let reader = SpectrumReader::build()
    .with_path(&path)
    .finalize()?;

// Parallel iteration over all spectra.
let peak_counts: Vec<usize> = reader
    .par_iter()
    .filter_map(|spectrum| spectrum.ok().map(|s| s.len()))
    .collect();
```

### Type-Safe Coordinates

TimsRust uses newtype wrappers to prevent unit confusion at compile time.
Index types (`TofIndex`, `ScanIndex`, `FrameIndex`) are constructed via
`TryFrom`, and value types (`Mz`, `Im`, `Rt`) via `From<f64>`:

```rust
use timsrust::core::{Converter, FrameIndex, Im, Mz, Rt, ScanIndex, TofIndex};

let tof = TofIndex::try_from(1000_u32)?;
let im: Im = im_converter.convert(ScanIndex::try_from(50_u32)?);
let mz: Mz = mz_converter.convert(tof);

// The converters only accept matching (index, value) type pairs, so e.g.
// passing a ScanIndex into mz_converter.convert(...) is a compile error.
```

## Using the Bruker SDK

The Bruker TimsData SDK provides proprietary calibration algorithms that typically yield ±5 ppm m/z accuracy (vs. ±10–20 ppm with pure Rust methods).

### Downloading & Installing

1. Download the SDK from [Bruker Mass Spectrometry Raw Data Access Libraries](https://www.bruker.com/protected/en/services/software-downloads/mass-spectrometry/raw-data-access-libraries.html)
   - Requires free registration
   - Available for Linux, Windows

2. Extract the binary (typically `libtimsdata.so` on Linux, `timsdata.dll` on Windows)

3. **Option A**: Copy to system path or library folder
   ```bash
   # Linux example
   cp libtimsdata.so /usr/local/lib/
   ```

4. **Option B**: Use environment variable
   ```bash
   export LD_LIBRARY_PATH=/path/to/sdk:$LD_LIBRARY_PATH
   ```

### Enabling in Your Project

Add the `sdk` feature to `Cargo.toml`:

```toml
[dependencies]
timsrust = { version = "0.5", features = ["sdk"] }
```

The SDK is used automatically when the `sdk` feature is enabled; user code is
identical to the non-SDK version:

```rust
let path = TimsTofPath::new("/data/sample.d")?;
let spectrum_reader = path.spectrum_reader()?;
let mz_converter = path.mz_converter().expect("No calibration data");
```

The selected calibration backend (SDK vs. pure-Rust TDF metadata vs. patched)
is chosen at runtime based on enabled features and the file format.

## Custom Calibration with `timsrust-patched`

TimsRust can use the optional `timsrust-patched` crate as a replacement calibration backend. This is useful when you want to use calibration logic that is not part of the default pure-Rust TDF metadata path and you do not want to depend on the Bruker SDK at runtime.

At the moment, `timsrust-patched` is used for TOF index to m/z calibration. Other extension points, such as custom frame readers, spectrum readers, or format-specific readers, may be supported through this crate family in the future, but they are not part of the current `timsrust-patched` integration.

### Enabling Patched Calibration

Enable the `patched` feature on the facade crate:

```toml
[dependencies]
timsrust = { version = "0.5", features = ["patched"] }
```

With this feature enabled, `MzConverter::new` prefers `timsrust-patched::Tof2MzConverter` for TDF data. Your application code does not need to change:

```rust
let path = TimsTofPath::new("/data/sample.d")?;
let mz_converter = path.mz_converter().expect("No calibration data");
```

### Replacing `timsrust-patched`

To test or deploy your own calibration implementation, patch `timsrust-patched` in your `Cargo.toml`:

```toml
[dependencies]
timsrust = { version = "0.5", features = ["patched"] }

[patch.crates-io]
timsrust-patched = { path = "../my-timsrust-patched" }
```

You can also patch from a Git repository:

```toml
[patch.crates-io]
timsrust-patched = { git = "https://github.com/myorg/timsrust-patched", branch = "custom-calibration" }
```

Your replacement crate must provide the same public API expected by TimsRust, currently including `Tof2MzConverter::from_tdf` and the converter implementation used for TOF index to m/z conversion.

## CLI Tools

TimsRust includes specialized command-line utilities:

### Centroiding CLI

Convert raw frame data to centroided peaks. The output format is selected by
the extension of `--out-path` (`.parquet`, `.mgf`, or `.spec.parquet`):

```bash
timsrust-centroid-cli /path/to/data.d --out-path data.centroided.parquet
timsrust-centroid-cli /path/to/data.d --out-path data.centroided.mgf
```

### MGF Export CLI

Export spectra in Mascot Generic Format (compatible with database search
engines):

```bash
timsrust-mgf-cli /path/to/data.d data.mgf
```

Run any CLI with `--help` to see all available options.

## Python Support

The [timsrust-pyo3](https://github.com/MannLabs/timsrust-pyo3) package binds TimsRust to Python via PyO3, enabling high-performance MS data processing in Python workflows.

### Installation

```bash
pip install timsrust-pyo3
```

### Basic Usage

```python
import tims_pyo3

# Read all spectra in one call.
spectra = tims_pyo3.read_all_spectra("/path/to/data.d")
for spectrum in spectra:
    # Each spectrum exposes mz_values, intensities and an optional precursor.
    print(spectrum)

# Or use the FrameReader / SpectrumReader iterators.
frame_reader = tims_pyo3.FrameReader("/path/to/data.d")
for frame in frame_reader:
    print(frame)

spectrum_reader = tims_pyo3.SpectrumReader("/path/to/data.d")
for spectrum in spectrum_reader:
    print(spectrum)
```

## Supported File Formats

### TDF (Bruker Raw Format)

- **Files**: `.d` folder containing `analysis.tdf` (SQLite) and `analysis.tdf_bin` (binary ion data)
- **Readers**: `timsrust-tdf` crate; full frame access via `TimsTofPath::frame_reader()`
- **Converters**: Tof → m/z, Scan → IM, Frame → RT (all from TDF metadata)
- **Best for**: Full data access, frame-level processing

### miniTDF (ProteoScape Format)

- **Files**: Binary + Parquet index pairs, e.g., `*.ms2spectrum.bin` + `*.ms2spectrum.parquet`
- **Readers**: `timsrust-minitdf` crate
- **Converters**: Limited; often requires external calibration
- **Best for**: Cloud storage, space-efficient storage

### TSF (Bruker MALDI/Imaging)

- **Files**: `*.d` folder with `analysis.tdf` (MALDI-specific schema)
- **Readers**: `timsrust-tsf` crate
- **Best for**: MALDI mass spectrometry and MS imaging data

### Parquet (Columnar Spectra)

- **Files**: Arrow Parquet files with spectrum metadata + peaks
- **Readers**: `timsrust-parquet-spectra` crate
- **Best for**: Archived data, cross-platform exchange

## Contributing & Design Principles

We welcome contributions! Please follow these principles:

### Code Quality
- **Correctness first**: Type safety and memory safety are non-negotiable
- **Clear interfaces**: Public APIs should be obvious to users unfamiliar with the internals
- **Small, focused functions**: Keep functions under ~30 lines when practical
- **No magic numbers**: Named constants for all domain-specific values
- **Comprehensive docs**: Public types and functions must have `///` doc comments

### Error Handling
- Use `thiserror` for error types
- Propagate errors with `?`; avoid `.unwrap()` in library code
- `.expect()` allowed only when the error is provably unreachable (with explanatory comment)

### Performance
- Parallel iterators via rayon for CPU-bound workloads
- Reuse allocations; minimize intermediate collections
- Profile before optimizing; prefer readability when trade-offs are small

### Ownership & Borrowing
- Prefer `&str` and `&[T]` in function parameters over owned types
- `.clone()` should be justified and explicit

### Testing
- New public functions should have unit tests
- Tests go in `#[cfg(test)]` modules in the same file

### API Stability
- Never break public APIs without discussion
- Deprecate before removing
- Consider downstream users (e.g., Hermes, other search engines)

See [CONTRIBUTING.md](CONTRIBUTING.md) (if present) for detailed workflow.

## Troubleshooting

### "Unknown file format"

Ensure the path points to the **folder** for `.d` data (not a specific file):
```rust
// ✅ Correct
let path = TimsTofPath::new("/data/sample.d")?;
```

### M/Z values look wrong

1. Check if SDK is available: `path.mz_converter()` may return `None` or a fallback method
2. Enable `sdk` feature and install the Bruker SDK (see [Using the Bruker SDK](#using-the-bruker-sdk))
3. Verify the data file includes calibration metadata (some formats lack this)

### Performance is slow

- Use the **parallel iterator**: `SpectrumReader::par_iter()` (rayon `ParallelIterator`)
- Enable **release builds**: `cargo build --release`
- Consider frame-level reading via `TdfFrameReader` if you need raw 2D data
- Profile with `cargo flamegraph` or similar to identify bottlenecks

### "Can't find libtimsdata.so" or similar

The Bruker SDK library is not in the system path:
```bash
# Check the LD_LIBRARY_PATH
echo $LD_LIBRARY_PATH

# Temporarily add the SDK folder
export LD_LIBRARY_PATH=/path/to/sdk:$LD_LIBRARY_PATH
cargo run
```

Or copy the library to a standard location (`/usr/local/lib`, etc.).

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.
