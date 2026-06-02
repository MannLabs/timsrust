use rayon::iter::plumbing::{Producer, ProducerCallback, bridge};
use rayon::prelude::*;

pub trait Reader<T, A = usize> {
    type Error: std::error::Error;
    fn get(&self, accessor: A) -> Result<T, Self::Error>;
}

// pub trait ReadableFrom<R, A = usize>: Sized {
//     type Error: std::error::Error;
//     fn from_reader(reader: &R, accessor: A) -> Result<Self, Self::Error>;
// }

// impl<T, R> Reader<T> for R
// where
//     T: ReadableFrom<R>,
// {
//     type Error = <T as ReadableFrom<R>>::Error;

//     fn get(&self, accessor: usize) -> Result<T, Self::Error> {
//         T::from_reader(self, accessor)
//     }
// }

pub trait IndexedReader<T, A = usize> {
    type Iter: Iterator<Item = A>;
    fn iter(&self) -> Self::Iter;
}

// pub trait IndexableFrom<R, A = usize>: Sized {
//     type Iter: Iterator<Item = A>;
//     fn iter_from_reader(reader: &R) -> Self::Iter;
// }

pub trait IterableReader<'a, T> {
    type Error: std::error::Error;
    fn iter(&'a self) -> impl Iterator<Item = Result<T, Self::Error>>;
    fn iter_ok(&'a self) -> impl Iterator<Item = T> {
        self.iter().filter_map(|res| res.ok())
    }
}

pub trait ParIterableReader<'a, T>: Sync {
    type Error: std::error::Error + Send;
    fn par_iter(
        &'a self,
    ) -> impl ParallelIterator<Item = Result<T, Self::Error>>;
    fn par_iter_ok(&'a self) -> impl ParallelIterator<Item = T>
    where
        T: Send,
    {
        self.par_iter().filter_map(|res| res.ok())
    }
}

pub struct ReaderIter<'a, R, T>
where
    R: IndexedReader<T>,
{
    reader: &'a R,
    iter: R::Iter,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, R, T> ReaderIter<'a, R, T>
where
    R: IndexedReader<T>,
{
    pub fn new(reader: &'a R) -> Self {
        ReaderIter {
            reader,
            iter: reader.iter(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, R, T> Iterator for ReaderIter<'a, R, T>
where
    R: IndexedReader<T> + Reader<T>,
{
    type Item = Result<T, <R as Reader<T>>::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.iter.next()?;
        Some(self.reader.get(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, R, T> IterableReader<'a, T> for R
where
    R: IndexedReader<T> + Reader<T>,
{
    type Error = <R as Reader<T>>::Error;

    fn iter(&'a self) -> impl Iterator<Item = Result<T, Self::Error>> {
        ReaderIter::new(self)
    }
}

pub struct ReaderParIter<'a, R, T> {
    reader: &'a R,
    indices: Vec<usize>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, R, T> ReaderParIter<'a, R, T>
where
    R: IndexedReader<T>,
{
    pub fn new(reader: &'a R) -> Self {
        ReaderParIter {
            reader,
            indices: reader.iter().collect(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, R, T> ParallelIterator for ReaderParIter<'a, R, T>
where
    R: IndexedReader<T> + Reader<T, Error: Send> + Sync,
    T: Send,
{
    type Item = Result<T, <R as Reader<T>>::Error>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}

struct ReaderProducer<'a, R, T> {
    reader: &'a R,
    indices: Vec<usize>,
    _marker: std::marker::PhantomData<T>,
}

struct ReaderProducerIter<'a, R, T> {
    reader: &'a R,
    indices: std::vec::IntoIter<usize>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, R, T> Iterator for ReaderProducerIter<'a, R, T>
where
    R: Reader<T>,
{
    type Item = Result<T, <R as Reader<T>>::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().map(|i| self.reader.get(i))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl<'a, R, T> DoubleEndedIterator for ReaderProducerIter<'a, R, T>
where
    R: Reader<T>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.indices.next_back().map(|i| self.reader.get(i))
    }
}

impl<'a, R, T> ExactSizeIterator for ReaderProducerIter<'a, R, T> where
    R: Reader<T>
{
}

impl<'a, R, T> Producer for ReaderProducer<'a, R, T>
where
    R: IndexedReader<T> + Reader<T, Error: Send> + Sync,
    T: Send,
{
    type Item = Result<T, <R as Reader<T>>::Error>;
    type IntoIter = ReaderProducerIter<'a, R, T>;

    fn into_iter(self) -> Self::IntoIter {
        ReaderProducerIter {
            reader: self.reader,
            indices: self.indices.into_iter(),
            _marker: std::marker::PhantomData,
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let right = self.indices[index..].to_vec();
        let mut left = self.indices;
        left.truncate(index);
        (
            ReaderProducer {
                reader: self.reader,
                indices: left,
                _marker: std::marker::PhantomData,
            },
            ReaderProducer {
                reader: self.reader,
                indices: right,
                _marker: std::marker::PhantomData,
            },
        )
    }
}

impl<'a, R, T> IndexedParallelIterator for ReaderParIter<'a, R, T>
where
    R: IndexedReader<T> + Reader<T> + Sync,
    T: Send,
    <R as Reader<T>>::Error: Send,
{
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::Consumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn len(&self) -> usize {
        self.indices.len()
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.callback(ReaderProducer {
            reader: self.reader,
            indices: self.indices,
            _marker: std::marker::PhantomData,
        })
    }
}

impl<'a, R, T> ParIterableReader<'a, T> for R
where
    R: IndexedReader<T> + Reader<T> + Sync,
    T: Send,
    <R as Reader<T>>::Error: Send,
{
    type Error = <R as Reader<T>>::Error;

    fn par_iter(
        &'a self,
    ) -> impl ParallelIterator<Item = Result<T, Self::Error>> {
        ReaderParIter::new(self)
    }
}
