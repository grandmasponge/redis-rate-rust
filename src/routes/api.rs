use axum::{http::StatusCode, response::IntoResponse};

pub struct Res {
    status: StatusCode,
    body: String,
}

pub async fn random_api_route() -> impl IntoResponse {
    let response = Res::random_response();
    (response.status, response.body).into_response()
}

impl Res {
    pub fn new(status: StatusCode, body: String) -> Self {
        Res { status, body }
    }

    pub fn random_response() -> Self {
        let data = rand::random::<u8>();
        Res::new(StatusCode::OK, data.to_string())
    }
}
