# Union Futures

When writing asynchronous code with the wonderful [futures](https://github.com/alexcrichton/futures-rs) library, it is sometimes
necessary to write code that branches. For example, your code may have an immediate answer _some_ of the time, in which case
you want to return a future that resolves immediately, or it may need to call the database if an immediate answer is not available.

As discussed in [the tutorial](https://github.com/alexcrichton/futures-rs/blob/master/TUTORIAL.md#trait-objects), there are multiple
options for this scenario, with the most popular being to create a `BoxedFuture`. There are downsides to this approach, such as runtime
allocation of the trait object. The `BoxedFuture` approach is popular because it is highly ergonomic (the future trait has a method `.boxed()`
that almost encourages the approach).

However, in high performance scenarios or when exposing a library, explicit state machines are the preferred approach to building futures. This
library makes it easy to write ergonomic code that is also efficient.

```rust
#[macro_use]
extern crate union_future;
extern crate futures;

use futures::*;
use futures::future::*;

// Macro will create the enum and necessary trait implementations
// for the QueryFuture. This enum will have 2 variants: Cached and Db.
union_future!(QueryFuture<u64, DbError>,
      Cached => FutureResult<u64, DbError>,
      Db => DbQueryFuture<u64>);

// Example code that branches, using the future created by the macro
pub fn query(db: &Db, key: &str) -> QueryFuture {
    if let Some(cached_val) = check_local_cache(key) {
        QueryFuture::Cached(ok(cached_val))
    } else {
        query_db(db, key).into()
    }
}

fn check_local_cache(key: &str) -> Option<u64> {
    // ...
}

fn query_db(db: &Db, key: &str) -> DbQueryFuture<u64> {
    // ...
}
```

## Installation

First, add this to your `Cargo.toml`:

```toml
[dependencies]
union-future = "0.1"
futures = "0.1"
```
