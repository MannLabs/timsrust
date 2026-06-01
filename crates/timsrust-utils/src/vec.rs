use crate::custom_error;

custom_error!(pub SparseVecError);

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
enum SparseVecEnum<T> {
    /// Dense representation: all elements stored directly.
    Dense(Vec<T>),
    /// Sparse representation: unique elements and their offsets.
    Sparse(Vec<T>, Vec<usize>),
}

impl<T> Default for SparseVecEnum<T> {
    fn default() -> Self {
        SparseVecEnum::Dense(Vec::new())
    }
}

/// A vector that can be stored in either dense or sparse format.
///
/// Useful for compressing repeated values.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct SparseVec<T> {
    inner: SparseVecEnum<T>,
}

impl<T> SparseVec<T> {
    /// Creates a new empty sparse vector in dense format.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let sv: SparseVec<u32> = SparseVec::new();
    /// assert_eq!(sv.len(), 0);
    /// assert!(sv.is_dense());
    /// ```
    pub fn new() -> Self {
        Self::dense()
    }

    /// Creates a new empty sparse vector in sparse format.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let sv: SparseVec<u32> = SparseVec::sparse();
    /// assert_eq!(sv.len(), 0);
    /// assert!(sv.is_sparse());
    /// ```
    pub fn sparse() -> Self {
        Self {
            inner: SparseVecEnum::Sparse(Vec::new(), vec![0]),
        }
    }

    /// Creates a new sparse vector from values and offsets.
    ///
    /// The `vals` vector contains the unique values, and `offsets` specifies the start of each run.
    /// The length of `offsets` must be one more than the length of `vals`, and must be non-decreasing.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let vals = vec![1, 2];
    /// let offsets = vec![0, 2, 5];
    /// let sv = SparseVec::sparse_from_offsets(vals, offsets).unwrap();
    /// assert_eq!(sv.iter().collect::<Vec<_>>(), vec![1, 1, 2, 2, 2]);
    /// ```
    pub fn sparse_from_offsets(
        vals: Vec<T>,
        offsets: Vec<usize>,
    ) -> Result<Self, SparseVecError> {
        if offsets.len() != vals.len() + 1 {
            return Err(SparseVecError::new(
                "Offsets must be one element longer than vals".to_string(),
            ));
        }
        if !offsets.windows(2).all(|w| w[0] <= w[1]) {
            return Err(SparseVecError::new(
                "Offsets must be non-decreasing".to_string(),
            ));
        }
        let result = Self {
            inner: SparseVecEnum::Sparse(vals, offsets),
        };
        Ok(result)
    }

    /// Returns the offsets array if the vector is in sparse format, or `None` if dense.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let vals = vec![1, 2];
    /// let offsets = vec![0, 2, 5];
    /// let sv = SparseVec::sparse_from_offsets(vals, offsets.clone()).unwrap();
    /// assert_eq!(sv.get_offsets(), Some(&offsets[..]));
    ///
    /// let dv: SparseVec<u32> = SparseVec::dense();
    /// assert_eq!(dv.get_offsets(), None);
    /// ```
    pub fn get_offsets(&self) -> Option<&[usize]> {
        match &self.inner {
            SparseVecEnum::Dense(_) => None,
            SparseVecEnum::Sparse(_, offsets) => Some(offsets),
        }
    }

    /// Creates a new empty sparse vector in dense format.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let sv: SparseVec<u32> = SparseVec::dense();
    /// assert_eq!(sv.len(), 0);
    /// assert!(sv.is_dense());
    /// ```
    pub fn dense() -> Self {
        Self {
            inner: SparseVecEnum::Dense(Vec::new()),
        }
    }

