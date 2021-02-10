//! This module provides an extension to the [`Iterator`] trait that allows iterating by chunks. One
//! important property of our chunks, is that they implement [`ExactSizeIterator`], which is
//! required by the [`FromBytes`] trait.
//!
//! [`Iterator`]: std::iter::Iterator
//! [`ExactSizeIterator`]: std::iter::ExactSizeIterator
//! [`FromBytes`]: crate::message::FromBytes

use std::{
    cell::RefCell,
    cmp,
    fmt,
    iter::{ExactSizeIterator, Iterator},
    ops::Range,
};

pub trait ChunkableIterator: Iterator + Sized {
    /// Return an _iterable_ that can chunk the iterator.
    ///
    /// Yield subiterators (chunks) that each yield a fixed number of
    /// elements, determined by `size`. The last chunk will be shorter
    /// if there aren't enough elements.
    ///
    /// Note that the chunks *must* be fully consumed in the order
    /// they are yielded. Otherwise, they will panic.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// # // private items can't be tested with doc tests
    /// let chunks = vec![0, 1, 2, 3, 4].into_iter().chunks(2);
    /// let mut chunks_iter = chunks.into_iter();
    ///
    /// let mut chunk_1 = chunks_iter.next().unwrap();
    /// assert_eq!(chunk_1.next().unwrap(), 0);
    /// assert_eq!(chunk_1.next().unwrap(), 1);
    /// assert!(chunk_1.next().is_none());
    ///
    /// let mut chunk_2 = chunks_iter.next().unwrap();
    /// assert_eq!(chunk_2.next().unwrap(), 2);
    /// assert_eq!(chunk_2.next().unwrap(), 3);
    /// assert!(chunk_2.next().is_none());
    ///
    /// let mut chunk_3 = chunks_iter.next().unwrap();
    /// assert_eq!(chunk_3.next().unwrap(), 4);
    /// assert!(chunk_3.next().is_none());
    ///
    /// assert!(chunks_iter.next().is_none());
    /// ```
    ///
    /// Attempting to consume chunks out of order fails:
    ///
    /// ```compile_fail
    /// # // private items can't be tested with doc tests
    /// let chunks = vec![0, 1, 2, 3, 4].into_iter().chunks(2);
    /// let mut chunks_iter = chunks.into_iter();
    ///
    /// let mut chunk_1 = chunks_iter.next().unwrap();
    /// let mut chunk_2 = chunks_iter.next().unwrap();
    ///
    /// chunk_2.next(); // panics because chunk_1 was not consumed
    /// ```
    ///
    /// Similarly, not _fully_ consuming the chunks fails:
    ///
    /// ```compile_fail
    /// # // private items can't be tested with doc tests
    /// let chunks = vec![0, 1, 2, 3, 4].into_iter().chunks(2);
    /// let mut chunks_iter = chunks.into_iter();
    ///
    /// let mut chunk_1 = chunks_iter.next().unwrap();
    /// let _ = chunk_1.next().unwrap();
    /// let mut chunk_2 = chunks_iter.next().unwrap();
    ///
    /// chunk_2.next(); // panics because chunk_1 was not fully consumed
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if size is 0.
    fn chunks(self, size: usize) -> IntoChunks<Self>;
}

impl<I> ChunkableIterator for I
where
    I: Iterator,
{
    fn chunks(self, size: usize) -> IntoChunks<Self> {
        IntoChunks::new(self, size)
    }
}

struct Inner<I>
where
    I: Iterator,
{
    /// The iterator we're chunking
    iter: I,
    /// Size of each chunk. Note that the last chunk may be smaller
    chunk_size: usize,
    /// Number of chunks that have been yielded
    nb_chunks: usize,
    /// Next item from `iter`. By buffering it, we can know when `iter`
    /// is exhausted.
    next: Option<(usize, I::Item)>,
}

impl<I> fmt::Debug for Inner<I>
where
    I: Iterator + fmt::Debug,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("iter", &self.iter)
            .field("chunk_size", &self.chunk_size)
            .field("nb_chunks", &self.nb_chunks)
            .field("next", &self.next)
            .finish()
    }
}

