use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use deadpool_redis::{
    redis::{cmd, FromRedisValue},
    Config, Pool, Runtime,
};
use hyper::StatusCode;

use futures_util::future::BoxFuture;
use std::{
    net::SocketAddr,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};

use crate::routes::api::res;

#[derive(Clone)]
pub struct RedisLayer;

impl<S> Layer<S> for RedisLayer {
    type Service = MyMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct MyMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for MyMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let ip = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .expect("ConnectInfo<SocketAddr> should be set by the server")
            .ip();

        let future = self.inner.call(request);
        Box::pin(async move {
            let mut con = Config::from_url("redis://localhost:6379");
            let pool: Pool = con.create_pool(Some(Runtime::Tokio1)).unwrap();

            let arc = Arc::new(pool);

            let res: Option<i32> = query_user(ip.to_string(), arc.clone()).await;

            match res {
                Some(val) => {
                    println!("User has made {} requests", val);
                }
                None => {
                    let res: Result<i32, http::Response<Body>> =
                        create_user(ip.to_string(), arc.clone()).await;
                    match res {
                        Ok(_) => {
                            println!("created");
                        }
                        Err(e) => {
                            return Ok(e);
                        }
                    }
                }
            };

            let response: Response = future.await?;
            Ok(response)
        })
    }
}

async fn incr<T, Auth>(ip: Auth, redis_conn: Arc<Pool>) -> Result<T, Response>
where
    Auth: ToString,
    T: FromRedisValue,
{
    let mut conn = redis_conn.get().await.unwrap();
    let res = cmd("INCR").arg(ip.to_string()).query_async(&mut conn).await;

    match res {
        Ok(val) => {
            return Ok(val);
        }
        Err(_) => {
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
            return Err(response);
        }
    }
}

async fn create_user<T, Auth>(ip: Auth, redis_conn: Arc<Pool>) -> Result<T, Response>
where
    Auth: ToString,
    T: FromRedisValue,
{
    let mut conn = redis_conn.get().await.unwrap();

    let res = cmd("SET")
        .arg(ip.to_string())
        .arg(0)
        .query_async(&mut conn)
        .await;

    match res {
        Ok(val) => {
            return Ok(val);
        }
        Err(_) => {
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
            return Err(response);
        }
    }
}

async fn query_user<T, Auth>(ip: Auth, redis_conn: Arc<Pool>) -> Option<T>
where
    Auth: ToString,
    T: FromRedisValue,
{
    let mut conn = redis_conn.get().await.unwrap();
    let res = cmd("GET").arg(ip.to_string()).query_async(&mut conn).await;

    match res {
        Ok(val) => {
            return Some(val);
        }
        Err(_) => None,
    }
}