    /// Returns the total number of elements in the vector.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::dense();
    /// sv.push(1);
    /// sv.push(2);
    /// assert_eq!(sv.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        match &self.inner {
            SparseVecEnum::Dense(v) => v.len(),
            SparseVecEnum::Sparse(_, offsets) => {
                *offsets.last().expect("cannot be empty")
            },
        }
    }

    /// Appends an element to the vector.
    ///
    /// In sparse format, consecutive equal values are compressed.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::sparse();
    /// sv.push(1);
    /// sv.push(1);
    /// sv.push(2);
    /// assert_eq!(sv.len(), 3);
    /// ```
    pub fn push(&mut self, val: T)
    where
        T: PartialEq,
    {
        match &mut self.inner {
            SparseVecEnum::Dense(v) => v.push(val),
            SparseVecEnum::Sparse(vals, offsets) => {
                if vals.last() != Some(&val) {
                    let offset = offsets.last().expect("cannot be empty");
                    vals.push(val);
                    offsets.push(*offset);
                }
                *offsets.last_mut().expect("cannot be empty") += 1;
            },
        }
    }

    /// Compresses the vector from dense to sparse format if it saves memory.
    ///
    /// Only compresses if the sparse representation uses less memory than the dense one.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::new();
    /// for i in 0..100 {
    ///     sv.push(i / 25);
    /// }
    /// assert_eq!(sv.len(), 100);
    /// assert!(sv.is_dense());
    /// let mut sv2 = sv.clone();
    /// sv2.compress();
    /// assert!(sv2.is_sparse());
    /// assert_eq!(Vec::from(sv), Vec::from(sv2));
    /// ```
    pub fn compress(&mut self)
    where
        T: PartialEq + Copy,
    {
        match &mut self.inner {
            SparseVecEnum::Dense(dense) => {
                let dense_size = dense.len() * std::mem::size_of::<T>();
                let mut sparse = SparseVec::sparse();
                for value in dense.iter() {
                    sparse.push(*value);
                    if sparse.size_of() >= dense_size {
                        return;
                    }
                }
                *self = sparse;
            },
            SparseVecEnum::Sparse(_, _) => {},
        }
    }

    /// Returns the memory usage in bytes.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut dv: SparseVec<u32> = SparseVec::dense();
    /// for i in 0..100 {
    ///     dv.push(i / 25);
    /// }
    /// let mut sv = dv.clone();
    /// sv.compress();
    /// assert!(sv.size_of() < dv.size_of());
    /// ```
    pub fn size_of(&self) -> usize {
        std::mem::size_of::<Self>()
            + match &self.inner {
                SparseVecEnum::Dense(v) => v.len() * std::mem::size_of::<T>(),
                SparseVecEnum::Sparse(vals, _) => {
                    vals.len()
                        * (std::mem::size_of::<T>()
                            + std::mem::size_of::<usize>())
                        + std::mem::size_of::<usize>()
                },
            }
    }

    /// Returns an iterator over all elements in the vector.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::dense();
    /// sv.push(1);
    /// sv.push(2);
    /// let v: Vec<_> = sv.iter().collect();
    /// assert_eq!(v, vec![1, 2]);
    /// ```
    pub fn iter(&self) -> SparseVecIter<'_, T> {
        SparseVecIter {
            inner: self,
            index: 0,
            offset_index: 0,
        }
    }

    /// Returns true if the vector contains no elements.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let sv: SparseVec<u32> = SparseVec::sparse();
    /// assert_eq!(sv.len(), 0);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the vector is in dense format.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv: SparseVec<u32> = SparseVec::dense();
    /// assert!(sv.is_dense());
    /// ```
    pub fn is_dense(&self) -> bool {
        matches!(self.inner, SparseVecEnum::Dense(_))
    }

    /// Returns true if the vector is in sparse format.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let sv: SparseVec<u32> = SparseVec::sparse();
    /// assert!(sv.is_sparse());
    /// ```
    pub fn is_sparse(&self) -> bool {
        matches!(self.inner, SparseVecEnum::Sparse(_, _))
    }

    /// Returns the indices that would sort the vector.
    ///
    /// The returned vector contains the indices of the elements in sorted order.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::dense();
    /// sv.push(3);
    /// sv.push(1);
    /// sv.push(2);
    /// let sorted_indices = sv.argsort();
    /// assert_eq!(sorted_indices, vec![1, 2, 0]);
    ///
    /// let mut sv = SparseVec::sparse();
    /// sv.push(2);
    /// sv.push(2);
    /// sv.push(1);
    /// sv.push(1);
    /// sv.push(1);
    /// let sorted_indices = sv.argsort();
    /// assert_eq!(sorted_indices, vec![2, 3, 0, 1, 2]);
    /// ```
    pub fn argsort(&self) -> Vec<usize>
    where
        T: Ord + Copy,
    {
        match &self.inner {
            SparseVecEnum::Dense(dense) => {
                let mut indices = (0..self.len()).collect::<Vec<_>>();
                indices.sort_by_key(|&a| dense[a]);
                indices
            },
            SparseVecEnum::Sparse(sparse, offsets) => {
                let mut indices = (0..sparse.len()).collect::<Vec<_>>();
                indices.sort_by_key(|&a| sparse[a]);
                let mut result = Vec::with_capacity(self.len());
                for (i, pos) in indices.iter().enumerate() {
                    let mut offset = offsets[*pos];
                    for _ in offsets[i]..offsets[i + 1] {
                        result.push(offset);
                        offset += 1;
                    }
                }
                result
            },
        }
    }
}

