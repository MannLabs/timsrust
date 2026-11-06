# Native acquisition-aware observations

## Status and scope

This branch adds an opt-in observation API for raw dia-PASEF acquisitions,
including experimental scan-wise transmission handling for diagonal isolation
trajectories. It does not replace the ordinary spectrum reader.

The implementation provides:

- Native centroid measurements with exact source identities and integer responses.
- Separate retention-time (RT) and ion-mobility (IM) context observations.
- Explicit missing observations, acquisition bounds, transmission masks, and
  collision-energy (CE) trajectories.
- Optional RT-supported admission of additional weak measured peaks.
- Native-preserving per-observation peak selection by default when RT support is
  enabled.
- Rust access, a command-line dataset exporter, versioned Parquet storage, and
  native Python extension access.

It does **not** provide joint coordinate fitting, pooled or summed RT spectra,
precursor deconvolution, peptide ownership assignments, a universal denoising
guarantee, or a sequence-accuracy claim. The experimental transmission policy
is not a default processing change.

## Measurement and acquisition contracts

A peak is a measurement in one acquired frame. Its identity is the triple
`(frame, scan, tof)`. The m/z, calibrated inverse mobility, and integer
kernel-weighted apex response belong to that source. Context selection never
replaces these values with a consensus coordinate or a support-weighted response.

`peak_apex_responses` is the authoritative integer response array. Spectrum
intensities are its floating-point representation, not a second measurement.
Keep integer responses when exact values above `2**53` matter. Response is not
integrated chromatographic area or summed raw ion volume.
RT support records also retain exact integer source responses; only ranking
scores and consensus coordinates are floating-point annotations.

RT is in **seconds**. RT offsets are same-group acquisition steps, not seconds
or arithmetic differences between frame IDs. IM offsets follow scan direction,
which need not follow increasing physical mobility. Calibrated mobility is
inverse reduced mobility. Scan and isolation-mass bounds are **half-open**.
Frame IDs need not be contiguous or zero-based.

### Acquisition policies

| Rust / Python name | CLI name | Meaning |
|---|---|---|
| `SingleInterval` / `single_interval` | `single-interval` | Use one nominal acquisition interval. |
| `SharedTransmission` / `shared_transmission` | `shared-transmission` | Cross contiguous intervals only when they transmit the reference mass at identical CE. |
| `ExperimentalTrajectory` / `experimental_trajectory` | `experimental-trajectory` | Use mass-transmission footprints and compatible scan-wise transmission/CE trajectories. |

The single-interval mode is the default. The other modes require acquisition
metadata and calibrated raw TDF access. A measured precursor mass can condition
the transmission test without establishing chemical ownership of any peak.
Window-first sampling instead uses the nominal interval center.

In the experimental mode, a single scalar CE is **not** a description of the
observation. Use `observation_domain` and the underlying acquisition rows.
Untransmitted or missing scan cells are unobserved, not measured zeros.
Incompatible CE trajectories, transmission gaps, cadence gaps, and acquisition
changes must not silently become RT support edges.

### Acquisition metadata versus spectrum summaries

The spectrum isolation interval is a conservative common mass interval for
the **retained source footprints**, seeded by the anchor interval. It is not
the complete physical acquisition trajectory. Rejected weak candidates do not
constrain that summary; it is recomputed after RT admission and capping.
Adding a retained source can legitimately narrow it.

Consequently, do not use a peak-dependent isolation summary to define an
otherwise fixed MS1 input window or to compare acquisition coverage between
admission policies. Use the original frame acquisition rows and requested
scan-wise domain for those purposes. Keep isotope-bearing MS1 observations
separate if the downstream application requires them; this MS2 observation
export does not synthesize MS1 inputs.

## Observation geometry and missingness

A neighborhood contains one focal `View` and an explicitly ordered list of
context slots. The focal offset is `(rt=0, im=0)`. Context offsets are ordered
by RT, then IM, excluding the focal slot. With RT extent two on each side and
no IM context, there are five observations.

Each available view has its own frame, anchor, actual RT, requested IM bounds,
clipped scan bounds, coverage flag, isolation summary, native arrays, and
optional aligned RT evidence. Requested bounds can extend beyond acquired
coverage. `fully_covered` describes IM coverage, not the presence of all RT
neighbors.

An unavailable observation carries an offset and reason, such as a run
boundary, acquisition change, center outside isolation, or time gap. It is
not interchangeable with an acquired view containing an empty peak array.
Consumers must preserve that distinction.

The default IM radius is half the learned native MS1 scan FWHM; optional
padding is expressed in FWHM units. Explicit scan radii/stride can be supplied
through `ImSizing::Scans`. Context extents and products are validated to avoid
unbounded geometry. Overlapping views are not independent acquired evidence.

### Configuration defaults

