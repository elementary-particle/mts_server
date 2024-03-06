use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRef, Json, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{routing, Router};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Claim, ServiceError};
use crate::repo;

use super::{timestamp_now, AuthRwLock, OptionalClaim, TOKEN_DURATION};

pub fn build_router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    AuthRwLock: FromRef<S>,
    repo::Repo: FromRef<S>,
{
    Router::new()
        .route("/sign-in", routing::post(sign_in))
        .route("/sign-out", routing::get(sign_out))
        .route("/claim", routing::get(get_claim))
        .route("/user", routing::get(get_user).post(add_user))
}

#[derive(Deserialize)]
struct SignInRequest {
    name: String,
    pass: String,
}

async fn sign_in(
    State(AuthRwLock(lock)): State<AuthRwLock>,
    State(repo): State<repo::Repo>,
    Json(request): Json<SignInRequest>,
) -> Result<Response, ServiceError> {
    let user = repo.get_user_by_name(request.name)?;

    let hash =
        PasswordHash::new(&user.hash).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;

    Argon2::default()
        .verify_password(&request.pass.into_bytes(), &hash)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid user name and password combination",
            )
        })?;

    let expires = timestamp_now() + TOKEN_DURATION;

    let claim = Claim {
        id: user.id.clone(),
        expires,
        is_admin: user.is_admin,
    };

    Ok((StatusCode::OK, super::make_token(lock, claim)?).into_response())
}

async fn sign_out() -> Response {
    (StatusCode::OK, super::empty_token()).into_response()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    pub id: Uuid,
    pub is_admin: bool,
}

async fn get_claim(
    OptionalClaim(option): OptionalClaim,
) -> Result<Json<Option<UserInfo>>, ServiceError> {
    Ok(Json(match option {
        Some(claim) => Some(UserInfo {
            id: claim.id,
            is_admin: claim.is_admin,
        }),
        None => None,
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
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
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
struct IdQuery {
    id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: Uuid,
    name: String,
}

async fn get_user(
    State(repo): State<repo::Repo>,
    _claim: Claim,
    Query(query): Query<IdQuery>,
) -> Result<Json<User>, ServiceError> {
    let user = repo.get_user_by_id(query.id)?;
    Ok(Json(User {
        id: user.id,
        name: user.name,
    }))
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
    pass: String,
}

async fn add_user(
    State(repo): State<repo::Repo>,
    claim: Claim,
    Json(new_user): Json<NewUser>,
) -> Result<Json<Uuid>, ServiceError> {
    if !claim.is_admin {
        return Err((
            StatusCode::UNAUTHORIZED,
            "You don't have the appropriate permission for the request",
        ).into());
    }

    create_user(repo.clone(), &new_user.name, &new_user.pass, false).map(|user_id| Json(user_id))
}
