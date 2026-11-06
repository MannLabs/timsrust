from __future__ import annotations

from typing import Any, Literal, Optional, TypedDict, Union

AcquisitionPolicy = Literal[
    "single_interval", "shared_transmission", "experimental_trajectory"
]
Anchor = tuple[int, int, int]

class AcquisitionRow(TypedDict):
    frame: int
    interval: int
    group: int
    rt_s: float
    scan_start: int
    scan_end: int
    isolation_lower: float
    isolation_upper: float
    collision_energy: float

class DomainSegment(TypedDict):
    scan_start: int
    scan_end: int
    transmitted: bool
    collision_energy: Optional[float]

class ObservationDomain(TypedDict):
    scan_start: int
    scan_end: int
    precursor_mz: float
    segments: list[DomainSegment]

class IsolationSummary(TypedDict):
    meaning: Literal["conservative_selected_source_support_intersection"]
    isolation_lower: float
    isolation_upper: float
    collision_energy: float

class PrecursorMetadata(TypedDict):
    mz: float
    rt_s: float
    inverse_mobility: float
    scan: int
    frame: int
    charge: Optional[int]
    intensity: Optional[float]
    index: int

class RtSupport(TypedDict):
    frame: int
    rt_s: float
    scan: int
    tof: int
    mz: float
    response: int

class RtEvidence(TypedDict):
    native: bool
    qualified: bool
    ambiguous: bool
    support: list[RtSupport]
    consensus_mz: float
    consensus_scan: float
    selection_score: float

class NativePeak(TypedDict):
    scan: int
    tof: int
    mz: float
    inverse_mobility: float
    apex_response: int
    rt_evidence: Optional[RtEvidence]

class ObservationView(TypedDict):
    frame: int
    interval: int
    scan: int
    rt_s: float
    inverse_mobility: float
    requested_scan_start: int
    requested_scan_end: int
    scan_start: int
    scan_end: int
    fully_covered: bool
    spectrum_id: int
    isolation_summary: IsolationSummary
    precursor: Optional[PrecursorMetadata]
    acquisition: list[AcquisitionRow]
    observation_domain: Optional[ObservationDomain]
    rt_evidence_present: bool
    peaks: list[NativePeak]

class AvailableObservation(TypedDict):
    rt_offset: int
    im_offset: int
    available: Literal[True]
    view: ObservationView

class UnavailableObservation(TypedDict):
    rt_offset: int
    im_offset: int
    available: Literal[False]
    view: None
    frame: Optional[int]
    reason: Literal[
        "RunBoundary", "AcquisitionChanged", "CenterOutsideIsolation", "TimeGap"
    ]

class Neighborhood(TypedDict):
    focal: ObservationView
    context: list[Union[AvailableObservation, UnavailableObservation]]
    measured_precursor: Optional[PrecursorMetadata]

class ContextExtent(TypedDict):
    before: int
    after: int

class RtConfig(TypedDict):
    mz_ppm: float
    minimum_support: int
    require_both_sides: bool
    selection: Literal["native_preserving", "support_weighted", "focal_response"]

class NativeFwhmIm(TypedDict):
    kind: Literal["native_fwhm"]
    padding_left: float
    padding_right: float

class ScanIm(TypedDict):
    kind: Literal["scans"]
    left: int
    right: int
    stride: int

class _Config(TypedDict):
    schema_version: int
    im: Union[NativeFwhmIm, ScanIm]
    rt_context: ContextExtent
    im_context: ContextExtent
    rt_consistency: Optional[RtConfig]
    peak_cap: int
    min_ms2_ion_count: float
    boundary: Literal["keep_masked", "require_complete"]

class Config(_Config, total=False):
    acquisition: AcquisitionPolicy

class ResolvedConfig(TypedDict):
    request: Config
    native_scan_fwhm: int
    im_left_scans: int
    im_right_scans: int
    anchor_stride_scans: int
    im_context_step_scans: int
    response_definition: Literal["native_kernel_weighted_apex"]

class FrameInfo(TypedDict):
    frame: int
    rt_s: float
    group: int
    cycle_index: Optional[int]
    acquisition: list[AcquisitionRow]

class Manifest(TypedDict):
    schema_version: int
    complete: bool
    sampling: Literal["window", "precursor", "explicit"]
    config: Config
    native_scan_fwhm: int
    response_definition: Literal["native_kernel_weighted_apex"]
    rt_unit: Literal["seconds"]
    mobility_unit: Literal["inverse_reduced_mobility"]
    bundles: int
    slots_per_bundle: int
    observations: int
    peaks: int
    acquisition_intervals: int
    bundle_limit: Optional[int]

class RtConsistencyConfig:
    def __init__(
        self, *, mz_ppm: float = 10.0, minimum_support: int = 3,
        require_both_sides: bool = True,
        selection: Literal[
            "native_preserving", "support_weighted", "focal_response"
        ] = "native_preserving",
    ) -> None: ...
    def to_dict(self) -> RtConfig: ...

class NativeNeighborhoodConfig:
    def __init__(
        self, *, acquisition: AcquisitionPolicy = "single_interval",
        rt_context: tuple[int, int] = (2, 2),
        im_context: tuple[int, int] = (0, 0),
        im_scans: Optional[tuple[int, int, int]] = None,
        im_padding: Optional[tuple[float, float]] = None,
        peak_cap: int = 300, min_ms2_ion_count: int = 2,
        require_complete: bool = False,
        rt_consistency: Optional[RtConsistencyConfig] = None,
    ) -> None: ...
    def to_dict(self) -> Config: ...

class NativeNeighborhoodReader:
    def __init__(
        self, path: str, config: Optional[NativeNeighborhoodConfig] = None,
    ) -> None: ...
    def read_at(
        self, frame: int, interval: int, scan: int,
        precursor_mz: Optional[float] = None,
    ) -> Neighborhood: ...
    def read_many(
        self, anchors: list[Anchor], *,
        precursor_mzs: Optional[list[float]] = None,
    ) -> list[Neighborhood]: ...
    def resolved_config(self) -> ResolvedConfig: ...
    def frame_infos(self) -> list[FrameInfo]: ...
    def export(
        self, path: str, anchors: list[Anchor], *,
        precursor_mzs: Optional[list[float]] = None,
    ) -> Manifest: ...

class ParquetNeighborhoodReader:
    def __init__(self, path: str) -> None: ...
    def __len__(self) -> int: ...
    def get(self, index: int) -> Neighborhood: ...
    def __getitem__(self, index: int) -> Neighborhood: ...
    def manifest(self) -> Manifest: ...
    def resolved_config(self) -> ResolvedConfig: ...
    def acquisition(self) -> list[AcquisitionRow]: ...

# The pre-existing frame/spectrum API is unchanged.
def __getattr__(name: str) -> Any: ...
