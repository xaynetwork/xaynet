//! Message utilities.
//!
//! See the [message module] documentation since this is a private module anyways.
//!
//! [message module]: ../index.html

mod chunkable_iterator;
pub use chunkable_iterator::{Chunk, ChunkableIterator, Chunks, IntoChunks};

use std::ops::Range;

/// Creates a range from `start` to `start + length`.
pub(crate) const fn range(start: usize, length: usize) -> Range<usize> {
    start..(start + length)
}
