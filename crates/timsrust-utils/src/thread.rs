use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
pub struct SyncedTimer(Synced<Duration>);

impl SyncedTimer {
    pub fn update_with_func<F, T>(&self, func: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = func(); // Call the function with no arguments here
        let elapsed = start.elapsed();
        self.0.with_lock(|t| *t += elapsed).unwrap();
        result
    }

    pub fn get(&self) -> Duration {
        self.0.with_lock(|t| *t).unwrap()
    }
}

#[derive(Debug)]
pub struct Synced<T> {
    internal: Arc<Mutex<T>>,
}

/// A thread-safe wrapper that provides synchronized access to an inner value.
///
/// `Synced<T>` wraps a value of type `T` in an `Arc<Mutex<T>>`, providing
/// safe concurrent access across multiple threads.
impl<T> Synced<T> {
    /// Executes a function with exclusive access to the inner value.
    ///
    /// This method acquires a lock on the internal mutex, passes a mutable
    /// reference to the inner value to the provided function, and returns
    /// the result.
    ///
    /// # Arguments
    ///
    /// * `func` - A closure that takes a mutable reference to `T` and returns `R`
    ///
    /// # Returns
    ///
    /// * `Ok(R)` - The result of the function if the lock was acquired successfully
    /// * `Err(PoisonError)` - If the mutex was poisoned (a thread panicked while holding the lock)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use timsrust_utils::thread::Synced;
    ///
    /// let synced = Synced::from(vec![1, 2, 3]);
    ///
    /// let sum = synced.with_lock(|data| {
    ///     data.iter().sum::<i32>()
    /// }).unwrap();
    ///
    /// assert_eq!(sum, 6);
    /// ```
    pub fn with_lock<F, R>(
        &self,
        func: F,
    ) -> Result<R, std::sync::PoisonError<std::sync::MutexGuard<'_, T>>>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.internal.lock()?;
        Ok(func(&mut guard))
    }

    /// Attempts to extract the inner value, consuming the `Synced` wrapper.
    ///
    /// This method will only succeed if this is the last reference to the
    /// internal `Arc` and the mutex is not poisoned.
    ///
    /// # Returns
    ///
    /// * `Some(T)` - The inner value if this was the last `Arc` reference and the mutex was not poisoned
    /// * `None` - If there are other references to the `Arc` or the mutex was poisoned
    ///
    /// # Examples
    ///
    /// ```rust
    /// use timsrust_utils::thread::Synced;
    ///
    /// let synced = Synced::from(42);
    /// let value = synced.try_finalize();
    ///
    /// assert_eq!(value, Some(42));
    /// ```
    ///
    /// ```rust
    /// use timsrust_utils::thread::Synced;
    ///
    /// let synced = Synced::from(42);
    /// let cloned = synced.clone();
    ///
    /// // Cannot finalize because there are multiple references
    /// let value = synced.try_finalize();
    /// assert_eq!(value, None);
    /// ```
    pub fn try_finalize(self) -> Option<T> {
        Arc::into_inner(self.internal)?.into_inner().ok()
    }
}

impl<T: Default> Default for Synced<T> {
    fn default() -> Self {
        Synced {
            internal: Arc::new(Mutex::new(T::default())),
        }
    }
}

impl<T> From<T> for Synced<T> {
    fn from(value: T) -> Self {
        Synced {
            internal: Arc::new(Mutex::new(value)),
        }
    }
}

impl<T> Clone for Synced<T> {
    fn clone(&self) -> Self {
        Synced {
            internal: Arc::clone(&self.internal),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::vec::arg_max;

    use super::*;

    fn scale_to_u64(vec: &[f32], upper_bound: f32) -> Vec<u64> {
        vec.iter()
            .map(|&x| {
                let t = (upper_bound * x) as u64;
                t.min(upper_bound as u64)
            })
            .collect()
    }

    #[test]
    fn test_synced_with_lock_and_finalize() {
        let synced = Synced::from(10);
        let res = synced.with_lock(|v| {
            *v += 5;
            *v
        });
        assert_eq!(res.unwrap(), 15);
        let finalized = synced.try_finalize();
        assert_eq!(finalized, Some(15));
    }

    #[test]
    fn test_synced_default() {
        let synced: Synced<i32> = Synced::default();
        let val = synced.try_finalize().unwrap();
        assert_eq!(val, 0);
    }

    #[test]
    fn test_synced_clone() {
        let synced = Synced::from(42);
        {
            let cloned = synced.clone();
            let _ = cloned.with_lock(|v| *v += 1);
        }
        let orig = synced.try_finalize().unwrap();
        assert_eq!(orig, 43);
    }

    #[test]
    fn test_arg_max() {
        let v = vec![1, 3, 2, 5, 4];
        let idx = arg_max(&v);
        assert_eq!(idx, Some(3));
        let empty: Vec<i32> = vec![];
        assert_eq!(arg_max(&empty), None);
    }

    #[test]
    fn test_scale_to_u64() {
        let v = vec![0.0, 0.5, 1.0, 1.5];
        let scaled = scale_to_u64(&v, 10.0);
        assert_eq!(scaled, vec![0, 5, 10, 10]);
    }
}
