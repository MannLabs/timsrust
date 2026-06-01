mod cache;
mod cloud_store;
#[cfg(feature = "async")]
mod runtime;
mod uri;

pub mod formats;

pub use cache::{global_cache, set_global_cache, CacheError, FileCache};
pub use cloud_store::CloudError;
pub use uri::{Uri, UriError};

pub(crate) fn range(
    range: impl std::ops::RangeBounds<usize>,
    len: usize,
) -> std::ops::Range<usize> {
    use std::ops::Bound;
    let start = match range.start_bound() {
        Bound::Included(&s) => s,
        Bound::Excluded(&s) => s + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&e) => e + 1,
        Bound::Excluded(&e) => e,
        Bound::Unbounded => len,
    };
    start..end
}