| Rust configuration | Default | Interpretation |
|---|---|---|
| `acquisition` | `SingleInterval` | No experimental transmission extension. |
| `rt_context` | Two steps before and after | Five RT slots including the focal view. |
| `im_context` | Zero before and after | No adjacent IM slots. |
| `im` | `NativeFwhm`, zero padding | Half-FWHM radii and half-FWHM stride, resolved from the run. |
| `peak_cap` | 300 | Per-view cap; zero disables capping. |
| `min_ms2_ion_count` | 2 | Native detection threshold; must be an integer. |
| `rt_consistency` | `None` | Weak-peak admission disabled. |
| `boundary` | `KeepMasked` | Keep partial/missing context explicitly. |

`RequireComplete` rejects incomplete neighborhoods instead. Extents are limited
to 32 steps per side and the complete RT/IM grid to 1,024 slots.
Inspect `resolved_config` rather than assuming a fixed scan width across runs.

## Optional RT-supported admission

Set `Config::rt_consistency` to enable this stage; `None` leaves it disabled.
The default qualification requires:

- At least three distinct acquired frames, including the source frame.
- Support on both earlier and later sides.
- Mutually unique mass/IM-compatible assignments.
- Pairwise-compatible members, not a transitive drift chain.
- Acquisition-eligible raw support under the selected transmission policy.

The default mass tolerance is 10 ppm. Parameters are validated against the
available configured context. Detection and qualification are source-centered;
context cropping does not redefine the source measurement.

Native peaks are not required to qualify for RT support in order to remain
native measurements. Weak peaks must qualify without ambiguity to be admitted.
RT evidence retains the native/weak flag, qualification, ambiguity, support
source identities/times, consensus coordinates, and selection score.
These are annotations, not replacement measurements or ownership probabilities.

### Peak selection

`peak_cap` applies **independently to each view**; zero means uncapped. Five
top-300 observations can contain up to 1,500 peak records. There is no pooled
global top-300 or implicit deduplication across time.

`NativePreserving` is the RT-enabled default. It protects the native response
top-N membership, with exact integer-response and deterministic coordinate
tie-breaking, and fills spare capacity with qualified weak sources.
This protects native peaks within that acquisition policy; it cannot promise
to retain the original single-interval set when switching acquisition policies.

Two explicit alternative RT ranking policies are available for controlled
assessment:

- `FocalResponse`: rank admitted measurements by source response.
- `SupportWeighted`: rank by response and qualified support count.

These alternatives can displace native peaks and are not the native-protection
guarantee. No research-only reserve or rescue policies are exposed.

If preserving the complete original measurement set is required, retain an
uncapped original channel **separately** from capped augmented views. Retention
does not grant a source clean-borrowing eligibility or create new RT edges.
There is no general guarantee that both complete channels fit a fixed cap.

## Rust usage

```rust
use timsrust::{
    NativeNeighborhoodReader,
    centroid::neighborhood::{
        AcquisitionPolicy, Anchor, Config, RtConsistencyConfig,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = NativeNeighborhoodReader::new("example.d", Config {
        acquisition: AcquisitionPolicy::ExperimentalTrajectory,
        peak_cap: 300,
        rt_consistency: Some(RtConsistencyConfig::default()),
        ..Config::default()
    })?;
    let bundle = reader.neighborhood_for_precursor(
        Anchor { frame: 100, interval: 2, scan: 500 },
        650.25,
    )?;
    let domain = reader.observation_domain(&bundle.focal)?;
    println!("{domain:#?}");
    // Keep bundle.context as separate observations.
    Ok(())
}
```

Coordinates above are illustrative, not valid for arbitrary files. Use the
file's acquisition metadata or a measured feature to obtain real anchors.

Additional entry points:

- `neighborhood_at`: explicit window-first coordinates without grid snapping.
- `neighborhoods_at`: explicit anchors with a bounded shared cache.
- `neighborhoods_with_precursors`: explicit measured features with metadata.
- `neighborhoods`: window-first iteration.
- `precursor_neighborhoods`: measured-MS1-feature sampling.
- `pseudospectra`: focal-spectrum projection, **not** a context union.
- `resolved_config` and `frame_infos`: inspect resolved geometry/acquisition.

The ordinary spectrum reader and its processing defaults are unaffected.
The feature reader propagates invalid input, calibration, acquisition, and
I/O errors rather than silently reverting to another processing mode.

Run the bounded explicit-coordinate example:

```sh
cargo run -p timsrust --example native_observations -- \
  example.d FRAME INTERVAL SCAN PRECURSOR_MZ
# Add --rt after the coordinates to enable supported weak-peak admission.
```

## Command-line export

```sh
cargo run -p timsrust-centroid-cli -- \
  example.d --neighborhoods --sampling window \
  --acquisition experimental-trajectory --peak-cap 300 \
  --rt-before 2 --rt-after 2 --im-neighbors 0 \
  --rt-centroiding=false --max-bundles 10 -o native-context
```

