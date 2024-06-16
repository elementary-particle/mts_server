use axum::body::Body;
use axum::extract::{FromRef, Path, Request, State};
use axum::response::{IntoResponse, Response};
use axum::{routing, Router};

use crate::auth::{AuthRwLock, Claim};
use crate::LmApiClient;

enum LmApiError {
    BadURL,
    ServiceUnavailable,
}

impl IntoResponse for LmApiError {
    fn into_response(self) -> Response {
        use axum::http::StatusCode;
        match self {
            LmApiError::BadURL => {
                (StatusCode::BAD_REQUEST, "The requested URL is invalid").into_response()
            }
            LmApiError::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "The chat service is not available",
            )
                .into_response(),
        }
    }
}

pub fn build_router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    AuthRwLock: FromRef<S>,
    LmApiClient: FromRef<S>,
{
    Router::new().route("/*path", routing::post(openai_proxy))
}

async fn openai_proxy(
    _: Claim,
    State(chat_api): State<LmApiClient>,
    Path(path): Path<String>,
    mut request: Request<Body>,
) -> Result<Response, LmApiError> {
    let base_path = chat_api.uri.path_and_query().unwrap().as_str();
    let api_path = if base_path.ends_with("/") {
        base_path.to_owned()
    } else {
        base_path.to_owned() + "/"
    } + path.as_ref();
    *request.uri_mut() = hyper::Uri::builder()
        .scheme(chat_api.uri.scheme().unwrap().clone())
        .authority(chat_api.uri.authority().unwrap().clone())
        .path_and_query(api_path)
        .build()
        .map_err(|_| LmApiError::BadURL)?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", chat_api.key).parse().unwrap(),
    );
    Ok(chat_api
        .client
        .request(request)
        .await
        .map_err(|_| LmApiError::ServiceUnavailable)?
        .into_response())
}
