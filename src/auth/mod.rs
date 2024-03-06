pub mod service;

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Mac, SimpleHmac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::OffsetDateTime;
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

struct Secret {
    pub keys: VecDeque<Key>,
}

impl Secret {
    pub fn new() -> Self {
        Secret {
            keys: VecDeque::new(),
        }
    }

    pub fn rotate(&mut self) -> &Key {
        let current_timestamp = timestamp_now();

        while let Some(key) = self.keys.front() {
            if key.expires <= current_timestamp {
                self.keys.pop_front();
            } else {
                break;
            }
        }

        if self.keys.back().is_none() {
            self.keys
                .push_back(Key::generate(current_timestamp + 2 * TOKEN_DURATION));
        }

        self.keys.back().unwrap()
    }
}

#[derive(Clone)]
pub struct AuthRwLock(Arc<RwLock<Secret>>);

impl AuthRwLock {
    pub fn new() -> Self {
        AuthRwLock(Arc::new(RwLock::new(Secret::new())))
    }
}

#[derive(Deserialize, Serialize)]
pub struct Claim {
    pub id: Uuid,
    pub expires: u64,
    pub is_admin: bool,
}

pub struct OptionalClaim(pub Option<Claim>);

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
    fn from_token(s: &str, lock: Arc<RwLock<Secret>>) -> Result<Self, TokenError> {
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

        let mut valid = false;
        {
            let secret = lock.read().unwrap();

            for key in secret.keys.iter().rev() {
                if key.expires > current_timestamp {
                    let mut mac = SimpleHmac::<Sha256>::new_from_slice(&key.bytes)?;

                    mac.update(&claim_raw);
                    if mac.verify_slice(&signature_raw).is_ok() {
                        valid = true;
                        break;
                    }
                }
            }
        }
        if !valid {
            return Err(TokenError);
        }

        let claim: Claim = serde_cbor::from_slice(&claim_raw)?;
        if claim.expires <= current_timestamp {
            return Err(TokenError);
        }

        Ok(claim)
    }

    fn to_token(&self, lock: Arc<RwLock<Secret>>) -> Result<String, TokenError> {
        let mut mac = {
            let mut secret = lock.write().unwrap();
            let key = secret.rotate();

            SimpleHmac::<Sha256>::new_from_slice(&key.bytes).map_err(|_| TokenError)?
        };

        let claim_bytes = serde_cbor::to_vec(self)?;
        mac.update(&claim_bytes);

        let signature_raw = mac.finalize().into_bytes();

        let claim_str = STANDARD.encode(&claim_bytes);
        let sig_str = STANDARD.encode(&signature_raw);

        Ok(format!("{}.{}", claim_str, sig_str))
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Claim
where
    AuthRwLock: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ServiceError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookie_jar = CookieJar::from_request_parts(parts, state).await.unwrap();
        let AuthRwLock(lock) = AuthRwLock::from_ref(state);
        let cookie = cookie_jar
            .get("token")
            .ok_or((StatusCode::UNAUTHORIZED, "No token is set for the request"))?;

        Ok(Claim::from_token(&cookie.value(), lock)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "The provided token is invalid"))?)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OptionalClaim
where
    AuthRwLock: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ServiceError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalClaim(
            Claim::from_request_parts(parts, state)
                .await
                .map_or(None, |claim| Some(claim)),
        ))
    }
}

fn make_token(lock: Arc<RwLock<Secret>>, claim: Claim) -> Result<CookieJar, ServiceError> {
    let token = claim
        .to_token(lock)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;

    let cookie: Cookie = Cookie::build(("token", token))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .expires(
            OffsetDateTime::UNIX_EPOCH + Duration::from_secs(claim.expires.try_into().unwrap()),
        )
        .into();

    Ok(CookieJar::new().add(cookie))
}

fn empty_token() -> CookieJar {
    let mut cookie: Cookie = Cookie::build(("token", ""))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .into();
    cookie.make_removal();

    CookieJar::new().add(cookie)
}
