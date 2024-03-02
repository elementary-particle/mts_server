use actix_web::web;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ServiceError;
use crate::auth::Claim;
use crate::repo;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct UnitQuery {
    pub unit_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    pub id: Uuid,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[actix_web::get("/commit")]
pub async fn list(
    repo: web::Data<repo::Repo>,
    unit_query: web::Query<UnitQuery>,
) -> Result<web::Json<Vec<Commit>>, ServiceError> {
    let commit_list =
        web::block(move || repo.get_commit_by_unit_id(unit_query.into_inner().unit_id)).await??;

    Ok(web::Json(
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
struct CommitQuery {
    pub id: Uuid,
}

#[actix_web::get("/commit/by-id")]
pub async fn get_by_id(
    repo: web::Data<repo::Repo>,
    commit_query: web::Query<CommitQuery>,
) -> Result<web::Json<Commit>, ServiceError> {
    let commit = web::block(move || repo.get_commit_by_id(commit_query.into_inner().id)).await??;

    Ok(web::Json(Commit {
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

#[actix_web::get("/commit/record")]
pub async fn get_record_list(
    repo: web::Data<repo::Repo>,
    commit_query: web::Query<CommitQuery>,
) -> Result<web::Json<Vec<Record>>, ServiceError> {
    let record_list =
        web::block(move || repo.get_record_by_commit_id(commit_query.into_inner().id)).await??;

    Ok(web::Json(
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

#[actix_web::post("/commit")]
pub async fn add(
    claim: Claim,
    repo: web::Data<repo::Repo>,
    new_commit: web::Json<NewCommit>,
) -> Result<web::Json<Uuid>, ServiceError> {
    let user_id = claim.id;

    let new_unit = new_commit.into_inner();
    let commit_id = Uuid::new_v4();
    let commit = repo::Commit {
        id: commit_id,
        unit_id: new_unit.unit_id,
        created_at: Utc::now().naive_utc(),
        editor_id: user_id,
    };
    let record_list = new_unit
        .record_list
        .into_iter()
        .map(|t| repo::Record {
            commit_id: commit_id,
            sq: t.sq,
            content: t.content,
        })
        .collect::<Vec<_>>();

    web::block(move || repo.add_commit(commit, record_list)).await??;

    Ok(web::Json(commit_id))
}
