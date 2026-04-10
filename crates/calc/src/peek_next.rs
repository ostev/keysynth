use core::iter::FusedIterator;

pub struct PeekableNext<I: Iterator> {
    iter: I,

    peeked: Option<I::Item>,
    peeked_next: Option<I::Item>,
}

impl<I: Iterator> PeekableNext<I> {
    pub fn new(mut iter: I) -> PeekableNext<I> {
        let peeked = iter.next();
        let peeked_next = iter.next();

        PeekableNext {
            iter,
            peeked,
            peeked_next,
        }
    }

    #[inline]
    pub fn peek(&self) -> Option<&I::Item> {
        self.peeked.as_ref()
    }

    #[inline]
    pub fn peek_next(&self) -> Option<&I::Item> {
        self.peeked_next.as_ref()
    }
}

impl<I: Iterator> Iterator for PeekableNext<I> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.peeked.take();

        self.peeked = self.peeked_next.take();
        self.peeked_next = self.iter.next();

        next
    }
}

/// Trait extension that provides [`peekable_next`] for [`Iterator`]s
pub trait IteratorExt {
    /// Creates an iterator that can peek the next element and the one
    /// after that.
    fn peekable_next(self) -> PeekableNext<Self>
    where
        Self: Sized + Iterator;
}
impl<I: Iterator> IteratorExt for I {
    fn peekable_next(self) -> PeekableNext<Self> {
        PeekableNext::new(self)
    }
}
