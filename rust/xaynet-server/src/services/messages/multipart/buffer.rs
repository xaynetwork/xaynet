use std::{
    collections::btree_map::{BTreeMap, IntoIter as BTreeMapIter},
    iter::{ExactSizeIterator, Iterator},
    vec::IntoIter as VecIter,
};

/// A data structure for reading a multipart message
pub struct MultipartMessageBuffer {
    /// message chunks that haven't been read yet
    remaining_chunks: BTreeMapIter<u16, Vec<u8>>,
    /// chunk being read
    current_chunk: Option<VecIter<u8>>,
    /// total length of the buffer
    initial_length: usize,
    /// number of bytes that have been read
    consumed: usize,
}

impl From<BTreeMap<u16, Vec<u8>>> for MultipartMessageBuffer {
    fn from(map: BTreeMap<u16, Vec<u8>>) -> Self {
        let initial_length = map.values().fold(0, |acc, chunk| acc + chunk.len());
        Self {
            remaining_chunks: map.into_iter(),
            current_chunk: None,
            initial_length,
            consumed: 0,
        }
    }
}

// Note that this Iterator implementation could be optimized. We
// currently increment a counter for every byte consumed, but we could
// exploits the fact that IterVec implements ExactSizeIterator avoid
// that.
impl Iterator for MultipartMessageBuffer {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_chunk.is_none() {
            let (_, chunk) = self.remaining_chunks.next()?;
            self.current_chunk = Some(chunk.into_iter());
            return self.next();
        }

        // Per `if` above, `self.current_chunk` is not None
        match self.current_chunk.as_mut().unwrap().next() {
            Some(b) => {
                self.consumed += 1;
                Some(b)
            }
            None => {
                self.current_chunk = None;
                self.next()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let lower_bound = self.initial_length - self.consumed;
        let upper_bound = self.initial_length;
        (lower_bound, Some(upper_bound))
    }
}

impl ExactSizeIterator for MultipartMessageBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut map: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
        map.insert(1, vec![0, 1, 2]);
        map.insert(2, vec![3]);
        map.insert(3, vec![4, 5]);

        let mut iter = MultipartMessageBuffer::from(map);
        assert_eq!(iter.consumed, 0);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_none());

        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.consumed, 1);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_some());

        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.consumed, 2);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_some());

        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.consumed, 3);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_some());

        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.consumed, 4);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_some());

        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.consumed, 5);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_some());

        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.consumed, 6);
        assert_eq!(iter.initial_length, 6);
        assert!(iter.current_chunk.is_some());
    }
}
