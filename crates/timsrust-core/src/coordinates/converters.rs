pub trait Converter<Input, Output> {
    fn convert(&self, value: Input) -> Output;

    fn batch_convert(&self, values: &[Input]) -> Vec<Output>
    where
        Input: Clone,
    {
        values.iter().map(|v| self.convert(v.clone())).collect()
    }
}

pub trait InvertibleConverter<Input, Output>:
    Converter<Input, Output> + Converter<Output, Input>
{
}

impl<Input, Output, T> InvertibleConverter<Input, Output> for T where
    T: Converter<Input, Output> + Converter<Output, Input>
{
}

pub trait ConvertibleTo<Output>: Sized + Copy {
    fn convert<C: Converter<Self, Output>>(&self, converter: &C) -> Output {
        converter.convert(*self)
    }
}

// impl<Input, Output, T, U> Converter<Input, Output> for T
// where
//     T: Deref<Target = U>,
//     U: Converter<Input, Output>,
impl<Input, Output, T> Converter<Input, Output> for &T
where
    T: Converter<Input, Output>,
{
    fn convert(&self, value: Input) -> Output {
        (*self).convert(value)
    }

    // fn batch_convert(&self, values: &[Input]) -> Vec<Output>
    // where
    //     Input: Clone,
    // {
    //     (*self).batch_convert(values)
    // }
}

#[derive(Debug, Clone)]
pub struct BitConverter();