/// Iterator for [SparseVec].
pub struct SparseVecIter<'a, T> {
    inner: &'a SparseVec<T>,
    index: usize,
    offset_index: usize,
}

impl<T: Copy> Iterator for SparseVecIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.inner.inner {
            SparseVecEnum::Dense(v) => {
                if self.index < v.len() {
                    self.index += 1;
                    return Some(v[self.index - 1]);
                }
            },
            SparseVecEnum::Sparse(vals, offsets) => {
                while self.index < offsets.len() {
                    if self.offset_index < offsets[self.index] {
                        self.offset_index += 1;
                        return Some(vals[self.index - 1]);
                    }
                    self.index += 1;
                }
            },
        }
        None
    }
}

impl<T: Copy> From<SparseVec<T>> for Vec<T> {
    /// Converts a `SparseVec` into a regular `Vec`, expanding any compressed values.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::sparse();
    /// sv.push(1);
    /// sv.push(1);
    /// sv.push(2);
    /// let v: Vec<_> = Vec::from(sv);
    /// assert_eq!(v, vec![1, 1, 2]);
    /// ```
    fn from(sparse: SparseVec<T>) -> Self {
        sparse.iter().collect()
    }
}

impl<T: Copy> From<&SparseVec<T>> for Vec<T> {
    /// Converts a `SparseVec` into a regular `Vec`, expanding any compressed values.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut sv = SparseVec::sparse();
    /// sv.push(1);
    /// sv.push(1);
    /// sv.push(2);
    /// let v: Vec<_> = Vec::from(sv);
    /// assert_eq!(v, vec![1, 1, 2]);
    /// ```
    fn from(sparse: &SparseVec<T>) -> Self {
        sparse.iter().collect()
    }
}

impl<T: Copy + PartialEq> From<Vec<T>> for SparseVec<T> {
    /// Converts a regular `Vec` into a compressed `SparseVec`.
    ///
    /// # Examples
    /// ```
    /// use timsrust_utils::vec::SparseVec;
    /// let mut v = Vec::new();
    /// for i in 0..100 {
    ///     v.push(i / 25);
    /// }
    /// let sv: SparseVec<_> = v.clone().into();
    /// assert_eq!(sv.iter().collect::<Vec<_>>(), v);
    /// ```
    fn from(vec: Vec<T>) -> Self {
        let mut sparse = Self {
            inner: SparseVecEnum::Dense(vec),
        };
        sparse.compress();
        sparse
    }
}

pub fn argsort<T: Ord>(vec: &[T]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..vec.len()).collect();
    indices.sort_by_key(|&i| &vec[i]);
    indices
}

pub fn group_and_sum<T: Ord + Copy, U: std::ops::Add<Output = U> + Copy>(
    groups: Vec<T>,
    values: Vec<U>,
) -> (Vec<T>, Vec<U>) {
    if groups.is_empty() {
        return (vec![], vec![]);
    }
    let order: Vec<usize> = argsort(&groups);
    let mut new_groups: Vec<T> = Vec::with_capacity(order.len());
    let mut new_values: Vec<U> = Vec::with_capacity(order.len());
    let mut current_group: T = groups[order[0]];
    let mut current_value: U = values[order[0]];
    for &index in &order[1..] {
        let group: T = groups[index];
        let value: U = values[index];
        if group != current_group {
            new_groups.push(current_group);
            new_values.push(current_value);
            current_group = group;
            current_value = value;
        } else {
            current_value = current_value + value;
        };
    }
    new_groups.push(current_group);
    new_values.push(current_value);
    (new_groups, new_values)
}

pub fn find_sparse_local_maxima_mask(
    indices: &[u32],
    values: &[u64],
    window: u32,
) -> Vec<bool> {
    let mut local_maxima: Vec<bool> = vec![true; indices.len()];
    for (index, sparse_index) in indices.iter().enumerate() {
        let current_intensity: u64 = values[index];
        for (_next_index, next_sparse_index) in
            indices[index + 1..].iter().enumerate()
        {
            let next_index: usize = _next_index + index + 1;
            let next_value: u64 = values[next_index];
            if (next_sparse_index - sparse_index) <= window {
                if current_intensity < next_value {
                    local_maxima[index] = false
                } else {
                    local_maxima[next_index] = false
                }
            } else {
                break;
            }
        }
    }
    local_maxima
}

