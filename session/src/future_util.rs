#![allow(dead_code)]

use http::{Response, StatusCode};
use pin_project_lite::pin_project;
use std::task::{Context, Poll};
use std::{future::Future, pin::Pin};

pin_project! {
    pub struct ResponseFuture<F, B> {
        #[pin]
        kind: ResponseFutureKind<F, B>,
    }
}

impl<F, B: Default> ResponseFuture<F, B> {
    pub fn future(future: F) -> Self {
        Self {
            kind: ResponseFutureKind::Future { future },
        }
    }

    pub fn invalid_auth() -> Self {
        let mut res = Response::new(B::default());
        *res.status_mut() = StatusCode::UNAUTHORIZED;
        Self {
            kind: ResponseFutureKind::Error { response: Some(res) },
        }
    }
}

pin_project! {
    #[project = KindProj]
    pub enum ResponseFutureKind<F, B> {
        Future {
            #[pin]
            future: F,
        },
        Error {
            response: Option<Response<B>>,
        },
    }
}

impl<F, B, E> Future for ResponseFuture<F, B>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().kind.project() {
            KindProj::Future { future } => future.poll(cx),
            KindProj::Error { response } => {
                let response = response.take().unwrap();
                Poll::Ready(Ok(response))
            }
        }
    }
}
