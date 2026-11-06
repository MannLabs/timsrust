"""Tests for the compiled native observation API; no raw acquisition needed."""

import os
from pathlib import Path
import shutil
import uuid

import pytest

from tims_pyo3 import (
    NativeNeighborhoodConfig,
    NativeNeighborhoodReader,
    ParquetNeighborhoodReader,
    RtConsistencyConfig,
)


def test_default_config_and_native_preserving_rt():
    config = NativeNeighborhoodConfig().to_dict()
    assert config["peak_cap"] == 300
    assert config["rt_consistency"] is None
    rt = RtConsistencyConfig()
    assert rt.to_dict()["selection"] == "native_preserving"
    for selection in ["support_weighted", "focal_response"]:
        assert RtConsistencyConfig(selection=selection).to_dict()["selection"] == selection
    configured = NativeNeighborhoodConfig(
        acquisition="experimental_trajectory",
        im_scans=(2, 3, 1),
        rt_consistency=rt,
    ).to_dict()
    assert configured["acquisition"] == "experimental_trajectory"
    assert configured["im"] == {
        "kind": "scans", "left": 2, "right": 3, "stride": 1,
    }
    configured["peak_cap"] = 1
    assert NativeNeighborhoodConfig().to_dict()["peak_cap"] == 300


@pytest.mark.parametrize("kwargs", [
    {"acquisition": "unknown"},
    {"rt_context": (33, 0)},
    {"im_context": (32, 32), "rt_context": (32, 32)},
    {"im_scans": (1, 1, 0)},
    {"im_scans": (1, 1, 1), "im_padding": (0.0, 0.0)},
    {"im_padding": (float("nan"), 0.0)},
    {"im_padding": (-1.0, 0.0)},
    {"min_ms2_ion_count": 0},
    {"rt_context": (0, 0), "rt_consistency": RtConsistencyConfig()},
])
def test_invalid_config(kwargs):
    with pytest.raises(ValueError):
        NativeNeighborhoodConfig(**kwargs)


@pytest.mark.parametrize("kwargs", [
    {"mz_ppm": 0.0},
    {"mz_ppm": float("inf")},
    {"minimum_support": 1},
    {"minimum_support": 66},
    {"selection": "unknown"},
])
def test_invalid_rt_config(kwargs):
    with pytest.raises(ValueError):
        RtConsistencyConfig(**kwargs)


def test_keyword_only_and_integer_thresholds():
    with pytest.raises(TypeError):
        NativeNeighborhoodConfig("single_interval")
    with pytest.raises(TypeError):
        NativeNeighborhoodConfig(min_ms2_ion_count=2.5)
    with pytest.raises(OverflowError):
        NativeNeighborhoodConfig(peak_cap=-1)
    with pytest.raises(TypeError):
        NativeNeighborhoodConfig(unknown=True)


def test_missing_paths_raise_oserror():
    with pytest.raises(OSError):
        NativeNeighborhoodReader("missing-native-acquisition.d")
    with pytest.raises(OSError):
        ParquetNeighborhoodReader("missing-neighborhood-dataset")


@pytest.fixture
def saved():
    path = os.environ.get("TIMSRUST_PYTHON_TEST_DATASET")
    if path is None:
        pytest.skip("set TIMSRUST_PYTHON_TEST_DATASET to the Rust synthetic fixture")
    return ParquetNeighborhoodReader(path)


def test_parquet_structured_measurements(saved):
    assert len(saved) == 1
    bundle = saved.get(0)
    assert saved[0] == bundle
    focal = bundle["focal"]
    assert (focal["frame"], focal["interval"], focal["scan"]) == (12, 0, 2)
    peak = focal["peaks"][0]
    assert type(peak["apex_response"]) is int
    assert peak["apex_response"] == 2**53 + 17
    assert (peak["scan"], peak["tof"]) == (3, 44)
    assert peak["mz"] == 300.25
    assert peak["inverse_mobility"] == 1.19
    assert peak["rt_evidence"]["consensus_mz"] == 300.251
    assert peak["rt_evidence"]["selection_score"] == 9.0
    assert peak["rt_evidence"]["support"][0]["frame"] == 12
    response = peak["rt_evidence"]["support"][0]["response"]
    assert type(response) is int
    assert response == 2**53 + 17
    assert focal["rt_evidence_present"]
    assert bundle["measured_precursor"] is None
    acquired, missing = bundle["context"]
    assert acquired["available"] is True
    assert acquired["view"]["peaks"] == []
    assert acquired["view"]["rt_evidence_present"]
    assert missing["available"] is False
    assert missing["view"] is None
    assert missing["reason"] == "RunBoundary"


def test_acquisition_is_distinct_from_scalar_summary(saved):
    focal = saved[0]["focal"]
    summary = focal["isolation_summary"]
    assert summary["meaning"] == "conservative_selected_source_support_intersection"
    assert summary["isolation_lower"] == 450.0
    assert focal["acquisition"][0]["isolation_lower"] == 400.0
    domain = focal["observation_domain"]
    assert (domain["scan_start"], domain["scan_end"]) == (-1, 7)
    assert [s["transmitted"] for s in domain["segments"]] == [
        False, True, True, False,
    ]
    assert [s["collision_energy"] for s in domain["segments"]] == [
        None, 30.0, 32.0, None,
    ]
    assert len(saved.acquisition()) == 4
    assert saved.manifest()["complete"]
    assert saved.resolved_config()["native_scan_fwhm"] == 4
    focal["peaks"].clear()
    assert len(saved[0]["focal"]["peaks"]) == 1


def test_parquet_index_errors(saved):
    with pytest.raises(IndexError):
        saved.get(len(saved))
    with pytest.raises(OverflowError):
        saved.get(-1)


def test_optional_native_extraction_and_export():
    path = os.environ.get("TIMSRUST_NATIVE_TEST_PATH")
    if path is None:
        pytest.skip("set TIMSRUST_NATIVE_TEST_PATH for raw extraction/export")
    reader = NativeNeighborhoodReader(
        path, NativeNeighborhoodConfig(
            rt_context=(1, 1), im_scans=(2, 2, 1), peak_cap=10,
        ),
    )
    frame = next(f for f in reader.frame_infos() if f["acquisition"])
    row = frame["acquisition"][0]
    anchor = (
        frame["frame"], row["interval"],
        (row["scan_start"] + row["scan_end"] - 1) // 2,
    )
    mz = (row["isolation_lower"] + row["isolation_upper"]) / 2
    result = reader.read_at(*anchor, precursor_mz=mz)
    assert result == reader.read_many([anchor], precursor_mzs=[mz])[0]
    assert reader.resolved_config()["request"]["peak_cap"] == 10
    for bad_mz in [float("nan"), float("inf"), 0.0, -1.0]:
        with pytest.raises(ValueError):
            reader.read_at(*anchor, precursor_mz=bad_mz)
    for anchors, masses in [([], None), ([anchor] * 1025, None), ([anchor], [])]:
        with pytest.raises(ValueError):
            reader.read_many(anchors, precursor_mzs=masses)
    destination = (
        Path(__file__).parent.parent / "target-python-bindings"
        / f"native-export-{uuid.uuid4().hex}"
    )
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        manifest = reader.export(
            str(destination), [anchor], precursor_mzs=[mz],
        )
        assert manifest["complete"] is True
        assert manifest["bundles"] == 1
        assert ParquetNeighborhoodReader(str(destination))[0] == result
        with pytest.raises(OSError):
            reader.export(str(destination), [anchor])
    finally:
        if destination.exists():
            shutil.rmtree(destination)
