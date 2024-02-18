use actix_web::body::BoxBody;
use actix_web::cookie::Cookie;
use actix_web::{web, HttpResponse, Responder};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{Claim, Secret, ServiceError};
use crate::repo;

#[derive(Deserialize)]
struct LoginRequest {
    name: String,
    pass: String,
}

struct LoginResponse {
    pub token: String,
    pub id: Uuid,
}

impl Responder for LoginResponse {
    type Body = BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok()
            .cookie(
                Cookie::build("token", self.token)
                    .path("/")
                    .secure(true)
                    .http_only(true)
                    .finish(),
            )
            .json(self.id)
    }
}

#[actix_web::post("/login")]
pub async fn login(
    secret: web::Data<Secret>,
    repo: web::Data<repo::Repo>,
    request: web::Json<LoginRequest>,
) -> Result<LoginResponse, ServiceError> {
    let request = request.into_inner();

    let user = repo
        .get_user_by_name(request.name)
        .ok_or(ServiceError::WrongPassword)?;

    let hash = PasswordHash::new(&user.hash).map_err(|_| ServiceError::ServerError)?;

    Argon2::default()
        .verify_password(&request.pass.into_bytes(), &hash)
        .map_err(|_| ServiceError::WrongPassword)?;

    let claim = Claim::User {
        id: user.id.clone(),
        is_admin: user.is_admin,
    };

    let token = claim
        .to_token(&secret.into_inner())
        .map_err(|_| ServiceError::ServerError)?;

    Ok(LoginResponse { token, id: user.id })
}

#[actix_web::post("/id")]
pub async fn check_id(claim: Claim) -> Result<web::Json<Uuid>, ServiceError> {
    match claim {
        Claim::User { id, .. } => Ok(web::Json(id)),
        Claim::Guest => Err(ServiceError::Unauthorized),
    }
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
    })
    .ok_or(ServiceError::ServerError)?;

    Ok(user_id)
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

    match claim {
        Claim::User { id: _, is_admin } => match is_admin {
            true => Ok(()),
            false => Err(ServiceError::Unauthorized),
        },
        Claim::Guest => Err(ServiceError::Unauthorized),
    }?;

    create_user(
        repo.get_ref().clone(),
        &new_user.name,
        &new_user.pass,
        false,
    )
    .map(|user_id| web::Json(user_id))
}

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(login).service(check_id).service(add_user);
}
