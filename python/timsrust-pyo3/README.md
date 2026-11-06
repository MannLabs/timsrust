# Native observation/context API

The compiled `tims_pyo3` extension exposes native diaPASEF observations directly.
Existing frame/spectrum readers are unchanged. No Python Parquet dependency is
required.

```python
from tims_pyo3 import (
    NativeNeighborhoodConfig, NativeNeighborhoodReader,
    ParquetNeighborhoodReader, RtConsistencyConfig,
)

config = NativeNeighborhoodConfig(
    acquisition="shared_transmission",
    rt_context=(2, 2),             # acquisition steps before/after
    im_context=(0, 0),             # scan-direction steps before/after
    im_padding=(0.5, 0.5),         # extra native scan-kernel FWHM
    peak_cap=300,
    min_ms2_ion_count=2,
    rt_consistency=RtConsistencyConfig(),  # optional, native-preserving
)
reader = NativeNeighborhoodReader("acquisition.d", config)
resolved = reader.resolved_config()
frames = reader.frame_infos()
frame = frames[0]
interval = frame["acquisition"][0]
anchor = (
    frame["frame"], interval["interval"],
    (interval["scan_start"] + interval["scan_end"] - 1) // 2,
)
bundle = reader.read_at(*anchor, precursor_mz=500.0)
for peak in bundle["focal"]["peaks"]:
    assert isinstance(peak["apex_response"], int)

# Explicit, bounded queries; not implicit whole-run materialization.
batch = reader.read_many([anchor], precursor_mzs=[500.0])
reader.export("observations", [anchor], precursor_mzs=[500.0])
saved = ParquetNeighborhoodReader("observations")
assert len(saved) == 1
restored = saved.get(0)  # equivalently saved[0]
```

Choose a precursor mass actually transmitted at the chosen anchor. Omitting it
uses the native reader's interval-centered precursor mass; that is a query
coordinate, not a measured precursor feature. `measured_precursor` is separate
and is `None` for direct native queries and explicit exports.

## Configuration and errors

All configuration arguments are named keywords. `to_dict()` returns an ordinary
mapping, not JSON. `resolved_config()` additionally reports calibrated native
scan FWHM, effective left/right widths, stride and context step.

* `acquisition`: `single_interval` (default), `shared_transmission`, or
  `experimental_trajectory`. The last is explicitly experimental.
* `rt_context=(before, after)`: same-group acquisition steps, not frame-ID
  arithmetic and not seconds. Each extent is at most 32; RT/IM slot product is
  at most 1024.
* `im_context=(before, after)`: increasing scan direction, not increasing
  physical inverse mobility.
* Either `im_padding=(left, right)` in native FWHM units (default zero) **or**
  `im_scans=(left, right, stride)` in scans. Stride must be positive.
* `peak_cap=300`; zero is uncapped. `min_ms2_ion_count=2` is an integer threshold
  of at least one.
* `require_complete=False` retains masked boundary context. Setting it true
  requests complete coverage according to the native reader.
* `rt_consistency=None` disables RT qualification. `RtConsistencyConfig` accepts
  `mz_ppm=10.0`, `minimum_support=3` (including focal frame), and
  `require_both_sides=True`. `selection="native_preserving"` is the default:
  native top-N membership is protected and spare capacity may admit supported
  peaks. Explicit alternatives are `"support_weighted"` and `"focal_response"`;
  these change admission/ranking, never measured coordinates or responses.

Invalid configuration, batch lengths or precursor mass raises `ValueError`;
invalid Python types raise `TypeError` (out-of-range unsigned integers raise
`OverflowError`). File/native read failures, including invalid source anchors,
raise `OSError`. Parquet indices are
nonnegative; indices at or beyond `len(reader)` raise `IndexError`.

## Returned measurements

Results are detached Python dictionaries/lists; `tims_pyo3.pyi` documents the
shape. Mutating a result never changes the native reader.

* `focal` is a view; `context` contains slots with signed `rt_offset` and
  `im_offset`. An unavailable slot has `available=False`, `view=None`, a reason
  and optional source frame. An acquired but empty slot has `available=True`
  and a view with `peaks=[]`. These states are not interchangeable.
* Frame, interval, scan, TOF and spectrum IDs retain native source identity.
  Scan/interval indices are zero-based; frame IDs are source IDs, not positions
  in `frame_infos()`. Scan ranges are half-open. Requested scan bounds can be
  negative; actual bounds are clipped. `fully_covered` describes IM coverage,
  not uniform RT spacing.
* `rt_s` is in seconds; `inverse_mobility` is inverse reduced mobility.
  Every peak has its own measured mobility, raw scan/TOF, measured `mz`, and
  exact unsigned-64-bit `apex_response` represented as a Python `int`, including
  values above 2**53. Response means native-kernel-weighted apex, **not**
  integrated ion volume.
* Optional `rt_evidence` contains native/qualified/ambiguous flags, support rows
  with actual frame/time/scan/TOF/mass/response, consensus coordinates and
  `selection_score`. Support responses are also exact unsigned-64-bit Python
  integers. Consensus coordinates and selection scores remain floating-point
  evidence; they never replace measured coordinates or responses.
  `rt_evidence_present` distinguishes disabled RT mode even for empty views.
* `isolation_summary` is explicitly labeled
  `conservative_selected_source_support_intersection`. It is peak-dependent
  and can change after capping or RT admission. It is **not** an authoritative
  acquisition range or a scan-wise collision-energy schedule.
* `acquisition` contains the separate raw acquisition rows for that view's
  source frame, including interval/group IDs, RT, scan bounds, isolation bounds
  and collision energy. `observation_domain` uses these rows and the query
  precursor mass to report scan-wise transmission masks and collision energies,
  including untransmitted/missing tails. A missing row has energy `None`;
  an acquired nontransmitting row can still have an energy.
  These acquisition rows and domains are authoritative, not the scalar summary.

## Scope, memory and I/O

`read_many` and `export` accept **1–1024 explicit anchors**. Optional
`precursor_mzs` must have the same length and contain finite positive values.
These are bounded explicit queries, not a streaming whole-acquisition iterator.
`read_many` materializes its results; uncapped peaks/context can still consume
substantial memory. It does not promise a shared cross-anchor frame cache.
`export` writes each requested bundle in turn and finalizes a completion
manifest only on success. Its destination must not already exist. A failed
export can leave an incomplete directory, which the reader will reject.

`ParquetNeighborhoodReader` retains acquisition metadata and reads individual
bundle peak ranges with `get`; it does not eagerly materialize all peaks.
`acquisition()` returns the full metadata table. Constructors, native
extraction, Parquet reads and export release the GIL; conversion into Python
objects runs with the GIL. Native raw calibration is required for raw reads.

## Development

Build/install with `maturin develop` or build a wheel with `maturin build`.
`cargo test -p timsrust-pyo3` runs synthetic native mapping/Parquet tests without
the extension-only linking feature. The focused Python tests use `pytest
tests/test_neighborhoods.py` after installing the extension. To also exercise
Python Parquet round trips, set `TIMSRUST_PYTHON_TEST_DATASET` to a new
project-local dataset directory when running the Rust tests, then use the same
environment variable for pytest. No acquisition data is required.
Optionally set `TIMSRUST_NATIVE_TEST_PATH` to an available raw diaPASEF
acquisition to test native extraction, batch equality and bounded export too.
