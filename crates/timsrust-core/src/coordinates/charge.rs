use crate::CoordinateError;

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub struct Charge(std::num::NonZeroI8);

impl std::fmt::Display for Charge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", i8::from(*self))
    }
}

impl From<Charge> for i8 {
    fn from(value: Charge) -> Self {
        value.0.get()
    }
}

impl From<Charge> for i16 {
    fn from(value: Charge) -> Self {
        value.0.get() as i16
    }
}

impl From<Charge> for i32 {
    fn from(value: Charge) -> Self {
        value.0.get() as i32
    }
}

impl From<Charge> for i64 {
    fn from(value: Charge) -> Self {
        value.0.get() as i64
    }
}

impl From<Charge> for isize {
    fn from(value: Charge) -> Self {
        value.0.get() as isize
    }
}

impl TryFrom<i8> for Charge {
    type Error = CoordinateError;
    fn try_from(value: i8) -> Result<Self, Self::Error> {
        let x = std::num::NonZeroI8::new(value).ok_or_else(|| {
            CoordinateError::new("Charge cannot be zero".to_string())
        })?;
        Ok(Self(x))
    }
}

impl TryFrom<i16> for Charge {
    type Error = CoordinateError;
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<i32> for Charge {
    type Error = CoordinateError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<i64> for Charge {
    type Error = CoordinateError;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<isize> for Charge {
    type Error = CoordinateError;
    fn try_from(value: isize) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<u8> for Charge {
    type Error = CoordinateError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<u16> for Charge {
    type Error = CoordinateError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<u32> for Charge {
    type Error = CoordinateError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<u64> for Charge {
    type Error = CoordinateError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}

impl TryFrom<usize> for Charge {
    type Error = CoordinateError;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let x = i8::try_from(value)
            .map_err(|e| CoordinateError::new(e.to_string()))?;
        Charge::try_from(x)
    }
}
