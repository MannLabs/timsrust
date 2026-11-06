# Native observations: choosing, combining, and evaluating modes

This guide explains what the feature does, which choices are independent, and
how to tell whether a particular combination helps your application.

**Availability:** build from `feat/native-observation-context`. These APIs are
not yet part of a released package. They require calibrated raw dia-PASEF TDF
data; scan-wise handling of diagonal isolation trajectories is experimental.
The ordinary spectrum reader and its processing defaults are unchanged.
This branch does include the separate
[centroid/acquisition correctness fixes](../README.md#centroid-and-acquisition-membership-corrections).
Reprocessing an older release can therefore change output; the observation
feature does not implicitly enable context or RT admission in the ordinary API.

## Contents

1. [The idea in one minute](#1-the-idea-in-one-minute)
2. [The five choices that define a configuration](#2-the-five-choices-that-define-a-configuration)
3. [Which combinations are supported?](#3-which-combinations-are-supported)
4. [Practical recipes](#4-practical-recipes)
5. [How to evaluate the combinations fairly](#5-how-to-evaluate-the-combinations-fairly)
6. [Understanding the output](#6-understanding-the-output)
7. [Configuration and API reference](#7-configuration-and-api-reference)
8. [Validation, limitations, and troubleshooting](#8-validation-limitations-and-troubleshooting)

## 1. The idea in one minute

Imagine an MS2 observation at one retention time (RT) and one ion-mobility (IM)
position.
Call it the **focal observation**. Instead of asking it to explain the entire
signal alone, we can also supply nearby observations:

```text
Same-group RT step:     -2       -1        0       +1       +2
Observation:         neighbor neighbor  FOCAL  neighbor neighbor
```

Each observation keeps its own measured peaks. The result is a **neighborhood**:
one focal view plus explicitly labeled context slots. RT steps are acquisition
steps, not seconds; each view also carries its actual acquisition time.

There are two different ways to use the extra information:

- **Provide context:** give an application the focal observation and its
  neighbors as separate inputs. No extra weak peaks need to be admitted.
- **Admit supported weak peaks:** allow an additional, locally measured weak
  peak when compatible observations support it across RT.

Neither operation adds neighboring intensities together or moves a measured
peak to an averaged coordinate. Native centroiding already smooths raw signal
in the scan/TOF dimensions; adding context is not another joint fit.

### What is, and is not, included

| Capability | This feature branch |
|---|---|
| Ordinary native centroiding | Preserved; the ordinary reader remains available. |
| MS1-precursor-first and DIA-isolation-first sampling | Both available in Rust and the CLI. |
| Separate RT and/or IM context | Available, with explicit missingness. |
| RT-supported weak-peak admission | Optional; native-preserving selection is the default. |
| Scan-wise transmission/CE handling | Explicit experimental acquisition policy. |
| Joint centroiding or joint coordinate fitting | **Not included.** |
| Merged/summed RT spectra or precursor deconvolution | **Not included.** |

Think of this as a set of composable choices, **not** three algorithms called
"standard", "enhanced", and "joint".

## 2. The five choices that define a configuration

| Choice | Question it answers | Main options |
|---|---|---|
| **Sampling** | Where do we request observations? | Detected MS1 precursors, or a DIA acquisition-window/IM grid. |
| **Acquisition policy** | Which acquisition rows may contribute? | Single interval, shared transmission, experimental trajectory. |
| **Context geometry** | Which neighboring observations do we return? | None, RT, IM, or RT and IM. |
| **Weak-peak admission** | May a supported weak source join the native peaks? | Off, or RT-supported admission. |
| **Selection and cap** | What survives a finite peak budget? | Native-preserving or explicit alternative ranking; cap per view. |

After these choices, the consumer still decides whether to use the focal view
alone or the whole neighborhood.

### Choice 1: precursor-first or isolation-first?

| | MS1-precursor-first | DIA-isolation/window-first |
|---|---|---|
| Starting point | A detected MS1 precursor feature. | An acquisition interval and an IM-grid position. |
| Reference mass | The measured precursor m/z. | The nominal interval center. |
| CLI | `--sampling precursor` | `--sampling window` |
| Rust iterator | `precursor_neighborhoods(...)` | `neighborhoods()` |
| Main use | Feature-conditioned MS2 observations. | Acquisition coverage without requiring a detected precursor feature. |
| Main caveat | Feature detection determines which anchors exist. | The window center is a query coordinate, not evidence for a real precursor. |

An **anchor** specifies `(frame, interval, scan)`: where to look, not the
identity of a fragment peak. Window-first sampling can place multiple anchors
across an interval's IM range; it is not necessarily one spectrum per window.
Both paths still require the native calibration and kernel-width estimation.

Precursor-first sampling does **not** make an MS2 observation chemically pure.
It also does not export a complete MS1 input channel for an application.

Python supports explicit anchors, optionally conditioned on supplied precursor
masses. It does **not** automatically discover MS1 features or expose the
Rust whole-run sampling iterators. Supplying a mass to Python is not equivalent
to running MS1-feature detection.

### Choice 2: how much acquisition structure do we respect?

| Policy | Plain-language meaning | When to assess it |
|---|---|---|
| **Single interval** | Stay within the nominal acquisition interval. | Start here for a simple control. This is the default. |
| **Shared transmission** | Cross contiguous rows only when they transmit the reference mass at identical collision energy (CE). | When nominal row boundaries split otherwise compatible transmission. |
| **Experimental trajectory** | Use mass-transmission footprints and compatible scan-wise transmission/CE trajectories. A single scalar CE is insufficient. | For acquisitions with changing isolation/CE trajectories, including diagonal layouts. |

The reference mass comes from the chosen sampling/query path. Therefore,
changing from a window center to a measured precursor mass can change the
transmission domain even at the same anchor.

Acquisition compatibility means "these measurements may be considered
together under the acquisition rules", **not** "these fragments belong to
the same molecule". More coverage can also mean more interference.
Changing this policy is a separate experiment from adding context or weak peaks.

### Choice 3: which neighbors do we return?

RT and IM extents are independently configurable:

| Example | RT `(before, after)` | IM `(before, after)` | Maximum slots, including focal |
|---|---|---|---|
| Focal only | `(0, 0)` | `(0, 0)` | 1 |
| RT context | `(2, 2)` | `(0, 0)` | 5 |
| IM context only | `(0, 0)` | `(1, 1)` | 3 |
| RT and IM context | `(2, 2)` | `(1, 1)` | 15 |

These are requested slots, not guarantees that every observation exists.
Run boundaries, transmission changes, time gaps, and IM boundaries can leave
unavailable or partial slots.

**Two IM controls do different things:**

- IM **width/padding** changes the scan aperture of each view.
- IM **context** adds views centered at neighboring scan positions.

Changing the aperture can change focal membership. Adding IM context should
not be described as simply widening the focal spectrum. Views may overlap,
so 15 slots do not mean 15 independent acquisitions.

### Choice 4: native peaks only, or RT-supported admission?

With admission **off**, views contain native centroid measurements selected
under the configured acquisition policy and cap. Context can still be returned.

With admission **on**, additional weak candidates must be measured locally
and have compatible, unambiguous RT support. By default this requires:

- At least three distinct acquired frames, including the source frame.
- Support both before and after the source frame.
- Mass/IM-compatible, mutually unique matches.
- Pairwise consistency, rather than a chain that gradually drifts.
- Raw support eligible under the acquisition policy.

The default mass tolerance is 10 ppm. Detection and matching use source-centered
domains; cropping an output view does not redefine the source measurement.

**What does not happen:** a missing focal peak is not copied in from a neighbor,
responses are not summed, and native peaks are not required to pass the
weak-peak support test. Thus this is not a general filter that removes every
nonpersistent native peak.

IM neighbors do not substitute for distinct RT frames. There is no separate
"IM-only weak-peak admission" switch.

### Choice 5: what fits into the peak budget?

The cap applies **to each view**, not to the entire neighborhood:

- Five top-300 views can contain up to 1,500 peak records.
- Fifteen top-300 views can contain up to 4,500 peak records.
- A cap of zero means uncapped, not empty.

With RT admission enabled, **native-preserving** is the default ranking policy.
It protects the native response top-N membership and uses spare capacity for
qualified weak peaks.
Native ordering uses exact integer responses and deterministic scan/TOF
tie-breaking, rather than rounded floating-point intensities.

For example:

| Native peaks before capping | Cap | Native-preserving outcome |
|---|---|---|
| 350 | 300 | The native top 300 remain; no spare slots for weak peaks. |
| 280 | 300 | All 280 remain; up to 20 qualified weak peaks may be added. |
| Any number | 0 | Native peaks and qualified weak peaks are uncapped. |

No weak additions at a full cap is an expected result, not a failed extraction.

Two explicit alternatives are available:

| Rust / Python selection | Ranking | Trade-off |
|---|---|---|
| `NativePreserving` / `native_preserving` | Protect native top-N, then fill by support score. | Conservative default. |
| `FocalResponse` / `focal_response` | Rank admitted sources by their own response. | A stronger weak source can displace a native peak. |
| `SupportWeighted` / `support_weighted` | Rank using response and qualified support count. | A persistent weaker source can displace a stronger native peak. |

Native protection is relative to the **same acquisition policy, anchor, and
view geometry**. It does not promise to preserve the original single-interval
set after changing acquisition policy or aperture.

## 3. Which combinations are supported?

### Sampling, acquisition, and admission

Every row below can return RT/IM context and use a per-view cap:

| Sampling | Acquisition policy | Native-only | RT-supported admission |
|---|---|---|---|
| Window-first | Single interval | Yes | Yes |
| Window-first | Shared transmission | Yes | Yes |
| Window-first | Experimental trajectory | Yes | Yes |
| Precursor-first | Single interval | Yes | Yes |
| Precursor-first | Shared transmission | Yes | Yes |
| Precursor-first | Experimental trajectory | Yes | Yes |

This table describes the Rust/CLI sampling paths. Python can explicitly query
the corresponding anchors with or without supplied masses, but does not
implement their automatic sampling.

**Supported does not mean every parameter combination is valid.** Default RT
admission needs a support window with earlier and later frames. It cannot be
enabled with `rt_context=(0, 0)`. Extents, support count, calibration, and
acquisition metadata are validated; invalid configurations fail explicitly.

### Support window versus consumer input

The frames needed to qualify a peak and the views passed to an application
are separate choices:

| Intended input | Extraction configuration | Consumer uses |
|---|---|---|
| Native focal spectrum | Native-only; context optional. | Focal view only. |
| Native augmented input | Native-only; RT and/or IM context enabled. | Focal plus separate context views. |
| RT-admitted focal spectrum | RT admission enabled with sufficient RT context. | Focal view only; neighbors were still used for qualification. |
| RT-admitted augmented input | RT admission enabled; chosen RT/IM geometry. | Focal plus separate context views. |

To compare focal-only and augmented inputs fairly, extract a native neighborhood
once and select its focal view for the focal-only arm. Do not silently change
the aperture, sampling population, or acquisition policy at the same time.

The Rust `pseudospectra()` projection returns focal spectra, **not** a union of
context peaks and not a deconvolution result. For precursor-conditioned focal
output, take the focal view from `precursor_neighborhoods(...)` instead.
Keep the full bundle when source identities, per-peak mobility, or RT evidence
are needed; a spectrum projection does not replace that metadata.

If complete original retention is required, keep an **uncapped original
channel separately** from capped augmented inputs. Retaining a source does
not authorize borrowing it across an otherwise incompatible acquisition edge.
There is no promise that both complete channels fit into a fixed budget.

## 4. Practical recipes

Start with native-only extraction. Add one choice at a time after the output
is understood. Use a fresh output directory for every export.

### CLI: three useful starting points

**Recipe 1: window-first, native peaks, five RT slots**

```sh
cargo run -p timsrust-centroid-cli -- \
  example.d --neighborhoods --sampling window \
  --acquisition single-interval --peak-cap 300 \
  --rt-before 2 --rt-after 2 --im-neighbors 0 \
  --rt-centroiding=false --max-bundles 10 -o window-native
```

**Recipe 2: the same settings, with RT-supported admission**

```sh
cargo run -p timsrust-centroid-cli -- \
  example.d --neighborhoods --sampling window \
  --acquisition single-interval --peak-cap 300 \
  --rt-before 2 --rt-after 2 --im-neighbors 0 \
  --rt-centroiding=true --max-bundles 10 -o window-rt
```

**Recipe 3: precursor-first native sampling**

```sh
cargo run -p timsrust-centroid-cli -- \
  example.d --neighborhoods --sampling precursor \
  --acquisition single-interval --peak-cap 300 \
  --rt-before 2 --rt-after 2 --im-neighbors 0 \
  --rt-centroiding=false --max-bundles 10 -o precursor-native
```

These recipes are controls, not universal recommended settings:

- Change `--acquisition` to `shared-transmission` or
  `experimental-trajectory` to assess acquisition handling.
- Change `--im-neighbors 0` to `--im-neighbors 1` for three IM slots at each RT.
- Use `--peak-cap 0` for an uncapped companion export.
- Keep sufficient RT context when enabling admission, even for focal-only use.
- Use Rust/Python for asymmetric IM context, explicit IM widths, or alternative
  RT qualification/ranking settings; these controls are not all CLI flags.

`--max-bundles` exports a prefix and records the limit in the manifest. Ten
bundles are a smoke test, **not** a representative evaluation panel. In
particular, prefixes of precursor-first and window-first exports are not
matched populations.

Neighborhood options require `--neighborhoods`. Without it, the legacy output
path remains separate and can overwrite an output file. The neighborhood
writer instead requires a new directory.

### Python: a paired native/admission comparison

Install the extension from this checkout as described in the
[Python package guide](../python/timsrust-pyo3/README.md).

The example below deliberately uses the same explicit anchor, reference mass,
geometry, cap, and acquisition policy for both readers. The coordinates and
mass are illustrative: replace them with valid values from your acquisition.

```python
from tims_pyo3 import (
    NativeNeighborhoodConfig,
    NativeNeighborhoodReader,
    ParquetNeighborhoodReader,
    RtConsistencyConfig,
)

common = dict(
    acquisition="experimental_trajectory",
    rt_context=(2, 2),
    im_context=(0, 0),
    peak_cap=300,
)
native = NativeNeighborhoodReader(
    "example.d", NativeNeighborhoodConfig(**common)
)
supported = NativeNeighborhoodReader(
    "example.d",
    NativeNeighborhoodConfig(**common, rt_consistency=RtConsistencyConfig()),
)
anchors = [(100, 2, 500)]
masses = [650.25]

native_bundles = native.read_many(anchors, precursor_mzs=masses)
supported_bundles = supported.read_many(anchors, precursor_mzs=masses)

focal_only = [bundle["focal"] for bundle in native_bundles]
native_augmented = native_bundles
supported_augmented = supported_bundles

supported.export("rt-observations", anchors, precursor_mzs=masses)
saved = ParquetNeighborhoodReader("rt-observations")
assert saved[0] == supported_bundles[0]
```

The three variables describe three different **consumer inputs**, not three
centroiding algorithms. Keep their encodings and any separate MS1 inputs
consistent when evaluating them.

Omit `precursor_mzs` to use interval-centered query masses. Supplied masses do
not create measured MS1-feature metadata: `measured_precursor` remains `None`
for these direct queries/exports.

Python `read_many` and `export` accept 1..1,024 explicit anchors. `read_many`
materializes its results and does not promise a shared cross-anchor frame
cache. Large uncapped neighborhoods can still require substantial memory.

### Rust: explicit coordinates and streaming sampling

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
    println!("{:#?}", reader.observation_domain(&bundle.focal)?);
    Ok(())
}
```

For automatic sampling, use `neighborhoods()` for window-first iteration or
`precursor_neighborhoods(min_ms1_ion_count)` for detected-MS1-feature sampling.
The latter retains measured precursor metadata and does not drop an anchor
merely because its focal MS2 view is empty.

A bounded runnable example is also provided:

```sh
cargo run -p timsrust --example native_observations -- \
  example.d FRAME INTERVAL SCAN PRECURSOR_MZ
# Append --rt to enable RT-supported admission.
```

## 5. How to evaluate the combinations fairly

The useful question is not simply "did the enhanced mode produce more peaks?"
It is **which change helped, under what budget, and at what cost?**

### Step 1: freeze the evaluation inputs

Before looking at gains, fix:

- Acquisitions and anchors, including the reference mass for every query.
- Calibration, native detection threshold, aperture, and context geometry.
- Peak cap and selection policy.
- Boundary/missingness policy and treatment of empty observations.
- Any separate MS1 assembly, normalization, model, decoding, and references.
- Primary endpoint, matching tolerances, and decision criteria.

For paired comparisons, reuse explicit anchors/masses rather than accepting
the first N nonempty results from each mode. Keep empty and unavailable cases
in the accounting. Prefer `KeepMasked` while establishing coverage; requiring
complete context can select a different population.

An ordinary-reader export and a feature-reader export are not automatically
paired controls: their sampling, geometry, and output filtering can differ.
Use the feature reader's native-only output as the local admission control.

### Step 2: verify measurement correctness before scoring usefulness

For unchanged source identities, require **zero differences** in measured m/z,
per-peak mobility, and integer response. Verify:

- Source IDs, acquisition rows, domains, and missing/empty distinctions survive
  export and reload.
- Native-preserving selection retains the native top-N set in each comparable
  available view, for the same policy, anchor, aperture, and cap.
- Consensus coordinates and selection scores never overwrite measurements.
- Raw gaps, incompatible CE trajectories, or duplicate IM views do not become
  extra independent RT support frames.

RT-enabled extraction also applies support-related acquisition/cadence barriers.
Its context availability can therefore differ from native-only extraction.
Report that difference separately. Check peak admission on common available
views, but do not silently discard the other cases from end-to-end results.

### Step 3: change one factor at a time

| Question | Change | Hold fixed | What to measure |
|---|---|---|---|
| Does acquisition handling help? | Single interval vs shared transmission vs trajectory. | Anchors/masses, geometry, cap; admission off initially. | Unique-source coverage, masks, gains and losses. |
| Does RT context help? | Focal-only vs focal plus RT neighbors. | The same native extraction and focal view. | Application endpoint and context cost. |
| Does IM context help? | Add IM slots, not aperture padding. | RT settings, aperture, sampling, acquisition, cap. | Application endpoint; repeated-source overlap. |
| Does RT admission help? | Admission off vs on. | Anchors/masses, policy, geometry, cap. | Retained weak sources, native retention, availability, application endpoint. |
| Does ranking help at a fixed budget? | Native-preserving vs alternative selection. | Eligible query population, qualification, cap. | Native losses, weak gains, downstream gains and losses. |
| Does precursor-first sampling help? | Detected-feature vs window-grid population. | Processing settings where applicable. | Coverage/accuracy by population, not row-by-row peak-count differences. |

Sampling is a **population experiment** as well as a processing choice.
Separate common anchors from precursor-only and window-only coverage. If an
aligned comparison uses one common reference mass, say so: it no longer
measures the effect of using each path's original reference mass.

### Step 4: use a small, interpretable application ablation

Choose one acquisition policy and cap, then compare:

| Arm | Application input | What the comparison isolates |
|---|---|---|
| A | Native focal only. | Baseline. |
| B | That same native focal plus native context. | B vs A: context usefulness. |
| C | RT-admission focal plus context, native-preserving. | C vs B: admission-stage effect, with availability reported separately. |

With the CLI recipes above, recipe 1 supplies both A and B; recipe 2 supplies C.
Recipe 3 changes the sampling population and is a separate comparison, not
another arm of this ablation.

For a trajectory assessment, use trajectory-native extraction for both A and
B. Comparing single-interval focal input with trajectory-augmented input
would mix acquisition coverage and context usefulness.

Optionally add RT-admission **focal only** to distinguish benefits to the focal
view from benefits to the supplied context. Add IM context as a subsequent
factor, rather than changing both RT and IM geometry in the first comparison.

### Step 5: assess the peak budget explicitly

Run both an uncapped companion (`peak_cap=0`) and the intended deployment cap
(for example, 300). The uncapped output shows qualified sources that a cap may
remove; a capped export alone does not expose every rejected candidate.

Keep two budget questions separate:

1. **Fixed per-view cap:** realistic when every observation has its own budget.
   Total records can increase when context is added.
2. **Fixed total consumer budget:** needed when the application has a global
   token/peak/memory limit. Specify how records are allocated across views.

The exporter implements the first, not an automatic pooled budget. If a
consumer truncates a concatenation to 300 records, that is another selection
policy and must be documented and evaluated separately.

Count gains **and** losses. Extra peaks are not automatically better, and a
full native-preserving cap can legitimately leave no weak-peak headroom.
There is no universally best top-300 policy.

### Step 6: report engineering and scientific results separately

| Report layer | Useful measurements |
|---|---|
| Coverage | Requested/available/empty/partial slots; anchor counts and reasons for missingness. |
| Measurement retention | Native sources retained/lost; qualified weak sources retained; capped vs uncapped differences. |
| Redundancy | Peak records versus unique `(frame, scan, tof)` sources; overlapping views. |
| Cost | Runtime, memory, storage, and total consumer records, using the same measurement setup. |
| Application accuracy | A prespecified independent endpoint, with paired gains/losses and acquisition-level replication. |

Separate extraction/export cost from application inference cost. Reusing one
native extraction for A and B establishes pairing; it does not measure the
cost of a dedicated focal-only production pipeline.

For sequence/fragment assessment, do not treat repeated observations, charge
states, or complementary fragment sides as independent cleavage constraints.
Define the counting unit before scoring.

Use independent reference material and acquisition-disjoint panels where
possible. References chosen because the focal already contains a fragment
have limited headroom and cannot establish independent sequence discovery.
Freeze RT qualification and reference-matching tolerances separately: one is
a processing parameter, the other an evaluation parameter.
Shifted-mass matches are incidental-match
controls, not chemical false labels or an automatic false-discovery estimate.

**Decision rule:** first pass measurement/retention checks, then demonstrate
benefit on the prespecified application endpoint without hiding coverage or
budget costs. Set the required improvement before evaluating; this guide
does not supply a universal accuracy threshold or claim an improvement.

## 6. Understanding the output

### What one neighborhood contains

| Item | Meaning |
|---|---|
| Focal view | The requested MS2 observation. |
| Context slots | Ordered by RT offset, then IM offset, excluding `(0, 0)`. |
| Peaks | Separate native source measurements, with optional aligned RT evidence. |
| Acquisition metadata | Original scan ranges, isolation bounds, CE, and group information. |
| Observation domain | Requested scan-wise transmission/missingness for the query mass. |
| Measured precursor metadata | Retained for the measured-feature path; distinct from a query mass. |

Python returns ordinary mappings/lists. Each context slot has `rt_offset`,
`im_offset`, and `available`; an available slot has a `view`, while an
unavailable slot has `view=None` and a reason.

**Missing is not empty.** An unavailable neighbor was not supplied under the
acquisition/boundary rules. An available view with `peaks=[]` was acquired but
has no retained peaks. `fully_covered` describes the view's IM coverage, not
whether all RT neighbors exist.

### Measurements, coordinates, and units

A peak's source identity is `(frame, scan, tof)`. RT support and context do not
replace its measured coordinates or response.

| Quantity | Contract |
|---|---|
| Frame | Actual source ID, not an index into a returned frame list; IDs may be sparse. |
| Interval and scan | Zero-based; scan and isolation-mass bounds are half-open. |
| TOF | Raw time-of-flight bin index; calibration converts it to m/z. |
| RT | Seconds; RT offsets count same-group acquisition steps, not frame-ID differences. |
| IM | Calibrated inverse reduced mobility; IM offsets follow scan direction, not necessarily increasing physical mobility. |
| Response | Native-kernel-weighted integer apex: a filtered peak-height measure, not integrated volume or an RT sum. |
| Consensus/score | Floating-point annotations, not replacement measurements or ownership probabilities. |

In Rust, `peak_apex_responses` is the authoritative integer array; spectrum
intensities are its floating-point representation. Python uses integer
`apex_response` values. RT support responses are exact integers too. Preserve
these fields when values above `2**53` matter.

### Acquisition truth versus a convenient spectrum header

The scalar spectrum isolation interval summarizes the **retained source
footprints**, seeded by the anchor interval. It is not the full physical
acquisition trajectory. Rejected weak candidates do not narrow it; adding a
retained source legitimately can.

Use original acquisition rows and `observation_domain` for scan-wise coverage,
transmission masks, and CE. Untransmitted or missing scan cells are unobserved,
not measured zeros.

Do not derive an otherwise fixed MS1 input window from a peak-dependent
isolation summary. Keep application-specific MS1 assembly separate and
consistent across comparison arms.

### On-disk format

| File | Contents |
|---|---|
| `manifest.json` | Completion marker, schema/units, response definition, requested/resolved configuration, counts, optional prefix limit. |
| `observations.parquet` | Slots, availability, geometry, and peak ranges. |
| `peaks.parquet` | Measured coordinates, source IDs, integer responses, and optional RT evidence. |
| `acquisition.parquet` | Original physical acquisition rows. |

The acquisition table can include metadata beyond an exported prefix.
Only finalized datasets are readable as complete. A failed write may leave an
incomplete directory; readers reject it rather than treating it as empty input.
Existing output directories are not silently overwritten.

Rust storage uses `timsrust_parquet_spectra::neighborhoods::{Writer, Reader}`.
Python uses `ParquetNeighborhoodReader`: `len`, `get`/indexing, `manifest()`,
`resolved_config()`, and `acquisition()`. It reads individual bundle peak
ranges rather than eagerly loading every peak. No Python Parquet dependency
is required.

## 7. Configuration and API reference

### Defaults

FWHM means full width at half maximum. Here, the learned scan-direction kernel
width sets the default IM aperture; it does not set the RT extent.

| Setting | Default | Important detail |
|---|---|---|
| Sampling | CLI: window-first. | Explicit Rust/Python queries are separate from automatic sampling. |
| Acquisition | Single interval. | Other policies are opt-in. |
| RT context | Two steps before and after. | Five slots when no IM context is requested. |
| IM context | None. | Adding neighbors is separate from changing aperture. |
| IM aperture | Half the learned native MS1 scan FWHM on each side, zero padding. | Inspect resolved scan widths, not an assumed constant. |
| Peak cap | Rust/Python: 300; CLI: **0**. | Set explicitly for comparisons. |
| Native MS2 detection threshold | 2. | Feature configuration requires an integer count of at least one. |
| RT admission | Off. | Enabling it defaults to native-preserving selection. |
| RT qualification | 10 ppm, at least 3 frames, both sides. | Subject to available context and acquisition eligibility. |
| Boundary policy | `KeepMasked`. | Preserve missing/partial context explicitly. |

Extents are limited to 32 steps per side and the complete RT/IM grid to 1,024
slots. `RequireComplete` skips incomplete neighborhoods in automatic iteration
and rejects incomplete explicit queries. Changing it can change the population.

### Names across interfaces

| Concept | Rust | Python | CLI |
|---|---|---|---|
| Single interval | `AcquisitionPolicy::SingleInterval` | `"single_interval"` | `--acquisition single-interval` |
| Shared transmission | `AcquisitionPolicy::SharedTransmission` | `"shared_transmission"` | `--acquisition shared-transmission` |
| Trajectory | `AcquisitionPolicy::ExperimentalTrajectory` | `"experimental_trajectory"` | `--acquisition experimental-trajectory` |
| RT extent | `rt_context: ContextExtent` | `rt_context=(before, after)` | `--rt-before`, `--rt-after` |
| IM extent | `im_context: ContextExtent` | `im_context=(before, after)` | `--im-neighbors` (symmetric) |
| RT admission | `rt_consistency: Some(...)` | `rt_consistency=RtConsistencyConfig(...)` | `--rt-centroiding=true` |
| Native-only | `rt_consistency: None` | `rt_consistency=None` | `--rt-centroiding=false` |
| Cap | `peak_cap` | `peak_cap` | `--peak-cap` |

Rust `ImSizing::NativeFwhm` and Python `im_padding=(left, right)` add aperture
padding in native FWHM units. Rust `ImSizing::Scans` and Python
`im_scans=(left, right, stride)` specify scan radii and sampling stride.
Padding and explicit scan sizing are alternatives, not additive controls.
The IM-context center step is `max(left, right) + 1`; it is not the window-grid
sampling stride.

### Entry points

| Need | Rust | Python |
|---|---|---|
| Explicit interval-centered query | `neighborhood_at` | `read_at` without a mass |
| Explicit mass-conditioned query | `neighborhood_for_precursor` | `read_at(..., precursor_mz=...)` |
| Multiple explicit queries | `neighborhoods_at`, `neighborhoods_with_precursors` | `read_many` |
| Automatic window sampling | `neighborhoods` | Not exposed; use Rust/CLI. |
| Automatic measured-feature sampling | `precursor_neighborhoods` | Not exposed; use Rust/CLI. |
| Focal-only projection | `pseudospectra` or take a bundle's focal view | Take `bundle["focal"]`. |
| Geometry/acquisition inspection | `resolved_config`, `frame_infos`, `observation_domain` | `resolved_config()`, `frame_infos()`, per-view metadata |

See the [Python guide](../python/timsrust-pyo3/README.md) and shipped type
declarations for full signatures, return shapes, exceptions, and GIL behavior.
Invalid inputs, missing calibration, acquisition errors, and I/O failures are
reported explicitly, not silently replaced by another processing mode.

## 8. Validation, limitations, and troubleshooting

### Reproducing engineering checks

Portable tests use synthetic acquisitions/raw snippets and require no private
data or benchmark labels. They cover native values and integer ranking,
capped/uncapped membership, sparse IDs, missing/empty/partial views,
transmission/CE/cadence barriers, raw-support checks, isolation summaries,
storage round trips, and Python error propagation.

From the repository root:

```sh
cargo test -p timsrust-centroid -p timsrust-parquet-spectra \
  -p timsrust-centroid-cli -p timsrust --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --exclude timsrust-sdk -- -D warnings
```

For native extension builds, Rust-generated Python fixtures, and Python tests,
follow the [Python development instructions](../python/timsrust-pyo3/README.md#development).
Optional raw-data tests use `TIMSRUST_TEST_DIA` for the CLI and
`TIMSRUST_NATIVE_TEST_PATH` for Python. A skipped raw test is not a successful
live-data test. Engineering checks do not establish application accuracy.

### Common surprises

| Observation | Explanation or next check |
|---|---|
| RT mode adds no peaks. | Check cap headroom, distinct-frame support, boundaries, ambiguity, and acquisition eligibility. No additions can be correct. |
| Augmentation exceeds 300 records. | The cap is per view, not per neighborhood. |
| The isolation header changed. | Check retained source footprints; compare physical acquisition using raw rows/domains instead. |
| Precursor-first and window-first export different counts. | They sample different anchor populations and use different reference masses. |
| An IM neighbor repeats a source. | Views can overlap; count source identities rather than treating every record as independent. |
| A context slot disappears with RT admission. | Support-related acquisition/cadence barriers can change availability; report it explicitly. |
| A reloaded dataset is rejected. | Check completion, schema version, row references, and counts; do not reinterpret a failed read as empty data. |

### Limits to keep in mind

- The raw reader is for calibrated DIA TDF, not every supported file format.
- Run-global calibration can still contain physical mass or mobility drift.
- Acquisition compatibility does not establish common chemical origin.
- Weak support may be ambiguous or absent, particularly near boundaries.
- Fixed budgets impose trade-offs; no policy promises all useful evidence.
- MS1 input assembly and application-specific inference remain separate.
- This branch makes no general sequence-accuracy, purity, denoising, or
  false-discovery improvement claim.
