pub mod service;

use std::collections::VecDeque;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, FromRequest, HttpMessage};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures_util::future::{FutureExt as _, LocalBoxFuture};
use hmac::{Mac, SimpleHmac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::api::ServiceError;

const TOKEN_DURATION: u64 = 2 * 24 * 60 * 60;

fn timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub struct Key {
    pub bytes: [u8; 32],
    pub expires: u64,
}

impl Key {
    pub fn generate(expires: u64) -> Self {
        let mut key = Key {
            bytes: [0; 32],
            expires,
        };
        OsRng.fill_bytes(&mut key.bytes);
        key
    }
}

pub struct Secret {
    pub keys: VecDeque<Key>,
}

impl Secret {
    pub fn new() -> Self {
        Secret {
            keys: VecDeque::new(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Claim {
    pub id: Uuid,
    pub expires: u64,
    pub is_admin: bool,
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
    fn from_token(s: &str, secret: &mut Secret) -> Result<Self, TokenError> {
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

        let current_timestamp = timestamp_now();

        while let Some(key) = secret.keys.front() {
            if key.expires <= current_timestamp {
                secret.keys.pop_front();
            } else {
                break;
            }
        }

        let mut valid = false;

        for key in secret.keys.iter().rev() {
            let mut mac = SimpleHmac::<Sha256>::new_from_slice(&key.bytes)?;

            mac.update(&claim_raw);
            if mac.verify_slice(&signature_raw).is_ok() {
                valid = true;
                break;
            }
        }

        let claim: Claim = serde_cbor::from_slice(&claim_raw)?;

        if claim.expires < current_timestamp {
            return Err(TokenError);
        }

        match valid {
            true => Ok(claim),
            false => Err(TokenError),
        }
    }

    fn to_token(&self, secret: &mut Secret) -> Result<String, TokenError> {
        let mut has_key = false;

        if let Some(key) = secret.keys.back() {
            if key.expires >= self.expires {
                has_key = true;
            }
        }

        if !has_key {
            let current_timestamp = timestamp_now();
            let key = Key::generate(current_timestamp + 2 * TOKEN_DURATION);

            secret.keys.push_back(key);
        }
        let key = secret.keys.back().unwrap();

        let mut mac = SimpleHmac::<Sha256>::new_from_slice(&key.bytes).map_err(|_| TokenError)?;

        let claim_raw = serde_cbor::to_vec(self)?;
        mac.update(&claim_raw);

        let signature_raw = mac.finalize().into_bytes();

        let claim_code = STANDARD.encode(&claim_raw);
        let signature_code = STANDARD.encode(&signature_raw);

        Ok(format!("{}.{}", claim_code, signature_code))
    }

    fn from_request_sync(req: &actix_web::HttpRequest) -> Result<Self, ServiceError> {
        let secret = req
            .app_data::<web::Data<Mutex<Secret>>>()
            .ok_or(ServiceError::ServerError)?;
        match req.cookie("token") {
            Some(cookie) => Self::from_token(cookie.value(), &mut secret.lock().unwrap())
                .or(Err(ServiceError::InvalidToken { cookie })),
            None => Err(ServiceError::Unauthorized),
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
            let secret = req
                .app_data::<Mutex<Secret>>()
                .ok_or(ServiceError::ServerError)?;

            if let Some(cookie) = req.cookie("token") {
                if let Ok(claim) = Claim::from_token(cookie.value(), &mut secret.lock().unwrap()) {
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
