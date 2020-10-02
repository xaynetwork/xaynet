#![allow(dead_code)]

use std::cmp;

/// Default chunk size, for [`Chunker`]
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

/// A struct that yields chunks of the given data.
pub struct Chunker<'a, T: AsRef<[u8]>> {
    data: &'a T,
    max_chunk_size: usize,
}

impl<'a, T> Chunker<'a, T>
where
    T: AsRef<[u8]>,
{
    /// Create a new [`Chunker`] that yields chunks of `T` of size
    /// `max_chunk_size`. If `max_chunk_size` is `0`, then the max
    /// chunk size will be set to [`DEFAULT_CHUNK_SIZE`].
    pub fn new(data: &'a T, max_chunk_size: usize) -> Self {
        let max_chunk_size = if max_chunk_size == 0 {
            DEFAULT_CHUNK_SIZE
        } else {
            max_chunk_size
        };
        Self {
            data,
            max_chunk_size,
        }
    }

    /// Get the total number of chunks
    pub fn nb_chunks(&self) -> usize {
        let data_len = self.data.as_ref().len();
        ceiling_div(data_len, self.max_chunk_size)
    }

    /// Get the chunk with the given ID.
    ///
    /// # Panics
    ///
    /// This method panics if the given `id` is bigger than `self.nb_chunks()`.
    pub fn get_chunk(&self, id: usize) -> &'a [u8] {
        if id >= self.nb_chunks() {
            panic!("no chunk with ID {}", id);
        }
        let start = id * self.max_chunk_size;
        let end = cmp::min(start + self.max_chunk_size, self.data.as_ref().len());
        let range = start..end;
        &self.data.as_ref()[range]
    }
}

/// A helper that performs division with ceil.
///
/// # Panic
///
/// This function panic if `d` is 0.
fn ceiling_div(n: usize, d: usize) -> usize {
    (n + d - 1) / d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "no chunk with ID 0")]
    fn test_0() {
        let data = vec![];
        let chunker = Chunker::new(&data, 0);
        assert_eq!(chunker.nb_chunks(), 0);
        chunker.get_chunk(0);
    }

    #[test]
    #[should_panic(expected = "no chunk with ID 5")]
    fn test_1() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunker = Chunker::new(&data, 2);
        assert_eq!(chunker.nb_chunks(), 5);
        assert_eq!(chunker.get_chunk(0), &[0, 1]);
        assert_eq!(chunker.get_chunk(1), &[2, 3]);
        assert_eq!(chunker.get_chunk(2), &[4, 5]);
        assert_eq!(chunker.get_chunk(3), &[6, 7]);
        assert_eq!(chunker.get_chunk(4), &[8, 9]);
        chunker.get_chunk(5);
    }

    #[test]
    #[should_panic(expected = "no chunk with ID 5")]
    fn test_2() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
        let chunker = Chunker::new(&data, 2);
        assert_eq!(chunker.nb_chunks(), 5);
        assert_eq!(chunker.get_chunk(0), &[0, 1]);
        assert_eq!(chunker.get_chunk(1), &[2, 3]);
        assert_eq!(chunker.get_chunk(2), &[4, 5]);
        assert_eq!(chunker.get_chunk(3), &[6, 7]);
        assert_eq!(chunker.get_chunk(4), &[8]);
        chunker.get_chunk(5);
    }

    #[test]
    #[should_panic(expected = "no chunk with ID 4")]
    fn test_3() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunker = Chunker::new(&data, 3);
        assert_eq!(chunker.nb_chunks(), 4);
        assert_eq!(chunker.get_chunk(0), &[0, 1, 2]);
        assert_eq!(chunker.get_chunk(1), &[3, 4, 5]);
        assert_eq!(chunker.get_chunk(2), &[6, 7, 8]);
        assert_eq!(chunker.get_chunk(3), &[9]);
        chunker.get_chunk(4);
    }

    #[test]
    #[should_panic(expected = "no chunk with ID 1")]
    fn test_4() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunker = Chunker::new(&data, 10);
        assert_eq!(chunker.nb_chunks(), 1);
        assert_eq!(chunker.get_chunk(0), data.as_slice());
        chunker.get_chunk(1);
    }

    #[test]
    #[should_panic(expected = "no chunk with ID 1")]
    fn test_5() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunker = Chunker::new(&data, 0);
        assert_eq!(chunker.max_chunk_size, DEFAULT_CHUNK_SIZE);
        assert_eq!(chunker.nb_chunks(), 1);
        assert_eq!(chunker.get_chunk(0), data.as_slice());
        chunker.get_chunk(1);
    }
}
