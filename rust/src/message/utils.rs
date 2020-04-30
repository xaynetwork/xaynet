use std::ops::Range;

pub(crate) const fn range(start: usize, length: usize) -> Range<usize> {
    start..(start + length)
}