impl<I> Inner<I>
where
    I: ExactSizeIterator,
{
    /// Number of items left in `self.iter`
    fn remaining(&self) -> usize {
        self.next.as_ref().map(|_| 1).unwrap_or(0) + self.iter.len()
    }
}

impl<I> Inner<I>
where
    I: Iterator,
{
    /// Return a new `Inner` with the given iterator and chunk size
    fn new(mut iter: I, chunk_size: usize) -> Self {
        if chunk_size == 0 {
            panic!("invalid chunk size (must be > 0)")
        }
        let next = iter.next().map(|elt| (0, elt));
        Self {
            iter,
            chunk_size,
            nb_chunks: 0,
            next,
        }
    }

    /// Get the `index`-th item from the underlying iterator. See
    /// [`IntoChunks::get`].
    fn get(&mut self, index: usize) -> Option<I::Item> {
        self.next.as_ref()?;

        let current_index = self.next.as_ref().unwrap().0;
        if index < current_index {
            return None;
        }

        if index == current_index {
            let res = Some(self.next.take().unwrap().1);
            // Buffer the next element
            self.next = self.iter.next().map(|elt| (index + 1, elt));
            res
        } else {
            panic!("previous chunks must be consumed");
        }
    }
}

/// A type that can be turned into an `Iterator<Item=Chunk<I>>`.
pub struct IntoChunks<I>
where
    I: Iterator,
{
    /// `inner` is just a mutable `Inner<I>`.
    inner: RefCell<Inner<I>>,
}

impl<I> fmt::Debug for IntoChunks<I>
where
    I: Iterator + fmt::Debug,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoChunks")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<I> IntoChunks<I>
where
    I: Iterator,
{
    /// Return a new `Chunk<I>`
    pub fn new(iter: I, chunk_size: usize) -> Self {
        Self {
            inner: RefCell::new(Inner::new(iter, chunk_size)),
        }
    }

    /// Get the range of the next chunk
    fn next_chunk_range(&self) -> Range<usize> {
        let start = self.inner.borrow().nb_chunks * self.inner.borrow().chunk_size;
        let end = start + self.inner.borrow().chunk_size;
        start..end
    }

    /// Return `true` if the iterator we're chunking is exhausted
    fn exhausted(&self) -> bool {
        self.inner.borrow().next.is_none()
    }

    /// Get the `index`-th item from the underlying iterator. If the
    /// iterator already advanced beyond `index`, `None` is
    /// returned. If the requested `index` hasn't been reached yet,
    /// this method panics. This is to enforce the invariant that all
    /// chunks must be consumed in order.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// # // private items can't be tested with doc tests
    /// let iter = vec![0, 1, 2, 3, 4, 5].into_iter();
    /// let chunk_size = 2;
    /// let chunks = IntoChunks::new(iter, chunk_size);
    /// assert_eq!(chunks.get(0), Some(0));
    /// assert_eq!(chunks.get(1), Some(1));
    /// // calling `get` for an index that have been consumed already
    /// assert_eq!(chunks.get(1), None);
    /// // this panics, because the expected index is `2`
    /// chunks.get(3);
    /// ```
    pub fn get(&self, index: usize) -> Option<I::Item> {
        self.inner.borrow_mut().get(index)
    }
}

impl<I> IntoChunks<I>
where
    I: ExactSizeIterator,
{
    /// Number of items left in the iterator we're chunking
    fn remaining(&self) -> usize {
        self.inner.borrow().remaining()
    }
}

impl<'a, I> IntoIterator for &'a IntoChunks<I>
where
    I: Iterator,
{
    type Item = Chunk<'a, I>;
    type IntoIter = Chunks<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        Chunks { parent: self }
    }
}

/// An iterator that yields chunks
pub struct Chunks<'a, I>
where
    I: Iterator,
{
    parent: &'a IntoChunks<I>,
}

impl<'a, I> fmt::Debug for Chunks<'a, I>
where
    I: Iterator + fmt::Debug,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunks")
            .field("parent", &self.parent)
            .finish()
    }
}