pub fn filter_with_mask<T: Copy>(vec: &[T], mask: &[bool]) -> Vec<T> {
    (0..vec.len())
        .filter(|&x| mask[x])
        .map(|x| vec[x])
        .collect()
}

pub fn is_strictly_ascending<T: std::cmp::PartialOrd>(vec: &[T]) -> bool {
    vec.windows(2).all(|w| w[0] < w[1])
}

pub fn arg_max<T: Ord>(kernel: &[T]) -> Option<usize> {
    kernel
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.cmp(b.1))
        .map(|(idx, _)| idx)
}

pub fn get_top_n<T: PartialOrd>(vec: &[T], n: usize) -> Vec<usize> {
    let top_n = if n == 0 { vec.len() } else { n.min(vec.len()) };
    if top_n == vec.len() {
        return (0..vec.len()).collect();
    }
    let mut indexed = vec.iter().enumerate().collect::<Vec<_>>();
    indexed.select_nth_unstable_by(top_n, |a, b| {
        b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut top_indices =
        indexed[..top_n].iter().map(|(i, _)| *i).collect::<Vec<_>>();
    top_indices.sort_unstable();
    top_indices
}

pub fn extract_kernel(kernel: &[u64], threshold: f32) -> Option<Vec<f32>> {
    let max_val = *kernel.iter().max()? as f32;
    let kernel = kernel
        .iter()
        .map(|&x| x as f32 / max_val)
        .collect::<Vec<_>>();
    let first = kernel.iter().position(|&x| x > threshold)? - 1;
    let last = kernel.iter().rposition(|&x| x > threshold)? + 1;
    let kernel = kernel[first..last + 1].to_vec();
    Some(kernel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_push_and_len() {
        let mut sv = SparseVec::dense();
        sv.push(1);
        sv.push(2);
        assert_eq!(sv.len(), 2);
        assert_eq!(Vec::from(sv.clone()), vec![1, 2]);
    }

    #[test]
    fn test_sparse_push_and_len() {
        let mut sv = SparseVec::sparse();
        sv.push(1);
        sv.push(1);
        sv.push(2);
        assert_eq!(sv.len(), 3);
        assert_eq!(Vec::from(sv.clone()), vec![1, 1, 2]);
    }

    #[test]
    fn test_compress_dense_to_sparse() {
        let mut sv = SparseVec::dense();
        for i in 0..100 {
            sv.push(i / 25);
        }
        assert!(sv.is_dense());
        sv.compress();
        assert!(sv.is_sparse());
    }

    #[test]
    fn test_size_of() {
        let mut sv = SparseVec::dense();
        for i in 0..100 {
            sv.push(i / 25);
        }
        let dense_size = sv.size_of();
        sv.compress();
        let sparse_size = sv.size_of();
        assert!(sparse_size < dense_size);
    }

    #[test]
    fn test_iter_dense() {
        let mut sv = SparseVec::dense();
        sv.push(1);
        sv.push(2);
        let v: Vec<_> = sv.iter().collect();
        assert_eq!(v, vec![1, 2]);
    }

    #[test]
    fn test_iter_sparse() {
        let mut sv = SparseVec::sparse();
        sv.push(1);
        sv.push(1);
        sv.push(2);
        let v: Vec<_> = sv.iter().collect();
        assert_eq!(v, vec![1, 1, 2]);
    }

    #[test]
    fn test_from_vec() {
        let mut v = Vec::new();
        for i in 0..123 {
            v.push(i / 25);
        }
        let sv: SparseVec<_> = SparseVec::from(v.clone());
        assert_eq!(sv.iter().collect::<Vec<_>>(), v);
    }

    #[test]
    fn test_empty_sparsevec() {
        let sv: SparseVec<u32> = SparseVec::sparse();
        assert_eq!(sv.len(), 0);
        let v: Vec<_> = sv.iter().collect();
        assert!(v.is_empty());
    }

    #[test]
    fn test_empty_densevec() {
        let sv: SparseVec<u32> = SparseVec::dense();
        assert_eq!(sv.len(), 0);
        let v: Vec<_> = sv.iter().collect();
        assert!(v.is_empty());
    }
}
