use std::ops::{AddAssign, Index, IndexMut};

use crate::custom_error;

custom_error!(pub NDArrayError);

/// A simple N-dimensional array type with shape, strides, and contiguous data storage.
///
/// # Example
/// ```
/// use timsrust_utils::ndarray::NDArray;
/// let arr = NDArray::new([2, 3], vec![1, 2, 3, 4, 5, 6]).unwrap();
/// assert_eq!(arr[[1, 2]], 6);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NDArray<T, const N: usize> {
    shape: [usize; N],
    strides: [usize; N],
    data: Vec<T>,
}

impl<T, const N: usize> NDArray<T, N> {
    /// Creates a new NDArray with the given shape and data.
    ///
    /// # Arguments
    ///
    /// * `shape` - The shape of the array as an array of dimension sizes.
    /// * `data` - The data to fill the array, must match the product of shape dimensions.
    ///
    /// # Errors
    ///
    /// Returns `TimsUtilsError` if the shape and data length are incompatible.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// assert_eq!(arr.shape(), [2, 2]);
    /// assert_eq!(arr[[1, 1]], 4);
    /// ```
    pub fn new(shape: [usize; N], data: Vec<T>) -> Result<Self, NDArrayError> {
        if shape.iter().product::<usize>() != data.len() {
            return Err(NDArrayError::new(format!(
                "Incompatible shapes: {:?} and {:?}",
                shape,
                data.len()
            )));
        }
        let mut strides = [0; N];
        let mut stride = 1;
        for (i, &dim) in shape.iter().rev().enumerate() {
            strides[N - 1 - i] = stride;
            stride *= dim;
        }
        let result = Self {
            shape,
            strides,
            data,
        };
        Ok(result)
    }

    /// Returns the shape of the array.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr = NDArray::new([2, 3], vec![1, 2, 3, 4, 5, 6]).unwrap();
    /// assert_eq!(arr.shape(), [2, 3]);
    /// ```
    pub fn shape(&self) -> [usize; N] {
        self.shape
    }

    /// Computes the flat index in the data vector for the given N-dimensional indices.
    ///
    /// # Arguments
    ///
    /// * `indices` - The N-dimensional indices.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// let idx = arr.index([1, 1]);
    /// assert_eq!(idx, 3);
    /// ```
    pub fn index(&self, indices: [usize; N]) -> usize {
        indices
            .iter()
            .zip(self.strides.iter())
            .map(|(&idx, &stride)| idx * stride)
            .sum()
    }

    /// Converts a flat index into N-dimensional indices.
    ///
    /// # Arguments
    ///
    /// * `idx` - The flat index.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// let indices = arr.inverted_index(3);
    /// assert_eq!(indices, [1, 1]);
    /// ```
    pub fn inverted_index(&self, mut idx: usize) -> [usize; N] {
        let mut indices = [0; N];
        // for d in 0..N {
        for (d, index) in indices.iter_mut().enumerate().take(N) {
            *index = idx / self.strides[d];
            idx %= self.strides[d];
        }
        indices
    }
}

impl<T: Default + Copy + AddAssign, const N: usize> NDArray<T, N> {
    /// Projects the array along the specified axis, summing over all other axes.
    ///
    /// # Arguments
    ///
    /// * `axis` - The axis to project onto.
    ///
    /// # Returns
    ///
    /// A vector of values, one for each index along the specified axis.
    ///
    /// # Panics
    ///
    /// Panics if the axis is out of bounds.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// let proj = arr.project_axis(0);
    /// assert_eq!(proj, vec![1+2, 3+4]);
    /// ```
    pub fn project_axis(&self, axis: usize) -> Vec<T> {
        assert!(axis < N, "Axis out of bounds");
        let mut result = vec![T::default(); self.shape[axis]];
        for (i, value) in self.data.iter().enumerate() {
            let indices = self.inverted_index(i);
            result[indices[axis]] += *value;
        }
        result
    }
}

impl<T: AddAssign + Copy, const N: usize> AddAssign for NDArray<T, N> {
    /// Adds another NDArray to this one, elementwise.
    ///
    /// # Panics
    ///
    /// Panics if the shapes do not match.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let mut a = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// let b = NDArray::new([2, 2], vec![10, 20, 30, 40]).unwrap();
    /// a += b;
    /// assert_eq!(a[[0, 0]], 11);
    /// assert_eq!(a[[1, 1]], 44);
    /// ```
    fn add_assign(&mut self, other: Self) {
        assert_eq!(self.shape(), other.shape());
        other
            .data
            .into_iter()
            .enumerate()
            .for_each(|(i, value)| self.data[i] += value);
    }
}

impl<T: Default, const N: usize> NDArray<T, N> {
    /// Creates an empty NDArray with the given shape, filled with default values.
    ///
    /// # Arguments
    ///
    /// * `shape` - The shape of the array.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr: NDArray<i32, 2> = NDArray::empty([2, 2]);
    /// assert_eq!(arr.shape(), [2, 2]);
    /// assert_eq!(arr[[1,1]], 0);
    /// ```
    pub fn empty(shape: [usize; N]) -> Self {
        let size = shape.iter().product();
        let data = (0..size).map(|_| T::default()).collect();
        Self::new(shape, data).expect("Failed to create empty NDArray")
    }
}

impl<T, const N: usize> Index<[usize; N]> for NDArray<T, N> {
    type Output = T;
    /// Indexes the array using N-dimensional indices.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let arr = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// assert_eq!(arr[[1, 1]], 4);
    /// ```
    fn index(&self, indices: [usize; N]) -> &Self::Output {
        let idx = self.index(indices);
        &self.data[idx]
    }
}

impl<T, const N: usize> IndexMut<[usize; N]> for NDArray<T, N> {
    /// Mutable indexing using N-dimensional indices.
    ///
    /// # Example
    /// ```
    /// use timsrust_utils::ndarray::NDArray;
    /// let mut arr = NDArray::new([2, 2], vec![1, 2, 3, 4]).unwrap();
    /// arr[[0, 0]] = 10;
    /// assert_eq!(arr[[0, 0]], 10);
    /// ```
    fn index_mut(&mut self, indices: [usize; N]) -> &mut Self::Output {
        let idx = self.index(indices);
        &mut self.data[idx]
    }
}
