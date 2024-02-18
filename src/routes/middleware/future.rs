use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::BoxError;

#[pin_project]
pub struct ExampleFuture<F> {
    #[pin]
    response_future: F,
}


impl<F> ExampleFuture<F> {
    #[warn(dead_code)]
    pub fn new(response_future: F) -> Self {
        Self { response_future }
    }
}

impl<F, T, E> Future for ExampleFuture<F>
where
    F: Future<Output = Result<T, E>>,
    E: Into<BoxError>,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        this.response_future.poll(cx)
    }
}
