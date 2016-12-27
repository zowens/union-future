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
//! Additionally, the underlying branch state machines can have *different* Item types that are
//! mapped to the `union_future` future's Item type via the `From` trait.
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
//! union-future = "0.1"
//! futures = "0.1"
//! ```
//! ## Examples
//!
//! The basic usage of the macro uses the same Item type from different underlying
//! futures. For example, if you have a locally cached version otherwise the code
//! will query the database:
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
//! // Macro will create the enum and necessary trait implementations
//! // for the QueryFuture. This enum will have 2 variants: Cached and Db.
//! union_future!(QueryFuture<u64, DbError>,
//!       Cached => FutureResult<u64, DbError>,
//!       Db => DbQueryFuture<u64>);
//!
//! // Example code that branches, using the future created by the macro
//! pub fn query(db: &Db, key: &str) -> QueryFuture {
//!     // this example shows multiple ways the QueryFuture can be constructed:
//!     // either by the explicit enum variant or by using the From/Into traits
//!     if let Some(cached_val) = check_local_cache(key) {
//!         QueryFuture::Cached(ok(cached_val))
//!     } else {
//!         query_db(db, key).into()
//!     }
//! }
//!
//! fn check_local_cache(key: &str) -> Option<u64> {
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
//! # }
//! # pub struct Db {
//! # }
//! # pub type DbQueryFuture<T> = Empty<T, DbError>;
//! # fn main() {}
//! ```
//!
//! You could, however, have a future that can be mapped into the future result type
//! with the `From` trait:
//!
//! ```
//! # #[macro_use]
//! # extern crate union_future;
//! # extern crate futures;
//! # use futures::*;
//! # use futures::future::*;
//! pub enum RedisValue {
//!     Null,
//!     Integer(i64),
//!     Bulk(String),
//! }
//!
//! // Implementing the From trait allows the underlying futures to expose
//! // different Item types transparently
//!
//! impl From<()> for RedisValue {
//!     fn from(_: ()) -> RedisValue {
//!         RedisValue::Null
//!     }
//! }
//!
//! impl From<i64> for RedisValue {
//!     fn from(other: i64) -> RedisValue {
//!         RedisValue::Integer(other)
//!     }
//! }
//!
//! impl From<String> for RedisValue {
//!     fn from(other: String) -> RedisValue {
//!         RedisValue::Bulk(other)
//!     }
//! }
//!
//! union_future!(RedisValueFuture<RedisValue, DbError>,
//!       Pong => FutureResult<(), DbError>,
//!       IntegerQuery => DbQueryFuture<i64>,
//!       StringQuery => DbQueryFuture<String>);
//!
//! # pub struct DbError {
//! # }
//! # pub struct MemDb {
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
    ($name:ident<$item:ty, $err:ty>, $($n:ident => $ft:ty),*) => (
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
    );
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
}
