//! Native, acquisition-aware observations with separate RT and IM context.

mod config;
pub(crate) mod raw_support;
mod reader;
mod rt_consistency;
mod source;
mod transmission;

pub use transmission::{
    MassTransmissionFootprint, ObservationDomain, ObservationSegment,
    TransmissionFootprint, TransmissionFrame,
};

pub use rt_consistency::{
    RtCandidateFrame, RtConsistencyConfig, RtPeakEvidence, RtSelectionPolicy,
    RtSupport,
};

pub use config::{
    AcquisitionPolicy, BoundaryPolicy, Config, ContextExtent, ImSizing,
    ResolvedConfig,
};
pub use reader::{
    Anchor, IterationStats, Neighborhood, NeighborhoodIter, NeighborhoodReader,
    Observation, Offset, UnavailableReason, View,
};
pub use source::{CentroidSource, NativeFrame, NativeSource};

#[cfg(test)]
mod tests;