impl<'a, I> Iterator for Chunks<'a, I>
where
    I: Iterator,
{
    type Item = Chunk<'a, I>;

    fn next(&mut self) -> Option<Chunk<'a, I>> {
        if self.parent.exhausted() {
            return None;
        }

        let chunk = Chunk {
            range: self.parent.next_chunk_range(),
            chunks: self.parent,
        };
        self.parent.inner.borrow_mut().nb_chunks += 1;
        Some(chunk)
    }
}

/// A chunk
pub struct Chunk<'a, I>
where
    I: Iterator,
{
    range: Range<usize>,
    chunks: &'a IntoChunks<I>,
}

impl<'a, I> fmt::Debug for Chunk<'a, I>
where
    I: Iterator + fmt::Debug,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("range", &self.range)
            .field("chunks", &self.chunks)
            .finish()
    }
}

impl<'a, I> Iterator for Chunk<'a, I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.start >= self.range.end {
            return None;
        }
        match self.chunks.get(self.range.start) {
            Some(elt) => {
                self.range.start += 1;
                Some(elt)
            }
            None => {
                self.range.start = self.range.end;
                None
            }
        }
    }
}

impl<'a, I> ExactSizeIterator for Chunk<'a, I>
where
    I: Iterator + ExactSizeIterator,
{
    fn len(&self) -> usize {
        cmp::min(self.chunks.remaining(), self.range.end - self.range.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_chunks_1() {
        let iter = vec![0, 1, 2].into_iter();
        let chunks = IntoChunks::new(iter, 1);
        let mut chunks_iter = chunks.into_iter();
        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 0);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 1);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 2);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        assert!(chunks_iter.next().is_none());
    }

    #[test]
    fn full_chunks_2() {
        let iter = vec![0, 1, 2, 3, 4, 5].into_iter();
        let chunks = IntoChunks::new(iter, 2);
        let mut chunks_iter = chunks.into_iter();

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c.next().unwrap(), 0);
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 1);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c.next().unwrap(), 2);
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 3);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c.next().unwrap(), 4);
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 5);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        assert!(chunks_iter.next().is_none());
    }

    #[test]
    fn partial_chunk() {
        let iter = vec![0, 1, 2].into_iter();
        let chunks = IntoChunks::new(iter, 2);
        let mut chunks_iter = chunks.into_iter();

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c.next().unwrap(), 0);
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 1);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());

        let mut c = chunks_iter.next().unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c.next().unwrap(), 2);
        assert_eq!(c.len(), 0);
        assert!(c.next().is_none());
    }

    #[test]
    #[should_panic(expected = "previous chunks must be consumed")]
    fn chunks_consumed_out_of_order() {
        let iter = vec![0, 1, 2, 3, 4, 5].into_iter();
        let chunks = IntoChunks::new(iter, 2);
        let mut chunks_iter = chunks.into_iter();

        let mut c1 = chunks_iter.next().unwrap();
        assert_eq!(c1.next().unwrap(), 0);
        assert_eq!(c1.next().unwrap(), 1);
        assert!(c1.next().is_none());

        let _c2 = chunks_iter.next().unwrap();
        let mut c3 = chunks_iter.next().unwrap();

        assert_eq!(c3.next().unwrap(), 4);
    }

    // This test case illustrates a weird behavior of our iterator:
    // everything being lazy, we can create chunks that start *beyond*
    // what our main iterator can provide in theory. Attempting to
    // consume such iterators should panic
    #[test]
    #[should_panic(expected = "previous chunks must be consumed")]
    fn weird() {
        let iter = vec![0, 1, 2].into_iter();
        let chunks = IntoChunks::new(iter, 1);
        let mut chunks_iter = chunks.into_iter();

        let mut c1 = chunks_iter.next().unwrap();
        let mut c2 = chunks_iter.next().unwrap();
        let mut c3 = chunks_iter.next().unwrap();
        // This chunks starts at index 3, which we don't even have
        let mut c4 = chunks_iter.next().unwrap();
        assert!(c4.next().is_none());
        assert!(c1.next().is_none());
        assert!(c2.next().is_none());
        assert!(c3.next().is_none());
    }
}
