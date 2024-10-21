
![Crates.io](https://img.shields.io/crates/v/timsrust?link=https%3A%2F%2Fcrates.io%2Fcrates%2Ftimsrust)
![docs.rs](https://img.shields.io/docsrs/timsrust?link=https%3A%2F%2Fdocs.rs%2Ftimsrust%2F0.2.1%2Ftimsrust%2F)

# TimsRust

A crate to read Bruker TimsTof data.

## Stability

**NOTE**: TimsRust does not yet have a stable version! Use with caution.

## Installation

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
timsrust = "x.x.x"
```

## Usage

TimsRust is intended to be used as a library and not as a stand-alone application. An example of how to use it is found in e.g. [Sage](https://github.com/lazear/sage).

### Basics

Two primary data types are exposed through TimsRust:
* Spectra: A traditional representation that expresses intensitites in function of mz values for a given precursor.
* Frames: All recorded data from a single TIMS elution (i.e. at one specific retention_time).

### File formats

Two file formats are supported:
* TDF - Bruker .d folder containing:
    * analysis.tdf
    * analysis.tdf_bin

* miniTDF - ProteoScape optimized Bruker file-format. Similar to TDF, miniTDF consists of multiple files: a binary '.bin'
 and an index '.parquet' file. The file-names are made up to the following convention: `<producing-engine-name>.<domain-name>.<extension>`.
 e.g. for MS2 spectrum information: `<producing-engine-name>.ms2spectrum.<extension>`. Therefore the following files are expected
 in the provided ms2 folder:
    * *.ms2spectrum.bin
    * *.ms2spectrum.parquet


## Python bindings

The [timsrust_pyo3](https://github.com/jspaezp/timsrust_pyo3) package is an example of how the performance of TimsRust can be utilized in Python

## Planned changes for future versions
TODO
* Improve docs
* Improve tests
* Pase CompressionType1
* Tarred file reader
* Clean up src (FrameReader, ...)
* Cleaner try_from conversions/readers
* Make Path of TimsTOF data into special type
* Single access point for all readers?
* Few unchecked unwraps left
* Queryable data in all dimensions
* ...
