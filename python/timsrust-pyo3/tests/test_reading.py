from dataclasses import dataclass

import pytest
import timsrust_pyo3
from timsrust_pyo3 import MSLevel


def test_read_all_frames(shared_datadir):
    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    all_frames = timsrust_pyo3.read_all_frames(file)
    assert all(isinstance(f, timsrust_pyo3.Frame) for f in all_frames)


def test_file_reader(shared_datadir):
    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    reader = timsrust_pyo3.FrameReader(file)
    all_frames = reader.read_all_frames()
    all_frames2 = timsrust_pyo3.read_all_frames(file)

    assert len(all_frames) == len(all_frames2)
    assert all(isinstance(f, timsrust_pyo3.Frame) for f in all_frames)

    dia_frames = reader.read_dia_frames()
    assert all(f.ms_level == MSLevel.MS2 for f in dia_frames)
    assert all(
        f.acquisition_type == timsrust_pyo3.AcquisitionType.DIAPASEF for f in dia_frames
    )
    assert all(isinstance(f, timsrust_pyo3.Frame) for f in dia_frames)

    ms1_frames = reader.read_ms1_frames()
    assert all(f.ms_level == MSLevel.MS1 for f in ms1_frames)
    assert all(isinstance(f, timsrust_pyo3.Frame) for f in ms1_frames)


EXPECTATIONS = {
    "test.ms2": {
        "n_spectra": 3,
        "first_mzs": [190.10706],
        "first_intensities": [350.0],
        "first_precursor": {
            "mz": 500.0,
            "rt": 0.1,
            "im": 1.3,
            "charge": 2,
            "intensity": 0,
        },
    },
    "test2.ms2": {
        "n_spectra": 2,
        "first_mzs": [100.0, 200.002, 300.03, 400.4],
        "first_intensities": [1.0, 2.0, 3.0, 4.0],
        "first_precursor": {
            "mz": 123.4567,
            "rt": 12.345,
            "im": 1.234,
            "charge": 1,
            "intensity": 0,
        },
    },
    "dda_test.d": {
        "n_spectra": 3,
        "first_mzs": [199.7633445943076],
        "first_intensities": [162],
        "first_precursor": {
            "mz": 500.0,
            "rt": 0.2,
            "im": 1.25,
            "charge": 2,
            "intensity": 10,
        },
    },
}


@pytest.mark.parametrize("file", ["test.ms2", "test2.ms2", "dda_test.d"])
def test_minitdf_file_reading(shared_datadir, file):
    datafile = str(shared_datadir / file)
    specs = timsrust_pyo3.read_all_spectra(datafile)

    assert len(specs) == EXPECTATIONS[file]["n_spectra"]
    assert specs[0].mz_values == EXPECTATIONS[file]["first_mzs"]
    assert specs[0].intensities == EXPECTATIONS[file]["first_intensities"]
    assert specs[0].precursor.mz == EXPECTATIONS[file]["first_precursor"]["mz"]
    assert specs[0].precursor.rt == EXPECTATIONS[file]["first_precursor"]["rt"]
    assert specs[0].precursor.im == EXPECTATIONS[file]["first_precursor"]["im"]
    assert specs[0].precursor.charge == EXPECTATIONS[file]["first_precursor"]["charge"]
    assert (
        specs[0].precursor.intensity
        == EXPECTATIONS[file]["first_precursor"]["intensity"]
    )


def test_mz_resolution(shared_datadir):
    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    file2 = str(
        shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d/analysis.tdf"
    )

    reader = timsrust_pyo3.FrameReader(file)
    metadata = timsrust_pyo3.Metadata(file2)
    allframes = reader.read_all_frames()
    resolved = metadata.resolve_mzs(allframes[0].tof_indices)
    assert len(resolved) == 242412
    assert all(isinstance(mz, float) for mz in resolved)


def test_frame_converters(shared_datadir):
    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    file2 = str(
        shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d/analysis.tdf"
    )
    reader = timsrust_pyo3.FrameReader(file)
    metadata = timsrust_pyo3.Metadata(file2)
    allframes = reader.read_all_frames()

    resolved_mzs = metadata.resolve_mzs(allframes[0].tof_indices)

    assert len(resolved_mzs) == 242412
    assert all(isinstance(mz, float) for mz in resolved_mzs)

    resolved_scans = metadata.resolve_scans(
        list(range(1, len(allframes[0].scan_offsets) + 1))
    )
    assert len(resolved_scans) == 710
    assert resolved_scans[0] >= 1.36
    assert resolved_scans[0] <= 1.37

    # These changed in timsrust 0.4.0 ... make sure they are correct
    # assert resolved_scans[-1] >= 0.81
    # assert resolved_scans[-1] <= 0.82
    assert resolved_scans[-1] >= 0.6389
    assert resolved_scans[-1] <= 0.6390


def test_flattening(shared_datadir):
    @dataclass
    class DenseFrame:
        rt: float
        intensities: list[int]
        mzs: list[float]
        imss: list[float]

        @classmethod
        def from_frame(
            cls, frame: timsrust_pyo3.Frame, metadata: timsrust_pyo3.Metadata
        ):
            mzs = metadata.resolve_mzs(frame.tof_indices)
            out_imss = [None] * len(mzs)
            last_so = 0
            for ims, so in zip(
                metadata.resolve_scans(list(range(1, len(frame.scan_offsets) + 1))),
                frame.scan_offsets,
                strict=True,
            ):
                out_imss[last_so:so] = [ims] * (so - last_so)
                last_so = so

            return cls(
                rt=frame.rt,
                intensities=frame.intensities,
                mzs=mzs,
                imss=out_imss,
            )

    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    reader = timsrust_pyo3.FrameReader(file)
    metadata = timsrust_pyo3.Metadata(file + "/analysis.tdf")
    allframes = reader.read_all_frames()

    _ = DenseFrame.from_frame(allframes[0], metadata=metadata)
    dense_frames = [DenseFrame.from_frame(f, metadata=metadata) for f in allframes]

    for f in dense_frames:
        assert len(f.intensities) == len(f.mzs)
        assert len(f.intensities) == len(f.imss)
        assert all(
            isinstance(i, int) for i in f.intensities
        ), f"Not all intensities are ints ({set([type(i) for i in f.intensities])})"
        assert all(isinstance(m, float) for m in f.mzs), "Not all mzs are floats"
        assert all(isinstance(i, float) for i in f.imss), "Not all imss are floats"
        assert all(i >= 0 for i in f.intensities)
