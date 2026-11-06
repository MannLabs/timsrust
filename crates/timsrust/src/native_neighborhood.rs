use timsrust_centroid::{
    PeakReader,
    neighborhood::{
        Anchor, CentroidSource, Config, Neighborhood, NeighborhoodIter,
        NeighborhoodReader, ObservationDomain, ResolvedConfig, View,
    },
};
use timsrust_core::{AcquisitionType, Mz, Spectrum};
use timsrust_tdf::{FrameInfoReader, TdfFrameReader, TdfIonReader};

use crate::{ImConverter, MzConverter, TimsRustError, TimsTofPath};

type Source =
    CentroidSource<TdfIonReader, FrameInfoReader, ImConverter, MzConverter>;

/// Opt-in acquisition-window sampling; legacy spectrum readers are unchanged.
pub struct NativeNeighborhoodReader {
    inner: NeighborhoodReader<Source>,
    path: String,
}

impl NativeNeighborhoodReader {
    pub fn new(
        path: impl AsRef<str>,
        config: Config,
    ) -> Result<Self, TimsRustError> {
        config.validate()?;
        let path = TimsTofPath::new(path)?;
        let im = ImConverter::new(path.as_ref()).ok_or_else(|| {
            timsrust_centroid::TimsError::new(
                "missing native mobility calibration",
            )
        })?;
        let mz = MzConverter::new(path.as_ref()).ok_or_else(|| {
            timsrust_centroid::TimsError::new("missing native mass calibration")
        })?;
        if matches!(im, ImConverter::Bit(_) | ImConverter::MiniTdf)
            || matches!(mz, MzConverter::Bit(_) | MzConverter::MiniTdf)
        {
            return Err(timsrust_centroid::TimsError::new(
                "native neighborhoods require raw TDF calibration",
            )
            .into());
        }
        let frames = TdfFrameReader::new(path.as_ref())?;
        if frames.get_acquisition() != AcquisitionType::DIAPASEF {
            return Err(timsrust_centroid::TimsError::new(
                "native neighborhoods require diaPASEF acquisition",
            )
            .into());
        }
        let peaks = PeakReader::new(
            frames.into_inner(),
            0.5,
            config.min_ms2_ion_count,
        )?;
        Ok(Self {
            path: path.as_ref().to_string(),
            inner: NeighborhoodReader::new(
                CentroidSource::new(peaks, im, mz),
                config,
            )?,
        })
    }

    pub fn resolved_config(&self) -> &ResolvedConfig {
        self.inner.resolved_config()
    }

    pub fn frame_infos(&self) -> &[timsrust_core::FrameInfo] {
        self.inner.frame_infos()
    }

    pub fn observation_domain(
        &self,
        view: &View,
    ) -> Result<ObservationDomain, TimsRustError> {
        Ok(self.inner.observation_domain(view)?)
    }

