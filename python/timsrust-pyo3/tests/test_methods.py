import timsrust_pyo3


def test_dunder_iter_frame(shared_datadir):
    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    reader = timsrust_pyo3.FrameReader(file)

    accum = []
    # Here I am testing that __iter__ is working
    for x in reader:
        accum.append(x)

    all_frames = timsrust_pyo3.read_all_frames(file)
    assert len(all_frames) == len(accum)

    for a, b in zip(all_frames, accum):
        assert a.ms_level == b.ms_level


def test_dunder_iter_spectrum(shared_datadir):
    file = str(shared_datadir / "230711_idleflow_400-1000mz_25mz_diaPasef_10sec.d")
    reader = timsrust_pyo3.SpectrumReader(file)
    all_spectra = timsrust_pyo3.read_all_spectra(file)
    accum = []
    # Here I am testing that __iter__ is working
    for x in reader:
        accum.append(x)

    assert len(all_spectra) == len(accum)

    for a, b in zip(all_spectra, accum):
        # RN timsrust exports only ms2 ... so it has no ms level
        # assert a.ms_level == b.ms_level
        assert a.collision_energy == b.collision_energy
