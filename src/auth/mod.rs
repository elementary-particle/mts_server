pub mod service;

use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, FromRequest, HttpMessage};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use bincode::{deserialize, serialize};
use futures_util::future::{FutureExt as _, LocalBoxFuture};
use hmac::{Mac, SimpleHmac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::api::ServiceError;

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
            Some(cookie) => Self::from_token(cookie.value(), secret)
                .or(Err(ServiceError::InvalidToken { cookie })),
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

pub struct Authenticate;

impl<S, B> Transform<S, ServiceRequest> for Authenticate
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = AuthenticateMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticateMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthenticateMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticateMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        async move {
            let secret = req.app_data::<Secret>().ok_or(ServiceError::ServerError)?;

            if let Some(cookie) = req.cookie("token") {
                if let Ok(claim) = Claim::from_token(cookie.value(), secret) {
                    req.extensions_mut().insert(claim);
                } else {
                    return Err(ServiceError::InvalidToken { cookie }.into());
                }
            }

            Ok(service.call(req).await?)
        }
        .boxed_local()
    }
}
