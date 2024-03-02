use std::sync::Mutex;

use actix_web::body::BoxBody;
use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, HttpResponse, Responder};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Claim, Secret, ServiceError};
use crate::repo;

use super::{timestamp_now, TOKEN_DURATION};

#[derive(Deserialize)]
struct SignInRequest {
    name: String,
    pass: String,
}

struct SignInResponse {
    pub token: String,
    pub expires: u64,
}

impl Responder for SignInResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok()
            .cookie(
                Cookie::build("token", self.token)
                    .path("/")
                    .secure(true)
                    .http_only(true)
                    .same_site(SameSite::Strict)
                    .expires(
                        OffsetDateTime::UNIX_EPOCH
                            + Duration::seconds(self.expires.try_into().unwrap()),
                    )
                    .finish(),
            )
            .body("")
    }
}

#[actix_web::post("/sign-in")]
pub async fn sign_in(
    secret: web::Data<Mutex<Secret>>,
    repo: web::Data<repo::Repo>,
    request: web::Json<SignInRequest>,
) -> Result<SignInResponse, ServiceError> {
    let request = request.into_inner();

    let user = repo.get_user_by_name(request.name)?;

    let hash = PasswordHash::new(&user.hash).map_err(|_| ServiceError::ServerError)?;

    Argon2::default()
        .verify_password(&request.pass.into_bytes(), &hash)
        .map_err(|_| ServiceError::WrongPassword)?;

    let expires = timestamp_now() + TOKEN_DURATION;

    let claim = Claim {
        id: user.id.clone(),
        expires: expires,
        is_admin: user.is_admin,
    };

    let token = claim
        .to_token(&mut secret.lock().unwrap())
        .map_err(|_| ServiceError::ServerError)?;

    Ok(SignInResponse { token, expires })
}

struct SignOutResponse {}

impl Responder for SignOutResponse {
    type Body = BoxBody;

    fn respond_to(self, req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        if let Some(_) = req.cookie("token") {
            let mut cookie = Cookie::build("token", "")
                .path("/")
                .same_site(SameSite::Strict)
                .finish();
            cookie.make_removal();
            HttpResponse::Ok()
                .cookie(cookie)
                .content_type("text/plain")
                .body("")
        } else {
            HttpResponse::Ok().content_type("text/plain").body("")
        }
    }
}

#[actix_web::get("/sign-out")]
async fn sign_out() -> SignOutResponse {
    SignOutResponse {}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    pub id: Uuid,
    pub is_admin: bool,
}

#[actix_web::get("/claim")]
pub async fn check_claim(claim: Claim) -> Result<web::Json<UserInfo>, ServiceError> {
    Ok(web::Json(UserInfo {
        id: claim.id,
        is_admin: claim.is_admin,
    }))
}

pub fn create_user(
    repo: repo::Repo,
    name: &str,
    pass: &str,
    is_admin: bool,
) -> Result<Uuid, ServiceError> {
    let user_id = Uuid::new_v4();
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(pass.as_bytes(), &salt)
        .map_err(|_| ServiceError::ServerError)?
        .to_string();

    repo.add_user(repo::User {
        id: user_id,
        name: name.into(),
        hash,
        is_admin: is_admin,
    })?;

    Ok(user_id)
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct UserQuery {
    id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: Uuid,
    name: String,
}

#[actix_web::get("/user")]
async fn get_user(
    repo: web::Data<repo::Repo>,
    _claim: Claim,
    query: web::Query<UserQuery>,
) -> Result<web::Json<User>, ServiceError> {
    let user = repo.get_user_by_id(query.id)?;
    Ok(web::Json(User {
        id: user.id,
        name: user.name,
    }))
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
    pass: String,
}

#[actix_web::post("/user")]
pub async fn add_user(
    repo: web::Data<repo::Repo>,
    claim: Claim,
    new_user: web::Json<NewUser>,
) -> Result<web::Json<Uuid>, ServiceError> {
    let new_user = new_user.into_inner();

    if !claim.is_admin {
        return Err(ServiceError::Unauthorized);
    }

    create_user(
        repo.get_ref().clone(),
        &new_user.name,
        &new_user.pass,
        false,
    )
    .map(|user_id| web::Json(user_id))
}

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(sign_in)
        .service(sign_out)
        .service(check_claim)
        .service(get_user)
        .service(add_user);
}
