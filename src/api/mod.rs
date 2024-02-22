pub mod commit;
pub mod project;
pub mod unit;

use actix_web::body::BoxBody;
use actix_web::cookie::Cookie;
use actix_web::error::BlockingError;
use actix_web::{http::StatusCode, HttpResponse, ResponseError};

use crate::repo;

#[derive(Debug)]
pub enum ServiceError {
    NotFound,
    BadRequest { err_msg: String },
    WrongPassword,
    Unauthorized,
    InvalidToken { cookie: Cookie<'static> },
    ServerError,
}

impl From<repo::Error> for ServiceError {
    fn from(error: repo::Error) -> Self {
        match error {
            repo::Error::NotFound => Self::NotFound,
            _ => Self::BadRequest {
                err_msg: error.to_string(),
            },
        }
    }
}

impl From<BlockingError> for ServiceError {
    fn from(_: BlockingError) -> Self {
        Self::ServerError
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::NotFound => write!(f, "{}", repo::Error::NotFound.to_string()),
            ServiceError::BadRequest { err_msg } => write!(f, "{}", err_msg),
            ServiceError::WrongPassword => {
                write!(f, "Wrong name and password combination")
            }
            ServiceError::Unauthorized => write!(f, "Permission denied"),
            ServiceError::InvalidToken { .. } => write!(f, "Token invaild"),
            ServiceError::ServerError => write!(f, "The server has encountered an internal error"),
        }
    }
}

impl ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        match self {
            ServiceError::NotFound => StatusCode::NOT_FOUND,
            ServiceError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ServiceError::WrongPassword => StatusCode::FORBIDDEN,
            ServiceError::Unauthorized => StatusCode::FORBIDDEN,
            ServiceError::InvalidToken { .. } => StatusCode::BAD_REQUEST,
            ServiceError::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            ServiceError::InvalidToken { cookie } => {
                let mut removal_cookie = cookie.clone();
                removal_cookie.make_removal();
                HttpResponse::build(self.status_code())
                    .content_type("text/plain")
                    .cookie(removal_cookie)
                    .body(self.to_string())
            }
            _ => HttpResponse::build(self.status_code())
                .content_type("text/plain")
                .body(self.to_string()),
        }
    }
}