    pub fn neighborhoods(&self) -> NeighborhoodIter<'_, Source> {
        self.inner.neighborhoods()
    }

    pub fn neighborhood_at(
        &self,
        anchor: Anchor,
    ) -> Result<Neighborhood, TimsRustError> {
        Ok(self.inner.neighborhood_at(anchor)?)
    }

    pub fn neighborhoods_at<'a>(
        &'a self,
        anchors: impl Iterator<Item = Anchor> + 'a,
    ) -> impl Iterator<Item = timsrust_centroid::TimsResult<Neighborhood>> + 'a
    {
        self.inner.neighborhoods_at(anchors)
    }

    pub fn neighborhood_for_precursor(
        &self,
        anchor: Anchor,
        mz: f64,
    ) -> Result<Neighborhood, TimsRustError> {
        Ok(self.inner.neighborhood_for_precursor(anchor, mz)?)
    }

    /// Explicit MS1 features with a shared bounded frame cache.
    pub fn neighborhoods_with_precursors<'a>(
        &'a self,
        anchors: impl Iterator<
            Item = timsrust_centroid::TimsResult<(
                Anchor,
                timsrust_core::Precursor,
            )>,
        > + 'a,
    ) -> impl Iterator<
        Item = timsrust_centroid::TimsResult<(
            Neighborhood,
            timsrust_core::Precursor,
        )>,
    > + 'a {
        self.inner.neighborhoods_with_precursors(anchors)
    }

    pub fn pseudospectra(
        &self,
    ) -> impl Iterator<Item = timsrust_centroid::TimsResult<Spectrum<Mz>>> + '_
    {
        self.inner.pseudospectra()
    }

    /// Streams MS1-feature-conditioned bundles with measured precursor metadata.
    ///
    /// Anchors include transmitted MS1 candidates even when the focal MS2 view
    /// is empty. RT qualification therefore cannot change the anchor population.
    pub fn precursor_neighborhoods(
        &self,
        min_ms1_ion_count: f64,
    ) -> Result<
        impl Iterator<
            Item = timsrust_centroid::TimsResult<(
                Neighborhood,
                timsrust_core::Precursor,
            )>,
        > + '_,
        TimsRustError,
    > {
        use std::{collections::VecDeque, sync::Arc};
        use timsrust_centroid::spectrum_reader::NarrowSpectrumReader;
        use timsrust_core::MSLevel;
        if !min_ms1_ion_count.is_finite() || min_ms1_ion_count <= 0.0 {
            return Err(timsrust_centroid::TimsError::new(
                "MS1 precursor threshold must be finite and positive",
            )
            .into());
        }
        let frames = TdfFrameReader::new(&self.path)?;
        let mut infos = frames
            .iter_indices()
            .map(|id| frames.get_info(id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| timsrust_centroid::TimsError::new(e.to_string()))?;
        infos.sort_by(|a, b| {
            a.rt_in_seconds()
                .total_cmp(&b.rt_in_seconds())
                .then(a.index().cmp(&b.index()))
        });
        let narrow = NarrowSpectrumReader::new(
            PeakReader::new(
                frames.into_inner(),
                min_ms1_ion_count,
                self.resolved_config().request.min_ms2_ion_count,
            )?,
            0,
            Arc::new(ImConverter::new(&self.path).ok_or_else(|| {
                timsrust_centroid::TimsError::new(
                    "missing mobility calibration",
                )
            })?),
            Arc::new(MzConverter::new(&self.path).ok_or_else(|| {
                timsrust_centroid::TimsError::new("missing mass calibration")
            })?),
        )?;
        let (mut position, mut pending, mut failed) =
            (0, VecDeque::new(), false);
        let anchors = std::iter::from_fn(move || {
            if failed {
                return None;
            }
            loop {
                if let Some(item) = pending.pop_front() {
                    return Some(Ok(item));
                }
                while position < infos.len()
                    && infos[position].ms_level() != MSLevel::MS1
                {
                    position += 1;
                }
                if position == infos.len() {
                    return None;
                }
                let ms1 = position;
                position += 1;
                while position < infos.len()
                    && infos[position].ms_level() != MSLevel::MS1
                {
                    position += 1;
                }
                let precursors = match narrow
                    .try_precursors_from_frame(infos[ms1].index())
                {
                    Ok(values) => values,
                    Err(error) => {
                        failed = true;
                        return Some(Err(error));
                    },
                };
                for info in &infos[ms1 + 1..position] {
                    if info.ms_level() != MSLevel::MS2 {
                        continue;
                    }
                    let q = info.quadrupole_settings();
                    for precursor in &precursors {
                        let scan = usize::from(precursor.scan_index());
                        if let Some(interval) =
                            q.get_isolation_window_index(scan)
                        {
                            let mz = f64::from(precursor.mz());
                            let window = &q.isolation_windows[interval];
                            if f64::from(window.lower()) <= mz
                                && mz < f64::from(window.upper())
                            {
                                pending.push_back((
                                    Anchor {
                                        frame: info.index(),
                                        interval,
                                        scan,
                                    },
                                    precursor.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        });
        Ok(self.inner.neighborhoods_with_precursors(anchors))
    }
}
