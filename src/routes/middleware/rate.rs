use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    response::Response,
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

#[derive(Clone, Copy)]
pub enum AuthMethod {
    Basic,
    Bearer,
}

#[derive(Clone)]
pub struct RedisLayer {
    ttl: i32,
    method: AuthMethod,
    req_limit: i32,
}

impl RedisLayer {
    pub fn new(ttl: i32, method: AuthMethod, req_limit: i32) -> Self {
        Self { ttl, method, req_limit}
    }
}

impl<S> Layer<S> for RedisLayer {
    type Service = RateLimiter<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimiter::new(self.ttl, self.method,self.req_limit, inner)
    }
}

#[derive(Clone)]
pub struct RateLimiter<S> {
    ttl: i32,
    method: AuthMethod,
    limit: i32,
    inner: S,
}

impl<S> RateLimiter<S> {
    fn new(ttl: i32, method: AuthMethod, limit:i32,  inner: S) -> Self {
        Self {ttl, method, limit, inner}
    }

}

impl<S> Service<Request<Body>> for RateLimiter<S>
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
        let token = match self.method {
            AuthMethod::Basic => {
                request
                    .extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .expect("ConnectInfo should be set")
                    .ip()
                    .to_string()
            }
            AuthMethod::Bearer => {
                request
                    .headers()
                    .get("Authorization")
                    .expect("Authorization header should be set")
                    .to_str()
                    .expect("Authorization header should be a valid string")
                    .to_string()
            }
        };

        println!("req from: {}", token);

        let future = self.inner.call(request);
        Box::pin(async move {
            let con = Config::from_url("redis://localhost:6379");
            let pool: Pool = con.create_pool(Some(Runtime::Tokio1)).unwrap();

            let arc = Arc::new(pool);

            let res: Option<i32> = query_user(token.to_string(), arc.clone()).await;

            match res {
                Some(val) => {
                    let _: i32 = incr(token.to_string(), arc.clone()).await
                    .expect("Failed to increment");
                    println!("User has made {} requests", val);
                    if val > 6 {
                        let response = Response::builder()
                            .status(StatusCode::TOO_MANY_REQUESTS)
                            .body(Body::empty())
                            .unwrap();
                        return Ok(response);
                    }

                }
                None => {
                    let res: Result<i32, http::Response<Body>> =
                        create_user(token.to_string(), arc.clone()).await;
                       let _: i32 = set_ttl(token.to_string(), arc.clone(), 60).await
                        .expect("Failed to set ttl");

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


async fn set_ttl<T, Auth>(ip: Auth , redis_conn: Arc<Pool>, ttl: i32) -> Result<T, Response>
where
    Auth: ToString,
    T: FromRedisValue,
 {
    let mut conn = redis_conn.get().await.unwrap();
    let res = cmd("EXPIRE")
        .arg(ip.to_string())
        .arg(ttl)
        .query_async(&mut conn)
        .await;

    match res {
        Ok(res) => {
            return Ok(res);
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
