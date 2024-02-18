pub mod service;

use std::future::{ready, Ready};

use actix_web::body::BoxBody;
use actix_web::{http, web, FromRequest, HttpResponse, ResponseError};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bincode::{deserialize, serialize};
use hmac::{Mac, SimpleHmac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Clone)]
pub struct Secret {
    pub key: [u8; 32],
}

impl Secret {
    pub fn generate() -> Self {
        let mut secret = Secret { key: [0; 32] };
        OsRng.fill_bytes(&mut secret.key);
        secret
    }
}

#[derive(Deserialize, Serialize)]
pub enum Claim {
    User { id: Uuid, is_admin: bool },
    Guest,
}

pub struct TokenError;

impl<T> From<T> for TokenError
where
    T: std::error::Error,
{
    fn from(_: T) -> Self {
        TokenError
    }
}

impl Claim {
    fn from_token(s: &str, secret: &Secret) -> Result<Self, TokenError> {
        let mut parts = s.split(".");

        let claim_raw = parts
            .next()
            .and_then(|t| STANDARD.decode(t).ok())
            .ok_or(TokenError)?;
        let signature_raw = parts
            .next()
            .and_then(|t| STANDARD.decode(t).ok())
            .ok_or(TokenError)?;

        match parts.next() {
            Some(_) => Err(TokenError),
            None => Ok(()),
        }?;

        let mut mac = SimpleHmac::<Sha256>::new_from_slice(&secret.key)?;

        mac.update(&claim_raw);
        mac.verify_slice(&signature_raw)?;

        Ok(deserialize(&claim_raw)?)
    }

    fn to_token(&self, secret: &Secret) -> Result<String, TokenError> {
        let mut mac = SimpleHmac::<Sha256>::new_from_slice(&secret.key).map_err(|_| TokenError)?;

        let claim_raw = serialize(self)?;
        mac.update(&claim_raw);

        let signature_raw = mac.finalize().into_bytes();

        let claim_code = STANDARD.encode(&claim_raw);
        let signature_code = STANDARD.encode(&signature_raw);

        Ok(format!("{}.{}", claim_code, signature_code))
    }

    fn from_request_sync(req: &actix_web::HttpRequest) -> Result<Self, ServiceError> {
        let secret = req
            .app_data::<web::Data<Secret>>()
            .ok_or(ServiceError::ServerError)?;
        match req.cookie("token") {
            Some(cookie) => Self::from_token(cookie.value(), secret).or(Ok(Claim::Guest)),
            None => Ok(Claim::Guest),
        }
    }
}

impl FromRequest for Claim {
    type Error = ServiceError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(Self::from_request_sync(req))
    }
}

#[derive(Debug)]
pub enum ServiceError {
    ServerError,
    WrongPassword,
    Unauthorized,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::ServerError => write!(f, "The server has encountered an internal error"),
            ServiceError::WrongPassword => {
                write!(f, "Wrong name and password combination")
            }
            ServiceError::Unauthorized => write!(f, "Permission denied"),
        }
    }
}

impl ResponseError for ServiceError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            ServiceError::ServerError => http::StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::WrongPassword => http::StatusCode::BAD_REQUEST,
            ServiceError::Unauthorized => http::StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code())
            .content_type("text/plain")
            .body(self.to_string())
    }
}
