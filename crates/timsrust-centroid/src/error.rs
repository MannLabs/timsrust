/// Error type for the timsrust_centroid crate.
///
/// Wraps an error message string and implements `std::error::Error`.
///
/// # Example
/// ```
/// use timsrust_centroid::TimsError;
/// let err = TimsError::new("Something went wrong");
/// assert_eq!(format!("{}", err), "Something went wrong");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimsError {
    message: String,
}

impl TimsError {
    /// Create a new `TimsError` with the given message.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message to store.
    ///
    /// # Example
    /// ```
    /// use timsrust_centroid::TimsError;
    /// let err = TimsError::new("error");
    /// assert_eq!(format!("{}", err), "error");
    /// ```
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TimsError {
    /// Formats the error message for display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TimsError {}

/// Result type for functions that can return a `TimsError`.
///
/// # Example
/// ```
/// use timsrust_centroid::{TimsResult, TimsError};
/// fn foo() -> TimsResult<i32> {
///     Err(TimsError::new("fail"))
/// }
/// assert!(foo().is_err());
/// ```
pub type TimsResult<T> = Result<T, TimsError>;
