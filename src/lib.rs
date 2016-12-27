//! Explicit and efficient future that results from a branched future.
//!
//! The `union_future` macro creates a future derived from a branch of different underlying
//! futures. The macro can prevent unnecessary boxing of futures when the code can branch
//! into multiple future types.
//!
//! The macro works by exposing an enum that implements the Future trait, where the underlying
//! future drives the polling behavior. The variants of the enum can have different underlying
//! state machines (types that implement the `Future` trait).
//!
//! Additionally, the underlying branch state machines can return *different* result types that are
//! mapped to the common result type via the `From` trait.
//!
//! Also, as an added bonus, the macro will derive the `From` trait for the underlying state
//! machines in order to make the branched code clean.
//!
//! ## Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! union_future = "0.1"
//! futures = "0.1"
//! ```
//! ## Examples
//!
//! ```
//! #[macro_use]
//! extern crate union_future;
//! extern crate futures;
//!
//! use futures::*;
//! use futures::future::*;
//!
//!
//! // Invocation of the macro, which creates the enum and necessary trait impls
//! union_future!(pub QueryFuture<u64, DbError>,
//!       Cached => FutureResult<u64, DbError>,
//!       Db => DbQueryFuture<u64>);
//!
//! // Example code that branches, using the future created by the macro
//! pub fn query(db: &Db, key: &str) -> QueryFuture {
//!     if let Some(cached_val) = check_cache(key) {
//!         ok(cached_val).into()
//!     } else {
//!         query_db(db, key).into()
//!     }
//! }
//!
//! fn check_cache(key: &str) -> Option<u64> {
//!     // ...
//!     # panic!("Unimplemented")
//! }
//!
//! fn query_db(db: &Db, key: &str) -> DbQueryFuture<u64> {
//!     // ...
//!     # panic!("Unimplemented")
//! }
//!
//! # pub struct DbError {
//! #     // ...
//! # }
//! # pub struct Db {
//! # }
//! # pub type DbQueryFuture<T> = Empty<T, DbError>;
//! # fn main() {}
//! ```

#[macro_use]
extern crate futures;

/// A macro to create a future that has branched from multiple underlying futures of distinct
/// types.
#[macro_export]
macro_rules! union_future {
    ($name:ident<$item:ty, $err:ty>, $($n: tt => $ft: ty),*) => (
        enum $name {
            $( $n($ft) ),*
        }

        impl futures::Future for $name {
            type Item = $item;
            type Error = $err;

            fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
                match *self {
                    $(
                        $name::$n(ref mut f) => {
                            match f.poll() {
                                Ok(futures::Async::Ready(t)) => Ok(futures::Async::Ready(From::from(t))),
                                Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
                                Err(e) => Err(From::from(e)),
                            }
                        }
                        ),*
                }
            }
        }

        $(
            impl From<$ft> for $name {
                fn from(other: $ft) -> $name {
                    $name::$n(other)
                }
            })*
    );
    (pub $name:ident<$item:ty, $err:ty>, $($n: tt => $ft: ty),*) => (
        pub enum $name {
            $( $n($ft) ),*
        }

        impl futures::Future for $name {
            type Item = $item;
            type Error = $err;

            fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
                match *self {
                    $(
                        $name::$n(ref mut f) => {
                            match f.poll() {
                                Ok(futures::Async::Ready(t)) => Ok(futures::Async::Ready(From::from(t))),
                                Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
                                Err(e) => Err(From::from(e)),
                            }
                        }
                        ),*
                }
            }
        }

        $(
            impl From<$ft> for $name {
                fn from(other: $ft) -> $name {
                    $name::$n(other)
                }
            })*

    )
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    extern crate futures;
    use futures::*;
    use futures::future::*;

    #[derive(PartialEq, Debug, Eq)]
    pub enum Error {
        Fail,
        BigFail,
    }

    #[derive(PartialEq, Debug, Eq)]
    pub struct OtherError {
        op: u64
    }

    impl From<OtherError> for Error {
        fn from(_: OtherError) -> Error {
            Error::BigFail
        }
    }

    #[test]
    fn same_types() {
        union_future!(TestFut<u64, Error>,
                Forever => Empty<u64, Error>,
                Immediate => FutureResult<u64, Error>);

        let mut a: TestFut = empty::<u64, Error>().into();
        assert_eq!(Ok(Async::NotReady), a.poll());
        let mut b: TestFut = ok::<u64, Error>(5).into();
        assert_eq!(Ok(Async::Ready(5u64)), b.poll());
    }

    #[test]
    fn different_item_types() {
        union_future!(TestFut<f64, Error>,
                Number => FutureResult<u32, Error>,
                Floating => FutureResult<f32, Error>);

        let mut a: TestFut = ok::<u32, Error>(5u32).into();
        assert_eq!(Ok(Async::Ready(5f64)), a.poll());
        let mut b: TestFut = ok::<f32, Error>(5.25f32).into();
        assert_eq!(Ok(Async::Ready(5.25f64)), b.poll());
    }

    #[test]
    fn different_err_types() {
        union_future!(TestFut<f64, Error>,
                Number => FutureResult<u32, Error>,
                Floating => FutureResult<f32, OtherError>);

        let mut a: TestFut = ok::<u32, Error>(5u32).into();
        assert_eq!(Ok(Async::Ready(5f64)), a.poll());
        let mut b: TestFut = ok::<f32, OtherError>(5.25f32).into();
        assert_eq!(Ok(Async::Ready(5.25f64)), b.poll());
    }

    #[test]
    fn pub_same_types() {
        union_future!(pub TestFut<u64, Error>,
                Forever => Empty<u64, Error>,
                Immediate => FutureResult<u64, Error>);

        let mut a: TestFut = empty::<u64, Error>().into();
        assert_eq!(Ok(Async::NotReady), a.poll());
        let mut b: TestFut = ok::<u64, Error>(5).into();
        assert_eq!(Ok(Async::Ready(5u64)), b.poll());
    }

    #[test]
    fn pub_different_item_types() {
        union_future!(pub TestFut<f64, Error>,
                Number => FutureResult<u32, Error>,
                Floating => FutureResult<f32, Error>);

        let mut a: TestFut = ok::<u32, Error>(5u32).into();
        assert_eq!(Ok(Async::Ready(5f64)), a.poll());
        let mut b: TestFut = ok::<f32, Error>(5.25f32).into();
        assert_eq!(Ok(Async::Ready(5.25f64)), b.poll());
    }

    #[test]
    fn pub_different_err_types() {
        union_future!(pub TestFut<f64, Error>,
                Number => FutureResult<u32, Error>,
                Floating => FutureResult<f32, OtherError>);

        let mut a: TestFut = ok::<u32, Error>(5u32).into();
        assert_eq!(Ok(Async::Ready(5f64)), a.poll());
        let mut b: TestFut = ok::<f32, OtherError>(5.25f32).into();
        assert_eq!(Ok(Async::Ready(5.25f64)), b.poll());
    }
}