Use `--rt-centroiding=true` for the RT-admission alternative and a different,
fresh output directory. `--sampling precursor` uses measured MS1 features;
window-first sampling does not require feature detection. The CLI's default
export cap is zero; the Rust configuration defaults to 300. Set caps
explicitly for controlled comparisons.

`--max-bundles` limits the exported prefix and is recorded in the manifest.
It is a smoke-test limit, not a statistically representative sample. The
acquisition table may include metadata outside that prefix.

Legacy file output remains separate. Neighborhood options require
`--neighborhoods`; invalid options fail explicitly.

## Parquet and Python access

The dataset consists of a completion manifest and three Parquet tables:

- `observations.parquet`: stable slots, availability, geometry and peak ranges.
- `peaks.parquet`: measured m/z, per-peak mobility, integer response, source
  coordinates, and optional RT evidence.
- `acquisition.parquet`: original frame acquisition rows with scan ranges,
  isolation bounds, CE and acquisition-group information.

The manifest records schema version, units, response definition, resolved
kernel width, requested configuration, counts and optional bundle limit.
Only finalized datasets are readable as complete. Writers require a fresh
directory, so existing results are not silently overwritten.

The Rust storage API is `timsrust_parquet_spectra::neighborhoods::{Writer, Reader}`.
Its reader reconstructs neighborhoods and measured precursor metadata.
Treat unknown versions, incomplete exports, invalid row references and
inconsistent counts as failures, not empty successful input.

Native Python installation and the directly callable API are documented in
[the Python package guide](../python/timsrust-pyo3/README.md).
Python integer responses preserve values above the exact float64 integer
range. Neither a Python environment nor a raw acquisition is bundled.

## How to assess the feature

Separate these questions:

1. **Correctness:** do unchanged source identities retain exact coordinates,
   integer responses, masks, support and missingness?
2. **Acquisition coverage:** what changes when the transmission policy changes?
3. **Context usefulness:** what changes when neighbors are added to an
   otherwise identical native focal observation?
4. **Admission usefulness:** what changes when RT-qualified weak sources are
   added without changing native top-N membership?
5. **Application accuracy:** does a fixed downstream procedure improve on
   independent, prespecified truth?

A clean MS2 ablation uses the same acquisition policy and cap:

| Arm | MS2 input |
|---|---|
| A | Trajectory-native focal only |
| B | The same trajectory-native focal plus native context |
| C | Trajectory RT-admission focal plus context |

Keep any MS1 inputs, normalization, encoding, decoding and references fixed
across arms. Comparing original single-interval focal input with trajectory
context confounds acquisition coverage with context. Preserve measurements
outside model inputs separately, rather than silently adding them to one arm.

Count both gains and losses. Collapse repeated source observations, charge
states and complementary fragment sides only when the endpoint calls for it;
repeated evidence may itself be useful to a model. Do not equate raw match
counts with independent cleavage constraints.

Prefer acquisition-disjoint panels and independent reference material.
References selected because the focal already contains a fragment have
limited headroom and cannot establish independent sequence discovery.
Shifted masses are incidental-match controls, not chemical false labels or
an automatic FDR estimate. Separate engineering fixtures, conditional fragment
compatibility, chemical ownership, and final application accuracy.

## Validation and known limitations

The portable regression suite uses synthetic acquisitions and raw snippets
constructed in tests; no proprietary files, labels, credentials or benchmark
artifacts are required. It covers:

- Stable native values, integer ranking, uncapped and capped membership.
- Missing, empty and partial observations; actual sparse frame IDs.
- Transmission gaps, CE transitions and changing scan-wise trajectories.
- Source-centered support, ambiguity and cadence barriers.
- Raw-support verification and rejection of unsupported weak sources.
- Selected-source isolation summaries rather than rejected-candidate bounds.
- Versioned storage, exact round-trips and Python error propagation.

Run the focused checks from the repository root:

```sh
cargo test -p timsrust-centroid -p timsrust-parquet-spectra \
  -p timsrust-centroid-cli -p timsrust --lib
cargo fmt --all -- --check
# See the Python guide for extension build and test commands.
```

The CLI also provides an explicitly ignored raw-data integration test using
`TIMSRUST_TEST_DIA`. It is optional and requires a suitable local file; absence
of that file must not be reported as a successful live-data test.

Remaining limitations:

- Raw calibrated DIA TDF support, not a universal reader for every format.
- A run-global calibration can still contain physical mass/mobility drift.
- Acquisition compatibility does not prove common chemical origin.
- Weak support can be ambiguous or absent, especially near boundaries.
- Fixed caps impose real trade-offs; no policy promises all useful evidence.
- MS1 observation assembly and application-specific inference are separate.
- No general sequence-accuracy, purity or false-discovery improvement is claimed.
