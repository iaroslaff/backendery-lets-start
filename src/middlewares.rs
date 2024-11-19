use axum::{
    extract::Request,
    http::{HeaderValue, Request as HttpRequest},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ulid::Ulid;

type RequestIdResult<T> = Result<T, Response>;

pub(super) async fn add_request_id(rq: Request, next: Next) -> RequestIdResult<impl IntoResponse> {
    let (mut parts, body) = rq.into_parts();

    // Adding a header if there is none
    parts
        .headers
        .entry("x-request-id")
        .or_insert_with(|| HeaderValue::from_str(&Ulid::new().to_string()).unwrap());

    // Create a new request with modified headers
    let new_request = HttpRequest::from_parts(parts, body);

    Ok(next.run(new_request).await)
}
