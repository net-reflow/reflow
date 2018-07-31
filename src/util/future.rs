use futures::{Future, Poll};

/// Combines two different futures yielding the same item and error
/// types into a single type.
#[derive(Debug)]
pub enum Either3<A, B, C> {
    /// First branch of the type
    A(A),
    /// Second branch of the type
    B(B),
    C(C),
}

impl<T, A, B, C> Either3<(T, A), (T, B), (T, C)> {
    /// Splits out the homogeneous type from an either of tuples.
    ///
    /// This method is typically useful when combined with the `Future::select2`
    /// combinator.
    pub fn split(self) -> (T, Either3<A, B, C>) {
        match self {
            Either3::A((a, b)) => (a, Either3::A(b)),
            Either3::B((a, b)) => (a, Either3::B(b)),
            Either3::C((a, b)) => (a, Either3::C(b)),
        }
    }
}

impl<A, B, C> Future for Either3<A, B, C>
    where A: Future,
          B: Future<Item = A::Item, Error = A::Error>,
          C: Future<Item = A::Item, Error = A::Error>
{
    type Item = A::Item;
    type Error = A::Error;

    fn poll(&mut self) -> Poll<A::Item, A::Error> {
        match *self {
            Either3::A(ref mut a) => a.poll(),
            Either3::B(ref mut b) => b.poll(),
            Either3::C(ref mut c) => c.poll(),
        }
    }
}
