use actix_web::{http::StatusCode, HttpResponse, ResponseError};

pub mod commit;
pub mod project;
pub mod unit;

#[derive(Debug)]
enum ApiError {
    ServerError,
    Unauthorized,
    BadRequest,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::ServerError => write!(f, "The server has encountered an internal error"),
            ApiError::Unauthorized => write!(
                f,
                "You don't have permission to access the requested resource"
            ),
            ApiError::BadRequest => write!(f, "Bad request"),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unauthorized => StatusCode::FORBIDDEN,
            Self::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain")
            .body(self.to_string())
    }
}
