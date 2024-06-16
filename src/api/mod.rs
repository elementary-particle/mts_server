mod lm;
mod commit;
mod project;
mod unit;

use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Router;

use crate::auth::AuthRwLock;
use crate::{repo, LmApiClient};

pub fn build_router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    AuthRwLock: FromRef<S>,
    repo::Repo: FromRef<S>,
    LmApiClient: FromRef<S>,
{
    Router::new()
        .nest("/project", project::build_router())
        .nest("/unit", unit::build_router())
        .nest("/commit", commit::build_router())
        .nest("/lm", lm::build_router())
}

#[derive(Debug)]
pub struct ServiceError {
    status_code: StatusCode,
    message: String,
}

impl From<(StatusCode, &str)> for ServiceError {
    fn from((status_code, message): (StatusCode, &str)) -> Self {
        ServiceError {
            status_code,
            message: String::from(message),
        }
    }
}

impl From<repo::Error> for ServiceError {
    fn from(error: repo::Error) -> Self {
        use repo::Error::*;
        match error {
            NotFound => ServiceError {
                status_code: StatusCode::NOT_FOUND,
                message: String::from("The requested resource could not be found"),
            },
            NotUnique { .. } | ForeignKeyViolation { .. } | ConstraintViolation { .. } => {
                ServiceError {
                    status_code: StatusCode::CONFLICT,
                    message: format!("The requested operation cannot be completeed: {}", error),
                }
            }
            DataError { .. } => ServiceError {
                status_code: StatusCode::BAD_REQUEST,
                message: String::from("Cannot serialize or deserialize data"),
            },
            _ => ServiceError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: String::from("The server has encountered an internal error"),
            },
        }
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        (self.status_code, self.message).into_response()
    }
}
