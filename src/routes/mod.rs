use self::middleware::rate::RedisLayer;
use axum::routing::get;
use axum::Router;

pub mod api;
pub mod middleware;

pub async fn api_router() -> Router {
    Router::new()
        .route("/data", get(api::random_api_route))
        .layer(RedisLayer)
}
