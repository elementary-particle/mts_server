use axum::extract::{FromRef, Json, Query, State};
use axum::{routing, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ServiceError;
use crate::auth::{AuthRwLock, Claim};
use crate::repo;

pub fn build_router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    AuthRwLock: FromRef<S>,
    repo::Repo: FromRef<S>,
{
    Router::new()
        .route("/", routing::get(get_list).post(add))
        .route("/by-id", routing::get(get_by_id))
        .route("/record", routing::get(get_record_list))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct UnitIdQuery {
    pub unit_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    pub id: Uuid,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

async fn get_list(
    State(repo): State<repo::Repo>,
    Query(query): Query<UnitIdQuery>,
) -> Result<Json<Vec<Commit>>, ServiceError> {
    let commit_list = repo.get_commit_by_unit_id(query.unit_id)?;

    Ok(Json(
        commit_list
            .into_iter()
            .map(|t| Commit {
                id: t.id,
                created_by: t.editor_id,
                created_at: t.created_at.and_utc(),
            })
            .collect::<Vec<_>>(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct IdQuery {
    pub id: Uuid,
}

async fn get_by_id(
    State(repo): State<repo::Repo>,
    Query(query): Query<IdQuery>,
) -> Result<Json<Commit>, ServiceError> {
    let commit = repo.get_commit_by_id(query.id)?;

    Ok(Json(Commit {
        id: commit.id,
        created_by: commit.editor_id,
        created_at: commit.created_at.and_utc(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    pub sq: i32,
    pub content: String,
}

async fn get_record_list(
    State(repo): State<repo::Repo>,
    Query(query): Query<IdQuery>,
) -> Result<Json<Vec<Record>>, ServiceError> {
    let record_list = repo.get_record_by_commit_id(query.id)?;

    Ok(Json(
        record_list
            .into_iter()
            .map(|t| Record {
                sq: t.sq,
                content: t.content,
            })
            .collect::<Vec<_>>(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewCommit {
    pub unit_id: Uuid,
    pub record_list: Vec<Record>,
}

async fn add(
    claim: Claim,
    State(repo): State<repo::Repo>,
    Json(new_commit): Json<NewCommit>,
) -> Result<Json<Uuid>, ServiceError> {
    let user_id = claim.id;

    let commit_id = Uuid::new_v4();
    let commit = repo::Commit {
        id: commit_id,
        unit_id: new_commit.unit_id,
        created_at: Utc::now().naive_utc(),
        editor_id: user_id,
    };
    let record_list = new_commit
        .record_list
        .into_iter()
        .map(|t| repo::Record {
            commit_id: commit_id,
            sq: t.sq,
            content: t.content,
        })
        .collect::<Vec<_>>();

    repo.add_commit(commit, record_list)?;

    Ok(Json(commit_id))
}
