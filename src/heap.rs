use std::{cmp, collections::BinaryHeap};

pub(crate) enum Heap<T: Ord> {
    Min(BinaryHeap<cmp::Reverse<T>>),
    Max(BinaryHeap<T>),
}

impl<T: Ord> Heap<T> {
    pub(crate) fn min() -> Self {
        Heap::Min(BinaryHeap::new())
    }

    pub(crate) fn max() -> Self {
        Heap::Max(BinaryHeap::new())
    }

    #[inline]
    pub(crate) fn push(&mut self, value: T) {
        match self {
            Heap::Min(heap) => heap.push(cmp::Reverse(value)),
            Heap::Max(heap) => heap.push(value),
        }
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> Option<T> {
        match self {
            Heap::Min(heap) => heap.pop().map(|rev| rev.0),
            Heap::Max(heap) => heap.pop(),
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        match self {
            Heap::Min(heap) => heap.len(),
            Heap::Max(heap) => heap.len(),
        }
    }

    #[inline]
    pub(crate) fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self {
            Heap::Min(heap) => Box::new(heap.iter().map(|rev| &rev.0)),
            Heap::Max(heap) => Box::new(heap.iter()),
        }
    }

    #[inline]
    pub(crate) fn into_iter<'a>(self) -> Box<dyn Iterator<Item = T> + 'a>
    where
        T: 'a,
    {
        match self {
            Heap::Min(heap) => Box::new(heap.into_iter().map(|rev| rev.0)),
            Heap::Max(heap) => Box::new(heap.into_iter()),
        }
    }
}
