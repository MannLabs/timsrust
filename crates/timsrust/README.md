# timsrust

[![Crates.io](https://img.shields.io/crates/v/timsrust)](https://crates.io/crates/timsrust)
[![docs.rs](https://img.shields.io/docsrs/timsrust)](https://docs.rs/timsrust)

Facade crate for reading Bruker timsTOF data. Re-exports the public API of the
`timsrust-*` sub-crates so that most users only need a single dependency.

## Installation

```toml
[dependencies]
timsrust = "0.5"
```

Optional features:

| Feature   | Description                                      |
|-----------|--------------------------------------------------|
| `sdk`     | Bruker SDK-backed reading via `timsrust-sdk`     |
| `patched` | Patched calibration support via `timsrust-patched` |

## Core types

| Type | Description |
|------|-------------|
| `TimsTofPath` | Entry point — auto-detects TDF / miniTDF / Parquet from a path string |
| `SpectrumReader` / `SpectrumReaderBuilder` | Read MS2 spectra |
| `PrecursorReader` / `PrecursorReaderBuilder` | Read precursor information |
| `MzConverter`, `ImConverter`, `RtConverter` | Unit converters (raw → m/z, ion mobility, retention time) |

Sub-crates are also re-exported as modules for lower-level access:

```rust
timsrust::core      // timsrust-core
timsrust::tdf       // timsrust-tdf
timsrust::minidf    // timsrust-minitdf
timsrust::centroid  // timsrust-centroid
```

## Supported file formats

- **TDF** — Bruker `.d` folder (`analysis.tdf` + `analysis.tdf_bin`)
- **miniTDF** — ProteoScape optimised format (`*.ms2spectrum.bin` + `*.ms2spectrum.parquet`)
- **Parquet spectra** — columnar spectrum storage

## Usage

```rust
use timsrust::{TimsTofPath, TimsTofPathLike};

let path = TimsTofPath::new("/data/sample.d")?;

// Read all spectra in parallel
let reader = path.spectrum_reader()?;

// Read precursors
let precursors = path.precursor_reader()?;
```
