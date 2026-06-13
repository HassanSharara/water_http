#![cfg(feature = "lazy_response")]

use std::future::Future;
use std::pin::Pin;
use smallbox::SmallBox;
use crate::server::HttpContext;
use smallbox::space::S32 as SmallBoxSize;

#[cfg(feature = "use_tokio_send")]
pub(crate) type BoxFutureSend<'a, T> = Pin<SmallBox<dyn Future<Output = T> + Send + 'a, SmallBoxSize>>;
#[cfg(not(feature = "use_tokio_send"))]
pub(crate) type BoxFuture<'a, T> = Pin<SmallBox<dyn Future<Output = T> + 'a, SmallBoxSize>>;

#[cfg(all(feature = "thread_shared_struct", feature = "use_tokio_send"))]
pub(crate) type InterceptorCallback<
    H,
    SHARED,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, SHARED, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFutureSend<'a, ()>;

#[cfg(all(feature = "thread_shared_struct", not(feature = "use_tokio_send")))]
pub(crate) type InterceptorCallback<
    H,
    SHARED,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, SHARED, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFuture<'a, ()>;

#[cfg(all(not(feature = "thread_shared_struct"), feature = "use_tokio_send"))]
pub(crate) type InterceptorCallback<
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFutureSend<'a, ()>;

#[cfg(all(not(feature = "thread_shared_struct"), not(feature = "use_tokio_send")))]
pub(crate) type InterceptorCallback<
    H,
    const HEADER_SIZE: usize,
    const QUERY_SIZE: usize,
> = for<'a, 'context> fn(
    &'a mut HttpContext<'context, H, HEADER_SIZE, QUERY_SIZE>,
) -> BoxFuture<'a, ()>;