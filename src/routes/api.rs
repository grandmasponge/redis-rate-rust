use axum::{http::StatusCode, response::IntoResponse};

pub struct res {
    status: StatusCode,
    body: String,
}

pub async fn random_api_route() -> impl IntoResponse {
    let response = res::random_response();
    (response.status, response.body).into_response()
}

impl res {
    pub fn new(status: StatusCode, body: String) -> Self {
        res { status, body }
    }

    pub fn random_response() -> Self {
        let data = rand::random::<u8>();
        res::new(StatusCode::OK, data.to_string())
    }
}
