macro_rules! value_dimension {
    (
        $value:ident
    ) => {
        #[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
        pub struct $value(f64);

        impl Eq for $value {}

        impl std::hash::Hash for $value {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.0.to_bits().hash(state);
            }
        }

        impl std::fmt::Display for $value {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:.6}", self.0)
            }
        }

        impl From<f32> for $value {
            fn from(value: f32) -> Self {
                Self(value as f64)
            }
        }

        impl From<f64> for $value {
            fn from(value: f64) -> Self {
                Self(value)
            }
        }

        impl From<$value> for f64 {
            fn from(value: $value) -> Self {
                value.0
            }
        }
    };
}

macro_rules! index_dimension {
    (
        $index:ident
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
        pub struct $index(std::num::NonZeroU32);

        impl std::fmt::Display for $index {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", u32::from(*self))
            }
        }

        impl From<$index> for u32 {
            fn from(index: $index) -> Self {
                index.0.get() - 1
            }
        }

        impl From<$index> for u64 {
            fn from(index: $index) -> Self {
                u32::from(index) as u64
            }
        }

        impl From<$index> for usize {
            fn from(index: $index) -> Self {
                u32::from(index) as usize
            }
        }

        impl From<$index> for i64 {
            fn from(index: $index) -> Self {
                u32::from(index) as i64
            }
        }

        impl From<$index> for f64 {
            fn from(index: $index) -> Self {
                f64::from(u32::from(index))
            }
        }

        impl TryFrom<$index> for i32 {
            type Error = CoordinateError;
            fn try_from(index: $index) -> Result<Self, Self::Error> {
                let index = u32::from(index);
                i32::try_from(index).map_err(|e| CoordinateError(e.to_string()))
            }
        }

        impl From<u8> for $index {
            fn from(index: u8) -> Self {
                let index = u32::from(index);
                Self::try_from(index).expect("Index must be non-zero")
            }
        }

        impl From<u16> for $index {
            fn from(index: u16) -> Self {
                let index = u32::from(index);
                Self::try_from(index).expect("Index must be non-zero")
            }
        }

        impl TryFrom<u32> for $index {
            type Error = CoordinateError;
            fn try_from(index: u32) -> Result<Self, Self::Error> {
                let x =
                    std::num::NonZeroU32::new(index + 1).ok_or_else(|| {
                        CoordinateError(format!(
                            "index {} too high for {}",
                            index,
                            stringify!($index)
                        ))
                    })?;
                Ok(Self(x))
            }
        }

        impl TryFrom<u64> for $index {
            type Error = CoordinateError;
            fn try_from(index: u64) -> Result<Self, Self::Error> {
                let x = u32::try_from(index)
                    .map_err(|e| CoordinateError(e.to_string()))?;
                Self::try_from(x)
            }
        }

        impl TryFrom<usize> for $index {
            type Error = CoordinateError;
            fn try_from(index: usize) -> Result<Self, Self::Error> {
                let x = u32::try_from(index)
                    .map_err(|e| CoordinateError(e.to_string()))?;
                Self::try_from(x)
            }
        }

        impl TryFrom<i8> for $index {
            type Error = CoordinateError;
            fn try_from(index: i8) -> Result<Self, Self::Error> {
                let x = u32::try_from(index)
                    .map_err(|e| CoordinateError(e.to_string()))?;
                Self::try_from(x)
            }
        }

        impl TryFrom<i16> for $index {
            type Error = CoordinateError;
            fn try_from(index: i16) -> Result<Self, Self::Error> {
                let x = u32::try_from(index)
                    .map_err(|e| CoordinateError(e.to_string()))?;
                Self::try_from(x)
            }
        }

        impl TryFrom<i32> for $index {
            type Error = CoordinateError;
            fn try_from(index: i32) -> Result<Self, Self::Error> {
                let x = u32::try_from(index)
                    .map_err(|e| CoordinateError(e.to_string()))?;
                Self::try_from(x)
            }
        }

        impl TryFrom<i64> for $index {
            type Error = CoordinateError;
            fn try_from(index: i64) -> Result<Self, Self::Error> {
                let x = u32::try_from(index)
                    .map_err(|e| CoordinateError(e.to_string()))?;
                Self::try_from(x)
            }
        }
    };
}
