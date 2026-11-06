use serde::{Deserialize, Serialize};

use crate::{TimsError, TimsResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextExtent {
    pub before: usize,
    pub after: usize,
}

impl Default for ContextExtent {
    fn default() -> Self {
        Self {
            before: 2,
            after: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImSizing {
    NativeFwhm {
        #[serde(default)]
        padding_left: f64,
        #[serde(default)]
        padding_right: f64,
    },
    Scans {
        left: usize,
        right: usize,
        stride: usize,
    },
}

impl Default for ImSizing {
    fn default() -> Self {
        Self::NativeFwhm {
            padding_left: 0.0,
            padding_right: 0.0,
        }
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryPolicy {
    #[default]
    KeepMasked,
    RequireComplete,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionPolicy {
    #[default]
    SingleInterval,
    /// Contiguous rows transmitting one precursor at exactly the same energy.
    SharedTransmission,
    /// Experimental mass transmission with matched scan-wise RT masks and CE.
    ExperimentalTrajectory,
}

impl AcquisitionPolicy {
    fn is_single_interval(&self) -> bool {
        *self == Self::SingleInterval
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "AcquisitionPolicy::is_single_interval")]
    pub acquisition: AcquisitionPolicy,
    pub im: ImSizing,
    pub rt_context: ContextExtent,
    /// RT-supported centroid admission; absent means native-only.
    pub rt_consistency: Option<super::RtConsistencyConfig>,
    pub im_context: ContextExtent,
    /// Zero means uncapped.
    pub peak_cap: usize,
    pub min_ms2_ion_count: f64,
    pub boundary: BoundaryPolicy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            acquisition: AcquisitionPolicy::default(),
            im: ImSizing::default(),
            rt_context: ContextExtent::default(),
            rt_consistency: None,
            im_context: ContextExtent {
                before: 0,
                after: 0,
            },
            peak_cap: 300,
            min_ms2_ion_count: 2.0,
            boundary: BoundaryPolicy::KeepMasked,
        }
    }
}

impl Config {
    pub fn validate(&self) -> TimsResult<()> {
        if let Some(rt) = &self.rt_consistency {
            rt.validate()?;
            if rt.minimum_support
                > self
                    .rt_context
                    .before
                    .saturating_add(self.rt_context.after)
                    .saturating_add(1)
                || (rt.require_both_sides
                    && (self.rt_context.before == 0
                        || self.rt_context.after == 0))
            {
                return Err(TimsError::new(
                    "RT consistency requires sufficient acquisition-step context",
                ));
            }
        }
        let extents = [
            self.rt_context.before,
            self.rt_context.after,
            self.im_context.before,
            self.im_context.after,
        ];
        if self.schema_version != 1
            || !self.min_ms2_ion_count.is_finite()
            || self.min_ms2_ion_count < 1.0
            || self.min_ms2_ion_count.fract() != 0.0
            || self.min_ms2_ion_count > u32::MAX as f64
            || extents.iter().any(|&x| x > 32)
            || (self.rt_context.before + self.rt_context.after + 1)
                * (self.im_context.before + self.im_context.after + 1)
                > 1024
        {
            return Err(TimsError::new(
                "unsupported native neighborhood configuration",
            ));
        }
        match self.im {
            ImSizing::NativeFwhm {
                padding_left,
                padding_right,
            } => {
                if [padding_left, padding_right]
                    .iter()
                    .any(|x| !x.is_finite() || *x < 0.0 || *x > 32.0)
                {
                    return Err(TimsError::new(
                        "IM padding must be finite and in 0..=32 FWHM",
                    ));
                }
            },
            ImSizing::Scans {
                left,
                right,
                stride,
            } => {
                if stride == 0
                    || [left, right, stride]
                        .iter()
                        .any(|&x| x > u32::MAX as usize / 2)
                {
                    return Err(TimsError::new("invalid explicit IM geometry"));
                }
            },
        }
        Ok(())
    }

    pub fn resolve(
        &self,
        native_scan_fwhm: usize,
    ) -> TimsResult<ResolvedConfig> {
        self.validate()?;
        if native_scan_fwhm == 0 || native_scan_fwhm > u32::MAX as usize / 128 {
            return Err(TimsError::new("invalid native MS1 scan-kernel width"));
        }
        let (left, right, stride) = match self.im {
            ImSizing::NativeFwhm {
                padding_left,
                padding_right,
            } => (
                native_scan_fwhm / 2
                    + (padding_left * native_scan_fwhm as f64).ceil() as usize,
                native_scan_fwhm / 2
                    + (padding_right * native_scan_fwhm as f64).ceil() as usize,
                (native_scan_fwhm / 2).max(1),
            ),
            ImSizing::Scans {
                left,
                right,
                stride,
            } => (left, right, stride),
        };
        Ok(ResolvedConfig {
            request: self.clone(),
            native_scan_fwhm,
            im_left_scans: left,
            im_right_scans: right,
            anchor_stride_scans: stride,
            im_context_step_scans: left.max(right) + 1,
            response_definition: "native_kernel_weighted_apex",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedConfig {
    pub request: Config,
    pub native_scan_fwhm: usize,
    pub im_left_scans: usize,
    pub im_right_scans: usize,
    pub anchor_stride_scans: usize,
    pub im_context_step_scans: usize,
    pub response_definition: &'static str,
}
